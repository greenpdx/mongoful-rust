
use rustful::filter::{ContextAction, ContextFilter, FilterContext, ResponseAction, ResponseFilter};
use rustful::{
    Server,
    Context,
    Response,
    Handler,
    DefaultRouter,
    SendResponse,
    ContentFactory
};
use sodiumoxide::crypto::box_;
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::*;
use serde_json::value::Value;

use tnvdata::TnvData;

pub type Database = TnvData;

pub struct ChkRequest {
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
