
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct JsonCmd {
    cmd: String,
    id: String,
    sess: String,
    params: Value,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RustCmd {
  cmd: String,
  id: String,
  key: String,
  params: Value,
  start: i64,
  last: i64,
  nonce: String,
  prekey: PrecomputedKey,
  role: i32,
  person: Option<Person>,
  state: i32,
}

impl RustCmd {
  fn new(jc: JsonCmd, pkey: PublicKey) -> RustCmd {

    let rc = RustCmd {
      cmd: jc.cmd,
      id: jc.id,
      key: jc.sess,
      params: jc.params,
      start: DateTime::timestamp(&Utc::now()),
      last: DateTime::timestamp(&Utc::now()),
      nonce: String::new(),
      prekey: box_::precompute(&pkey, &skey),
      role: 0,
      person: None,
      state: 1,
    };
    rc
  }
}
/*impl From<JsonCmd> for RustCmd {
  fn from(json: JsonCmd) -> RustCmd {
    let key: &[u8] = &decode(&json.sess).unwrap();
    let rtn = RustCmd {
      cmd: json.cmd,
      id: json.id,
      key: json.sess,
      params: json.params,
      start: DateTime::timestamp(&Utc::now()),
      last: DateTime::timestamp(&Utc::now()),
      nonce: String::new(),
      prekey: PrecomputedKey::from_slice(&[0,32]).unwrap(),
      role: -1,
      person: None,
      state: 0,
    };
    rtn
  }
}*/
impl From<Document> for RustCmd {
  fn from(doc: Document) -> RustCmd {
    let rtn = RustCmd {
      cmd:  String::new(),
      id:  String::new(),
      key:  doc.get_str("key").unwrap().to_string(),
      params: json!(null),
      start: doc.get_i64("start").unwrap(),
      last: DateTime::timestamp(&Utc::now()),
      nonce:  doc.get_str("nonce").unwrap().to_string(),
      prekey: PrecomputedKey::from_slice(doc.get_binary_generic("prekey").unwrap()).unwrap(),
      role: doc.get_i32("role").unwrap(),
      person: None,
      state: doc.get_i32("state").unwrap(),
    };
    rtn
  }
}
/* impl From<RustCmd> for Bson {
  fn from(cmd: RustCmd) -> Bson {

  }
} */

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Person {
  email: String,    // EMail
  salt: Nonce,
  hpass: Nonce,
  fname: String,
  role: i32,
}

pub enum Error {
    ParseError,
    BadId,
    MissingHostHeader,
    CouldNotReadBody,
    MissingFileCache,
    CmdNotFound,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct TmpLogin {
  sess: String,  // client session base64 encoded sodiumoxide PublicKey
  addr: String,  // remote address for hacking issues
  start: i64,   // session start time
  last: i64,   // last access
  prekey: PrecomputedKey,
  nonce: String,  // session nonce base64 encoded
  salt: Option<String>,  // The ID of the user, only decode during verification
  role: i32,
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

    let mongo = &tnvdata.mongo;
    let coll = mongo.db("tmp").collection("cmd");
    let doc = doc! { "key" => sess };
    let now = DateTime::timestamp(&Utc::now());

    let update = doc! { "last" => now };
    let item = coll.find_one_and_update( doc.clone(), update.clone(), None).unwrap();
    let cmd: RustCmd = match item {
        Some(doc) => {
          //let tst: Value = Bson::Document(doc).clone().into();
          let mut rec = RustCmd::from(doc);
          rec.id = rpc.id;
          rec.cmd = rpc.cmd;
          rec.params = rpc.params;
          if rec.cmd == "hello" { // found match and also hello, bad
            println!("Found and hello {:?}", rec);
          } else { // found match, normal processing
            println!("Found, normal {:?}", rec);
          };
          rec
        },
        None => {
          let mut rec;
          if rpc.cmd == "hello" {  // log new session
            rec = new_sess(rpc, coll, );
            println!("new {:?}", rec);
          } else {    // bad hacking
            println!("hacking {:?}", rpc);
            return Err(Error::CmdNotFound);
          };
          rec
        }
    };
    println!("{:?}", cmd );
    let rtn = match cmd.cmd.as_ref() {
        "hello" => {cmd_hello(tnvdata, context, &cmd)},
        "login" => {cmd_login(tnvdata, context, &cmd)},
        "pass" => {cmd_pass(tnvdata, context, &cmd)},
        "regdata" => {cmd_regdata(tnvdata, context, &cmd)},
        _ => Err(Error::CmdNotFound)
    };
    match rtn {
      Ok(v) => {
        let id = cmd.id;
        let rslt = json!({"id": id, "result": v});
        Ok(Some(serde_json::to_string(&rslt).unwrap()))
      },
      Err(e) => Err(e)
    }
}

fn new_sess(rpc: JsonCmd, coll: Collection) -> RustCmd {
  let cmd = RustCmd::new(rpc);
  cmd
}


fn load_person(salt: String, mongo: &Client) -> Option<Person> {
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

fn cmd_pass(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
    let mut rslt = json!({"salt": "1023456", "nonce":"12345"});
    Ok(rslt)
}

fn cmd_regdata(tnvdata: &Database, context: Context, mut cmd: &RustCmd) -> Result<Value, Error> {
    let mut rslt = json!({"salt": "1023456", "nonce":"12345"});
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

  let (pkey, ref skey) = tnvdata.key;
  if cmd.role <= 0 {    //

    let ckey = &PublicKey::from_slice(&decode(&cmd.key).unwrap()).unwrap();
//    PublicKey(decode(&cmd.key).unwrap().as_str()).unwrap();
    let prekey = &box_::precompute(ckey, &skey);
    let fast = encode(&prekey[ .. ]);

    let addr = format!("{}", context.address.ip());
    let ts = DateTime::timestamp(&Utc::now());
    let sess = &cmd.key;

    let mongo = &tnvdata.mongo;
    let coll = mongo.db("tmp").collection("login");

    println!("HELLO {:?} {} {:?}", pkey, addr, prekey);

    coll.insert_one(doc!{"sess" => sess, "addr" => addr, "ts" => ts, "prekey" => fast, "nonce" => hello, "role" => 0, "salt" => Null }, None).unwrap();

//      coll.insert_one(doc!{"sess" => sess, "addr" => addr, "ts" => ts, "prekey" => fast, "nonce" => hello, "role" => 0, "salt" => Null }, None).unwrap();
  }
  let nonce = box_::gen_nonce();
  println!("HELLO {:?}", pkey );
  let rslt = json!({
    "hello": encode(&pkey),
    "nonce": encode(&nonce),
  });
  Ok(rslt)
}
