
extern crate serde_json;
extern crate rustful;
extern crate mongodb;
extern crate std;
extern crate serde;
extern crate rmp_serde;

use std::result::Result;
use rustful::{
    Server,
    Context,
    Response,
    Handler,
    DefaultRouter,
    SendResponse,
    ContentFactory
};
use rustful::handler::MethodRouter;
use rustful::handler::method_router::Builder;
use rustful::server::Global;
use rustful::header::{
    ContentType,
    AccessControlAllowOrigin,
    AccessControlAllowMethods,
    AccessControlAllowHeaders,
    Host
};
use rustful::StatusCode;
use rustful::context::{};
use mongodb::{
    Client,
    ThreadedClient
};
use mongodb::coll::Collection;
use mongodb::db::ThreadedDatabase;
use mongodb::cursor::Cursor;
use bson::{Bson, Document, encode_document, decode_document, Encoder, Decoder};
use bson::Bson::Null;
use bson;
use bson::spec::{ElementType, BinarySubtype};
use serde_json::value::Value;
use serde_json::*;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use sodiumoxide::crypto::box_;
use sodiumoxide::init;
use base64::{encode, decode};
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::*;
use unicase::UniCase;

use tnvdata::{Database, TnvData};

use person::Person;
use rustcmd::RustCmd;
use rustcmd::JsonCmd;

#[derive(Debug, Clone)]
pub enum Error {
    ParseError,
    BadId,
    MissingHostHeader,
    CouldNotReadBody,
    MethodNotFound,
    InvalidRequest,
    InvalidParams,
    InternalError,
    ServerError,
    Hacking,
    BadSess,
}

pub fn build_router() -> MethodRouter<Api> {
  let mut router = MethodRouter::<Api>::new();
//  let mut router = MethodRouter::default();
  //Global actions
  router.build().many(|mut endpoint| {
      endpoint.on_get(Api(Some(tnv_get)));
      endpoint.on_post(Api(Some(tnv_post)));
      endpoint.on_options(Api(Some(tnv_get)));
  });
  router
}
impl<'a, 'b> SendResponse<'a, 'b> for Error {
    type Error = rustful::Error;

    fn send_response(self, mut response: Response<'a, 'b>) -> Result<(), rustful::Error> {
        let message = match self {
            Error::CouldNotReadBody => "Can not read body",
            Error::ParseError => "Couldn't parse the command",
            Error::BadId => "The 'id' parameter should be a non-negative integer",
            Error::MissingHostHeader => "No 'Host' header was sent",
            Error::MethodNotFound => "Method Not Found",
            Error::InvalidRequest => "Invalid Request",
            Error::InvalidParams => "Invalid Parameters",
            Error::InternalError => "Internal Error",
            Error::ServerError => "Server_error",
            Error::Hacking => "Stop This",
            Error::BadSess => "Invalid Session"
        };
        response.headers_mut().set(ContentType(content_type!(Text / Plain; Charset = Utf8)));
        response.set_status(StatusCode::BadRequest);
        message.send_response(response)

//        response.try_send("")
    }
}

//An API endpoint with an optional action
pub struct Api(Option<fn(&Database, Context) -> Result<Option<String>, Error>>);

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

