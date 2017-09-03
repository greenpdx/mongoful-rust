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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct JsonCmd {
    pub cmd: String,
    pub id: String,
    pub sess: String,
    pub params: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RustCmd {
  pub cmd: String,
  pub id: String,
  pub key: String,
  pub params: Value,
  pub addr: String,
  pub start: i64,
  pub last: i64,
  pub nonce: String,
  pub idx: String,
  pub prekey: PrecomputedKey,
  pub role: i32,
  pub person: Option<Person>,
  pub state: i32,
}

impl RustCmd {
  pub fn new(jc: JsonCmd, skey: &SecretKey) -> RustCmd {
    // assume jc.cmd == "hello"
    let params = jc.params.clone();
    let obj = jc.params.as_object().unwrap();
    let pkey = &PublicKey::from_slice(&decode(&jc.sess).unwrap()).unwrap();
//    PublicKey(decode(&cmd.key).unwrap().as_str()).unwrap();
//    let prekey = &box_::precompute(ckey, &skey);
    let rc = RustCmd {
      cmd: jc.cmd,
      id: jc.id,
      key: jc.sess,
      params: params,
      addr: String::new(),
      start: DateTime::timestamp(&Utc::now()),
      last: DateTime::timestamp(&Utc::now()),
      nonce: String::new(),
      idx: String::new(),
      prekey: box_::precompute(&pkey, skey),
      role: 0,
      person: None,
      state: 0,
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
    let pk: &[u8] = &decode(doc.get_str("prekey").unwrap()).unwrap();
    let rtn = RustCmd {
      cmd:  String::new(),
      id:  String::new(),
      key:  doc.get_str("key").unwrap().to_string(),
      params: json!(null),
      addr: doc.get_str("addr").unwrap().to_string(),
      start: doc.get_i64("start").unwrap(),
      last: DateTime::timestamp(&Utc::now()),
      nonce:  doc.get_str("nonce").unwrap().to_string(),
      idx:  doc.get_str("idx").unwrap().to_string(),
      prekey: PrecomputedKey::from_slice(pk).unwrap(),
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
