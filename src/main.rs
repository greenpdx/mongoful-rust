#[macro_use]
extern crate rustful;
#[macro_use]
extern crate log;
#[macro_use(bson, doc)]
extern crate bson;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

extern crate env_logger;
extern crate config;
extern crate mongodb;
extern crate unicase;
extern crate chrono;
extern crate sodiumoxide;
extern crate base64;

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
use rustful::filter::{ContextAction, ContextFilter, FilterContext, ResponseAction, ResponseFilter};
use rustful::header::Headers;
use rustful::context::{};
use mongodb::{
    Client,
    ThreadedClient
};
use mongodb::coll::Collection;
use mongodb::db::ThreadedDatabase;
use mongodb::cursor::Cursor;
use bson::{Bson, Document, encode_document, decode_document};
use bson::Bson::Null;
use serde_json::value::Value;
use unicase::UniCase;
use std::any::Any;
use chrono::{DateTime, Utc};
use sodiumoxide::crypto::box_;
use sodiumoxide::init;
use base64::{encode, decode};
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::*;


fn main() {
    env_logger::init().unwrap();
    let args: Vec<String> = env::args().collect();

    init();

    let config = rd_config();

    let mut router = DefaultRouter::<Api>::new();

    println!("Visit http://localhost:8080 to try this example.");

    //Global actions
    router.build().then().many(|mut endpoint| {
        endpoint.on_get(Api(Some(tnv_get)));
        endpoint.on_post(Api(Some(tnv_post)));
        endpoint.on_options(Api(Some(tnv_get)));
    });

    //Enables hyperlink search, which will be used in CORS
    router.find_hyperlinks = true;

    //Our imitation of a database
    let tnvdata: Box<TnvData> = Box::new(TnvData::new(config)).into();

    //let mut gbl: Global = budget;
    let mut gbl: Global = tnvdata.into();

    //println!("{:?}", gbl.get());
    //The ContentFactory wrapper allows simplified handlers that return their
    //responses
    let server_result = Server {
        handlers: router,
        threads: Some(1),
        server: "tnv".to_string(),
        host: 3030.into(),
        content_type: content_type!(Application / Json; Charset = Utf8),
        global: gbl,
        context_filters: vec![
          Box::new(ChkRequest::new()),
        ],
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

struct ChkRequest {
  id: String,
  sess: PublicKey,
  cmd: String,
  params: Value,
  prekey: PrecomputedKey,
  person: String,
}

impl ChkRequest {
  pub fn new() -> ChkRequest {
    ChkRequest {
      id: "".to_string(),
      sess: PublicKey([0; 32]),
      cmd: "".to_string(),
      params: json!({}),
      prekey: PrecomputedKey([0; 32]),
      person: "".to_string(),
    }
  }
}

impl ContextFilter for ChkRequest {
  fn modify(&self, _ctx: FilterContext, context: &mut Context) -> ContextAction {
    //let mut body = String::new();
    //let len = context.body.by_ref().read_to_string(&mut body);
    //println!("{:?} {:?}", len, body);
//    match rpc {
//      Ok(x) => {
//        println!("{:?}", x );
//      },
//      Err(e) => {
//        println!("ERR {:?}", e);
//      }
//    }
//    let mut cmd: RustCmd = rpc.unwrap().clone().into();
//    println!("RCMD {:?}", cmd);
    let tnvdata: Option<&TnvData> = context.global.get();
    match tnvdata {
        Some(t) => {
          let key = &t.key;
          println!("KEY {:?}", key);
        },
        None => {

        }
    }
//    {
//      tnvdata
//    } else {
//      error!("expected a globally accessible Database");
//      None
//    };
//    let mongo = tnvdata.mongo;


    // context.headers, http_version, address, method, uri, hyperlinks, variables, query, fragment, global, body
//    println!("{:?} {:?}", context.headers, context.method);
    ContextAction::next()
  }
}

enum Error {
    ParseError,
    BadId,
    MissingHostHeader,
    CouldNotReadBody,
    MissingFileCache,
    CmdNotFound,
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
            Error::CmdNotFound => "Command Not Found",
        };
        response.headers_mut().set(ContentType(content_type!(Text / Plain; Charset = Utf8)));
        response.set_status(StatusCode::BadRequest);
        message.send_response(response)

//        response.try_send("")
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct JsonCmd {
    cmd: String,
    id: String,
    sess: String,
    params: Value,
}
#[derive(Debug)]
struct RustCmd {
  cmd: String,
  id: String,
  ckey: PublicKey,
  params: Value,
  login: Option<TmpLogin>,
  person: Option<Person>,
}
impl From<JsonCmd> for RustCmd {
  fn from(json: JsonCmd) -> RustCmd {
    let key: &[u8] = &decode(&json.sess).unwrap();
    let rtn = RustCmd {
      cmd: json.cmd,
      id: json.id,
      ckey: PublicKey::from_slice(key).unwrap(),
      params: json.params,
      login: None,
      person: None,
    };
    rtn
  }
}
#[derive(Debug)]
struct Person {
  email: String,    // EMail
  salt: Nonce,
  hpass: Nonce,
  fname: String,
  role: i32,
}

fn cmd_login(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
  let login = cmd.params.as_object().unwrap();
  let email = login.get("email").unwrap().as_str().unwrap();

  let mongo = &tnvdata.mongo;
  let coll = mongo.db("tnv").collection("person");
  let doc = doc! { "email" => email };
  let item = coll.find_one(Some(doc.clone()), None).unwrap();

  let rslt =  match item {
    Some(p) => {
      let salt = "gggg".to_string();
      let nonce = "mmmm".to_string();
      json!({ "salt": salt, "nonce": nonce })
    },
    None => {
      println!("Not Found {:?}", email);
      json!({ "nonce": "Not found"})
    }
  };
//    if (email) {}
    println!("LOGIN {:?} {:?} {:?}", email, rslt.get("nonce").unwrap(), rslt.get("salt"));
    Ok(rslt)
}

fn cmd_pass(tnvdata: &Database, context: Context, cmd: Value) -> Result<Option<String>, Error> {
    let mut rslt = json!({"salt": "1023456", "nonce":"12345"});
    Ok(Some(serde_json::to_string(&rslt).unwrap()))
}

fn cmd_regdata(tnvdata: &Database, context: Context, cmd: Box<Value>) -> Result<Option<String>, Error> {
    let mut rslt = json!({"salt": "1023456", "nonce":"12345"});
    Ok(Some(serde_json::to_string(&rslt).unwrap()))
}

fn cmd_hello(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
  #[derive(Serialize, Deserialize)]
  struct Hello {
    hello: Nonce,
  };
  let nonce = box_::gen_nonce();
  let (pkey, ref skey) = tnvdata.key;
  match cmd.login {
    Some(ref x) => println!("{:?}", x ),
    None => {
      let params = cmd.params.as_object().unwrap();
      let hello = params.get("hello").unwrap().as_str().unwrap();
      let cnonce = decode(hello).unwrap();

      let ckey = &cmd.ckey;
      let prekey = &box_::precompute(ckey, &skey);
      let fast = encode(&prekey[ .. ]);

      let addr = format!("{}", context.address.ip());
      let ts = DateTime::timestamp(&Utc::now());
      let sess = encode(&cmd.ckey[ .. ]);

      let mongo = &tnvdata.mongo;
      let coll = mongo.db("tmp").collection("login");

      println!("HELLO {:?} {} {:?}", pkey, addr, prekey);
      coll.insert_one(doc!{"sess" => sess, "addr" => addr, "ts" => ts, "prekey" => fast, "nonce" => hello, "role" => 0, "salt" => Null }, None).unwrap();
    }
  }
  println!("HELLO {:?}", pkey );
  let rslt = json!({
    "hello": encode(&pkey),
    "nonce": encode(&nonce),
  });
  Ok(rslt)
}

#[derive(Debug)]
struct TmpLogin {
  sess: PublicKey,
  addr: String,
  ts: i64,
  prekey: PrecomputedKey,
  nonce: Nonce,
  salt: Option<Nonce>,
  role: i32,
}

//type Sess = [u8; 32];
//type Sess = &[u8];

impl From<Document> for TmpLogin {
  fn from(doc: Document) -> TmpLogin {
    let nonce: &[u8] = &decode(doc.get_str("nonce").unwrap()).unwrap();
    let sess: &[u8] = &decode(doc.get_str("sess").unwrap()).unwrap();
    let val = TmpLogin {
      sess: PublicKey::from_slice(sess).unwrap(),
      addr: doc.get_str("addr").unwrap().to_string(),
      ts: doc.get_i64("ts").unwrap(),
      prekey: PrecomputedKey::from_slice(&decode(doc.get_str("prekey").unwrap()).unwrap()).unwrap(),
      nonce: Nonce::from_slice(nonce).unwrap(),
      salt: None,
      role: 0,
    };
    println!("FROM {:?} {:?}", doc, val );
    val
  }
}

impl TmpLogin {

}

fn tnv_post(tnvdata: &Database, mut context: Context) -> Result<Option<String>, Error> {
    let rpc: JsonCmd = try!(serde_json::from_reader(&mut context.body).map_err(|_| Error::ParseError));
    let mut cmd: RustCmd = rpc.clone().into();
    let key = &cmd.ckey;
    println!("{:?} {:?} {:?} {:?}", cmd.cmd, cmd.id, key, cmd.params);
    let mongo = &tnvdata.mongo;
    let coll = mongo.db("tmp").collection("login");
    let sess = &rpc.sess;
    let doc = doc! { "sess" => sess };
    let mut cursor = coll.find( Some(doc.clone()), None).unwrap();

    let hello = rpc.cmd == "hello".to_string();
    let item = cursor.next();
    let found: Option<TmpLogin> = match item {
        Some(x) => {
          let doc = x.unwrap();
          //let tst: Value = Bson::Document(doc).clone().into();
          let rec = TmpLogin::from(doc);
          if hello { // found match and also hello, bad
            println!("Found and hello {:?}", rec);
          } else { // found match, normal processing
            if rec.salt != None {
              let salt = encode(&rec.salt.unwrap()[ .. ]);
              let coll = mongo.db("tnv").collection("person");
              let doc = doc! { "salt" => salt };
              let mut item = coll.find_one( Some(doc.clone()), None).unwrap();
//              let item = cursor.next();
              let person: Option<Person> = match item {
                Some(p) => {
                  let person = Person {
                    email: "".to_string(),
                    hpass: box_::gen_nonce(),
                    salt: box_::gen_nonce(),
                    fname: "".to_string(),
                    role: 0,
                  };
                  Some(person)
                },
                None => {
                  None
                }
              };
              cmd.person = person;
            };
            println!("Found, normal {:?}", rec);
          };
          Some(rec)
        },
        None => {
          if hello {  // log new session
            println!("new {:?}", rpc);
          } else {    // bad hacking
            println!("hacking {:?}", rpc);
          };
          None
        }
    };
    println!("{:?}", found );
    cmd.login = found;
    let rtn = match rpc.cmd.as_ref() {
          "hello" => {cmd_hello(tnvdata, context, &cmd)},
          "login" => cmd_login(tnvdata, context, &cmd),
//        "pass" => cmd_pass(tnvdata, context, todo),
//        "regdata" => cmd_regdata(tnvdata, context, todo),
        _ => Err(Error::CmdNotFound)
    };
    match rtn {
      Ok(v) => {
        let id = rpc.id;
        let rslt = json!({"id": id, "result": v});
        Ok(Some(serde_json::to_string(&rslt).unwrap()))
      },
      Err(e) => Err(e)
    }
}

fn rdPerson(item: Document, rpc: JsonCmd) {

}

fn tnv_get(tnvdata: &Database, context: Context) -> Result<Option<String>, Error> {
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
        let tnvdata = if let Some(tnvdata) = context.global.get() {
            tnvdata
        } else {
            error!("expected a globally accessible Database");
            response.set_status(StatusCode::InternalServerError);
            return
        };
        if let Some(action) = self.0 {
            response.send(action(tnvdata, context));
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
    key: (PublicKey, SecretKey),
}

impl TnvData {
    fn new(config: Config) -> Self {
        TnvData {
            mongo: init_mongo(&config),
            budget: get_budget(&config),
            key: box_::gen_keypair(),
        }
    }
}
