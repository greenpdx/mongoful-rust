#[macro_use]
extern crate rustful;
#[macro_use]
extern crate log;
#[use_macro(bson, doc)]
extern crate bson;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate env_logger;
extern crate config;
extern crate mongodb;
extern crate unicase;

use std::io::{self, Read};
use std::fs::File;
use std::path::Path;
use std::borrow::Cow;
use std::error::Error as ErrorTrait;

use rustful::StatusCode::{InternalServerError, BadRequest};

use std::env;
use config::Config;
use rustful::{
    Server,
    Context,
    Response,
    Handler,
    DefaultRouter,
    SendResponse,
    ContentFactory
};
use rustful::server::Global;
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
use serde_json::value::Value;
use unicase::UniCase;
use std::any::Any;

fn main() {
    env_logger::init().unwrap();
    let args: Vec<String> = env::args().collect();

    let config = rd_config();

    let mut router = DefaultRouter::<Api>::new();

    println!("Visit http://localhost:8080 to try this example.");

    //Global actions
    router.build().then().many(|mut endpoint| {
        endpoint.on_get(Api(Some(base_get)));
        endpoint.on_post(Api(Some(base_post)));
    });

    //Note actions
    router.build().path(":tnv").then().many(|mut endpoint| {
        endpoint.on_get(Api(Some(tnv_get)));
        endpoint.on_post(Api(Some(tnv_post)));
        endpoint.on_options(Api(Some(tnv_get)));
    });

    //Enables hyperlink search, which will be used in CORS
    router.find_hyperlinks = true;

    //Our imitation of a database
    let database: Box<TnvData> = Box::new(TnvData::new(config)).into();

    //let mut gbl: Global = budget;
    let mut gbl: Global = database.into();
    //println!("{:?}", gbl.get());
    //The ContentFactory wrapper allows simplified handlers that return their
    //responses
    let server_result = Server {
        handlers: router,
        threads: Some(1),
        server: "tnv".to_string(),
        host: 8080.into(),
        content_type: content_type!(Application / Json; Charset = Utf8),
        global: gbl,
        ..Server::default()
    }.run();

    //Check if the server started successfully
    match server_result {
        Ok(_server) => {},
        Err(e) => error!("could not start server: {}", e.description())
    }
}

fn rd_config () -> Config {
    let mut config = Config::new();
    config
        .merge(config::File::with_name("config")).unwrap()
        .merge(config::Environment::with_prefix("APP")).unwrap();
//    let addr = SocketAddr::new(
//        IpAddr::from_str(&config.get_str("bind_addr").unwrap()).unwrap(),
//        config.get_int("bind_port").unwrap() as u16);
    config
}

enum Error {
    ParseError,
    BadId,
    MissingHostHeader,
    CouldNotReadBody,
    MissingFileCache
}

impl<'a, 'b> SendResponse<'a, 'b> for Error {
    type Error = rustful::Error;

    fn send_response(self, mut response: Response<'a, 'b>) -> Result<(), rustful::Error> {
        let message = match self {
            Error::CouldNotReadBody => "Can not read body",
            Error::MissingFileCache => "no files",
//                error!("the global data should be of the type `Files`, but it's not");
//                response.set_status(InternalServerError);
//            },
            Error::ParseError => "Couldn't parse the todo",
            Error::BadId => "The 'id' parameter should be a non-negative integer",
            Error::MissingHostHeader => "No 'Host' header was sent",
        };
        response.headers_mut().set(ContentType(content_type!(Text / Plain; Charset = Utf8)));
        response.set_status(StatusCode::BadRequest);
        message.send_response(response)

//        response.try_send("")
    }
}

#[derive(Serialize, Deserialize)]
struct Person {
    cmd: String,
    host: String,
}
//List all the to-dos in the database
fn base_get(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let host: &Host = try!(context.headers.get().ok_or(Error::MissingHostHeader));
    let bob = host.to_string();

    let obj = Person {
        cmd: "list_all".to_owned(),
        host: bob, //+ host.port.unwrap_or(0).to_string()
    };
//    let todo = try!(
//        serde_json::from_reader(context.body).map_err(|_| Error::ParseError)
//    );

//    let todos: Vec<_> = database.read().unwrap().iter()
//      .map(|(&id, todo)| NetworkTodo::from_todo(todo, host, id))
//      .collect();

//    Ok(Some(serde_json::to_string(&todos).unwrap())) */
    let tmp = serde_json::to_string(&obj).unwrap();
    Ok(Some(tmp))
}

fn cmd_login(database: &Database, context: Context, cmd: Value) {
    let mongo = database.mongo;
    let email = cmd["email"];
//    if (email) {}
    println!("LOGIN {:?}", email.as_str().unwrap());
    let coll = mongo.db("tnv").collection("person");
    let doc = doc! { "email" => email };
    let cursor = coll.find(doc, None);
}

fn tnv_post(database: &Database, context: Context) -> Result<Option<String>, Error> {
    let todo: Value = try!(serde_json::from_reader(context.body).map_err(|_| Error::ParseError));
    println!("{:?}", todo);
    let obj = todo.as_object().unwrap();
    let cmd = todo["cmd"].as_str().unwrap();
    println!("{:?} {:?}", cmd, obj);
    match cmd {
        "login" => {cmd_login(database, context, todo)},
        "pass" => {println!("PASS {:?}", todo["auth"].as_str().unwrap())},
        "regdata" => {println!("REG {:?}", todo)}
        _ => {},
    }
    let mut rslt = json!({"salt": "1023456", "nonce":"12345"});
    Ok(Some(serde_json::to_string(&rslt).unwrap()))
}
fn base_post(database: &Database, context: Context) -> Result<Option<String>, Error> {
    Ok(Some(String::from(r#"{"cmd":"edit_todo"}"#)))
}
fn tnv_get(database: &Database, context: Context) -> Result<Option<String>, Error> {
    Ok(Some(String::from(r#"{"cmd":"delete_todo"}"#)))
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

fn init_mongo(config: &Config) -> Client {
    mongodb::Client::connect(&config.get_str("mongo_addr").unwrap(), config.get_int("mongo_port").unwrap() as u16)
      .expect("Failed connect")
}

fn get_budget(config: &Config) -> Value {
    let mongo = mongodb::Client::connect(&config.get_str("mongo_addr").unwrap(), config.get_int("mongo_port").unwrap() as u16)
      .expect("Failed connect");
    let coll = mongo.db("budget").collection("full");

    let cursor = coll.find(None, None)
          .ok().expect("find failed");

    json_value_from_cursor(cursor).expect("Unable to receive all documents from cursor")
}

type Database = TnvData;

struct TnvData {
    mongo: Client,
    budget: Value,
}

impl TnvData {
    fn new(config: Config) -> Self {
        TnvData {
            mongo: init_mongo(&config),
            budget: get_budget(&config),
        }
    }
}