fn tnv_get(tnvdata: &Database, context: Context) -> Result<Option<String>, Error> {
    Ok(Some(String::from(r#"{"cmd":"delete_todo"}"#)))
}

///
///
pub fn tnv_post(tnvdata: &Database, mut context: Context) -> Result<Option<String>, Error> {
    let rpc: JsonCmd = try!(serde_json::from_reader(&mut context.body).map_err(|_| Error::ParseError));
    let sess = &rpc.sess.clone();
    println!("POST {:?}", rpc );
    let mongo = &tnvdata.mongo;
    let coll = mongo.db("tmp").collection("cmd");
    let doc = doc! { "key" => sess };
    let now = DateTime::timestamp(&Utc::now());

    let update = doc! { "$set" => { "last" => now }};
    let item = coll.find_one_and_update( doc.clone(), update.clone(), None).unwrap();
    let mut cmd: RustCmd = match item {
        Some(doc) => {
          //let tst: Value = Bson::Document(doc).clone().into();
          let mut rec = RustCmd::from(doc);
          rec.id = rpc.id;
          rec.cmd = rpc.cmd;
          rec.params = rpc.params;
          if rec.cmd == "hello" { // found match and also hello, bad
            println!("Found and hello {:?}", rec);
            let doc = doc! { "key" => sess };
            let inc = doc! { "$inc" => { "state" => 1}};
            coll.update_one(doc.clone(), inc.clone(), None);
            if rec.state > 5 {
              return Err(Error::Hacking);
            }
          } else { // found match, normal processing
            println!("Found, normal {:?}", rec);
          };
          rec
        },
        None => {
          let mut rec;
          if rpc.cmd == "hello" {  // log new session
            let (pkey, ref skey) = tnvdata.key;
            rec = new_sess(rpc, skey);
            println!("new {:?}", rec);
          } else {    // bad hacking
            println!("hacking {:?}", rpc);
            return Err(Error::Hacking);
          };
          rec
        }
    };
    let rtn = match cmd.cmd.as_ref() {
        "hello" => {cmd_hello(tnvdata, context, &cmd)},
        "login" => {cmd_login(tnvdata, context, &cmd)},
        "pass" => {cmd_pass(tnvdata, context, &cmd)},
        "create" => {cmd_create(tnvdata, context, &mut cmd)},
        "regdata" => {cmd_regdata(tnvdata, context, &cmd)},
        _ => Err(Error::MethodNotFound)
    };
    match rtn {
      Ok(v) => {
        let id = cmd.id;
        let rslt = json!({"id": id, "result": v});
        println!("SEND {:?} {:?}", cmd.cmd, rslt );
        Ok(Some(serde_json::to_string(&rslt).unwrap()))
      },
      Err(e) => {
        println!("{:?}", e);
        Err(e)
      }
    }
}

fn new_sess(rpc: JsonCmd, skey: &SecretKey) -> RustCmd {
  let cmd = RustCmd::new(rpc, skey);
  cmd
}


fn person_salt(salt: String, mongo: &Client) -> Option<Person> {
  let coll = mongo.db("tnv").collection("person");
  let doc = doc! { "salt" => salt };
  let mut item = coll.find_one( Some(doc.clone()), None).unwrap();
//              let item = cursor.next();
  let person: Option<Person> = match item {
    Some(p) => {
      let person = Person::new(json!({}), json!({}));
      Some(person)
    },
    None => {
      None
    }
  };
  person
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
      json!({ "id": salt, "nonce": nonce })
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

fn cmd_pass(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
    let mut rslt = json!({"salt": "1023456", "nonce":"12345"});
    Ok(rslt)
}

fn cmd_create(tnvdata: &Database, context: Context, mut cmd: &mut RustCmd) -> Result<Value, Error> {
  println!("CREATE {:?}", cmd);
  let params = cmd.params.as_object().unwrap();
  let email = params.get("email").unwrap().as_str().unwrap();

  let mongo = &tnvdata.mongo;
  let coll = mongo.db("tnv").collection("person");

  let doc = doc! { "email" => email };
  let item = coll.find_one(Some(doc.clone()), None).unwrap();

  let rslt: Value = match item {
    Some(p) => {
      json!({ "nonce": "Try another"})
    },
    None => {
      let salt = encode(&box_::gen_nonce());
      let nonce = encode(&box_::gen_nonce());
      let coll = mongo.db("tmp").collection("cmd");
      let doc = doc! { "key" => (&cmd.key) };
      let set = doc! { "$set" => { "nonce" => email, "idx" => (salt.clone()) }};
      coll.update_one(doc.clone(), set.clone(), None);
      json!({ "salt": salt, "nonce": nonce })
    }

  };

  Ok(rslt)
}

fn cmd_regdata(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
    println!("REGDATA {:?}", cmd);
    let mut params = cmd.params.clone();
    let email = &cmd.nonce;
    let salt = &cmd.idx;

    let mongo = &tnvdata.mongo;
    let coll = mongo.db("tnv").collection("person");
    let doc = doc! { "email" => email};
    let item = coll.find_one(Some(doc.clone()), None).unwrap();
    let rslt: Value = match item {
      Some(p) => {  //error
        return Err(Error::InvalidParams)
      },
      None => {   //should not be found
        // fname, lname, phon, age, hpass, wealth, social, fiscal
        let person = Person::from(params);
        println!("PERSON {:?}", person );
        let doc = person.to_bson();
        let rtn = coll.insert_one(doc.clone(), None);
        let sess = encode(&box_::gen_nonce()[ .. ]);
        json!({"sess": "Ok"})
      }

    };
    Ok(rslt)
}

///
///
fn cmd_hello(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
  #[derive(Serialize, Deserialize)]
  struct Hello {
    hello: Nonce,
  };
  let params = cmd.params.as_object().unwrap();
  let hello = params.get("hello").unwrap().as_str().unwrap();

  let rtn = encode(&box_::gen_nonce()[ .. ]);

  let mongo = &tnvdata.mongo;
  let coll = mongo.db("tmp").collection("cmd");

  let rslt: Value = if cmd.state <= 1 {
    let person = String::new();  // Bson!(cmd.person);

    coll.insert_one(doc!{
      "key" => (&cmd.key),
      "addr" => (format!("{}", context.address.ip())),
      "start" => (cmd.start),
      "last" => (cmd.last),
      "nonce" => (&cmd.nonce),
      "idx" => (&cmd.id),
      "prekey" => (encode(&cmd.prekey[ .. ])),
      "role" => (cmd.role),
      "person" => (String::new()),  // Bson!(cmd.person) or email
      "state" => 1 }, None).unwrap();

      json!({
        "hello": cmd.key,
        "nonce": encode(&rtn),
      })
  } else {
    json!({
      "hello": cmd.key,
      "nonce": encode(&rtn),
    })

  };
//      coll.insert_one(doc!{"sess" => sess, "addr" => addr, "ts" => ts, "prekey" => fast, "nonce" => hello, "role" => 0, "salt" => Null }, None).unwrap();
  println!("HELLO {:?} {:?} {:?}", cmd.key, cmd.addr, cmd.state);
  Ok(rslt)
}
