#[macro_use]
extern crate rustful;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate unicase;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate mongodb;
extern crate bson;
extern crate config;

// use std::sync::RwLock;
// use std::collections::btree_map::{BTreeMap, Iter};

use unicase::UniCase;

use rustful::{
    Server,
    Context,
    Response,
    Handler,
    DefaultRouter,
    SendResponse
};
use rustful::header::{
    ContentType,
    AccessControlAllowOrigin,
    AccessControlAllowMethods,
    AccessControlAllowHeaders,
    Host
};
use rustful::StatusCode;
use mongodb::{
    Client,
    ThreadedClient
};
use mongodb::coll::Collection;
use mongodb::db::ThreadedDatabase;
use mongodb::cursor::Cursor;
use bson::{Bson, Document, encode_document, decode_document};
use serde_json::Value;
use config::Config;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use std::marker::PhantomData;
use std::io;
use std::env;
use std::any::Any;

fn main() {
    env_logger::init().unwrap();
    let args: Vec<String> = env::args().collect();

    let config = rd_config();

    let mut router = DefaultRouter::<Api>::new();

    //Global actions
    router.build().then().many(|mut endpoint| {
        endpoint.on_get(Api(Some(list_all)));
        endpoint.on_post(Api(Some(store)));
        endpoint.on_delete(Api(Some(clear)));
        endpoint.on_options(Api(None));
    });

    //Note actions
    router.build().path(":id").then().many(|mut endpoint| {
        endpoint.on_get(Api(Some(get_todo)));
        endpoint.on_patch(Api(Some(edit_todo)));
        endpoint.on_delete(Api(Some(delete_todo)));
        endpoint.on_options(Api(None));
    });

    //Enables hyperlink search, which will be used in CORS
    router.find_hyperlinks = true;

    //Our imitation of a database
    let database = Dummy::new();

    let server_result = Server {
        handlers: router,
        host: 8080.into(),
        content_type: content_type!(Application / Json; Charset = Utf8),
        global: Box::new(database).into(),
        ..Server::default()
    }.run();

    match server_result {
      Ok(server) => {
        println!(
          "This example is a showcase implementation of the Todo-Backend project (http://todobackend.com/), \
          visit http://localhost:{0}/ to try it or run reference test suite by pointing \
          your browser to http://todobackend.com/specs/index.html?http://localhost:{0}",
          server.socket.port()
        );
      },
      Err(e) => error!("could not run the server: {}", e)
    };
}

fn rd_config () {
    let mut config = Config::new();
    config
        .merge(config::File::with_name("config")).unwrap()
        .merge(config::Environment::with_prefix("APP")).unwrap();
    let addr = SocketAddr::new(
        IpAddr::from_str(&config.get_str("bind_addr").unwrap()).unwrap(),
        config.get_int("bind_port").unwrap() as u16);
}

//Errors that may occur while parsing the request
enum Error {
    ParseError,
    BadId,
    MissingHostHeader,
}

impl<'a, 'b> SendResponse<'a, 'b> for Error {
    type Error = rustful::Error;

    fn send_response(self, mut response: Response<'a, 'b>) -> Result<(), rustful::Error> {
        let message = match self {
            Error::ParseError => "Couldn't parse the todo",
            Error::BadId => "The 'id' parameter should be a non-negative integer",
            Error::MissingHostHeader => "No 'Host' header was sent",
        };

        response.headers_mut().set(ContentType(content_type!(Text / Plain; Charset = Utf8)));
        response.set_status(StatusCode::BadRequest);
        message.send_response(response)
    }
}



//List all the to-dos in the database
fn list_all(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let host = try!(context.headers.get().ok_or(Error::MissingHostHeader));

    let todos: Vec<_> = database.read().unwrap().iter()
      .map(|(&id, todo)| NetworkTodo::from_todo(todo, host, id))
      .collect();

    Ok(Some(serde_json::to_string(&todos).unwrap()))
}

//Store a new to-do with data from the request body
fn store(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let todo: NetworkTodo = try!(
        serde_json::from_reader(context.body).map_err(|_| Error::ParseError)
    );

    let host = try!(context.headers.get().ok_or(Error::MissingHostHeader));

    let mut database = database.write().unwrap();
    database.insert(todo.into());

//    let todo = database.last().map(|(id, todo)| {
//        NetworkTodo::from_todo(todo, host, id)
//    });

    Ok(Some(serde_json::to_string(&todo).unwrap()))
}

//Clear the database
fn clear(database: &Database, _context: Context) -> Result<Option<String>, Error> {
    database.write().unwrap().clear();
    Ok(Some("".into()))
}

//Send one particular to-do, selected by its id
fn get_todo(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let host = try!(context.headers.get().ok_or(Error::MissingHostHeader));
    let id = try!(context.variables.parse("id").map_err(|_| Error::BadId));

//    let todo = database.read().unwrap().get(id).map(|todo| {
//        NetworkTodo::from_todo(&todo, host, id)
//    });

    Ok(todo.map(|todo| serde_json::to_string(&todo).unwrap()))
}

//Update a to-do, selected by its id with data from the request body
fn edit_todo(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let edits: NetworkTodo = try!(
        serde_json::from_reader(context.body).map_err(|_| Error::ParseError)
    );
    let host = try!(context.headers.get().ok_or(Error::MissingHostHeader));
    let id = try!(context.variables.parse("id").map_err(|_| Error::BadId));

    let mut database =  database.write().unwrap();
    let mut todo = database.get_mut(id);
    todo.as_mut().map(|mut todo| todo.update(edits));

//    let todo = todo.map(|todo| {
//        NetworkTodo::from_todo(&todo, host, id)
//    });

    Ok(Some(serde_json::to_string(&todo).unwrap()))
}

//Delete a to-do, selected by its id
fn delete_todo(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let id = try!(context.variables.parse("id").map_err(|_| Error::BadId));
    database.write().unwrap().delete(id);
    Ok(Some("".into()))
}




//An API endpoint with an optional action
struct Api(Option<fn(&Database, Context) -> Result<Option<String>, Error>>);

impl Handler for Api {
    fn handle(&self, context: Context, mut response: Response) {
        //Collect the accepted methods from the provided hyperlinks
        let mut methods: Vec<_> = context.hyperlinks.iter().filter_map(|l| l.method.clone()).collect();
        methods.push(context.method.clone());

        //Setup cross origin resource sharing
        response.headers_mut().set(AccessControlAllowOrigin::Any);
        response.headers_mut().set(AccessControlAllowMethods(methods));
        response.headers_mut().set(AccessControlAllowHeaders(vec![UniCase("content-type".into())]));

        //Get the database from the global storage
        let database = if let Some(database) = context.global.get() {
            database
        } else {
            error!("expected a globally accessible Database");
            response.set_status(StatusCode::InternalServerError);
            return
        };

        if let Some(action) = self.0 {
            response.send(action(database, context));
        }
    }
}

fn json_value_from_cursor(cursor: Cursor) -> mongodb::Result<Value> {
    let jsons: mongodb::Result<Vec<Value>> = cursor
        .map(|doc| {
            let json: Value = Bson::Document(doc?).into();
            Ok(json)
        })
        .collect();

    Ok(jsons.map(Value::Array)?)
}

fn get_budget(config: Config) {
    let mongo = mongodb::Client::connect(&config.get_str("mongo_addr").unwrap(), config.get_int("mongo_port").unwrap() as u16)
      .expect("Failed connect");
    let coll = mongo.db("budget").collection("full");

    let cursor = coll.find(None, None)
          .ok().expect("find failed");

    let json = json_value_from_cursor(cursor).expect("Unable to receive all documents from cursor");

}

type Database = Dummy;

struct TNVData {
    d: i32,
}

impl TNVData {
    fn new() -> Self {
        TNVData {
            d: 0,
        }
    }
    fn insert(&mut self, item: i32) {

    }
    fn delete (&mut self, id: usize) {

    }
    fn get_mut(&self, id: i32) {

    }
    fn clear(&self) {

    }
    fn get(&self, id: i32) {

    }
    fn iter(&self) -> Iter<> {

    }
}

pub struct Dummy {
    tnv: TNVData,
}

impl Dummy {
    pub fn new() -> Self { // Dummy<TNVData> {
        Dummy {
            tnv: TNVData::new(),
        }

    }
}
impl Dummy {
    pub fn read(&self) -> Result<TNVData, io::Error>{

        Ok(self.tnv)
    }
    pub fn write(&self) -> Result<TNVData, io::Error> {
        Ok(self.tnv)
    }
    pub fn insert(&self, itm: i32) {

    }
}

/*
//A read-write-locked Table will do as our database
type Database = RwLock<Table>;

//A simple imitation of a database table
struct Table {
    next_id: usize,
    items: BTreeMap<usize, Todo>
}

impl Table {
    fn new() -> Table {
        Table {
            next_id: 0,
            items: BTreeMap::new()
        }
    }

    fn insert(&mut self, item: Todo) {
        self.items.insert(self.next_id, item);
        self.next_id += 1;
    }

    fn delete(&mut self, id: usize) {
        self.items.remove(&id);
    }

    fn clear(&mut self) {
        self.items.clear();
    }

    fn last(&self) -> Option<(usize, &Todo)> {
        self.items.keys().next_back().cloned().and_then(|id| {
            self.items.get(&id).map(|item| (id, item))
        })
    }

    fn get(&self, id: usize) -> Option<&Todo> {
        self.items.get(&id)
    }

    fn get_mut(&mut self, id: usize) -> Option<&mut Todo> {
        self.items.get_mut(&id)
    }

    fn iter(&self) -> Iter<usize, Todo> {
        (&self.items).iter()
    }
}
*/

//A structure for what will be sent and received over the network
/*#[derive(Serialize, Deserialize)]
struct NetworkTodo {
    title: Option<String>,
    completed: Option<bool>,
    order: Option<u32>,
    url: Option<String>
}

impl NetworkTodo {
    fn from_todo(todo: &Todo, host: &Host, id: usize) -> NetworkTodo {
        let url = if let Some(port) = host.port {
            format!("http://{}:{}/{}", host.hostname, port, id)
        } else {
            format!("http://{}/{}", host.hostname, id)
        };

        NetworkTodo {
            title: Some(todo.title.clone()),
            completed: Some(todo.completed),
            order: Some(todo.order),
            url: Some(url)
        }
    }
}
*/

//The stored to-do data
struct Todo {
    title: String,
    completed: bool,
    order: u32
}

impl Todo {
    fn update(&mut self, changes: NetworkTodo) {
        if let Some(title) = changes.title {
            self.title = title;
        }

        if let Some(completed) = changes.completed {
            self.completed = completed;
        }

        if let Some(order) = changes.order {
            self.order = order
        }
    }
}

impl From<NetworkTodo> for Todo {
    fn from(todo: NetworkTodo) -> Todo {
        Todo {
            title: todo.title.unwrap_or(String::new()),
            completed: todo.completed.unwrap_or(false),
            order: todo.order.unwrap_or(0)
        }
    }
}
