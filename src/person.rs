use base64::{encode, decode};
use sodiumoxide::crypto::box_;
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::*;
use chrono::{DateTime, Utc};
use bson::{Bson, Document, encode_document, decode_document, Encoder, Decoder};
use bson::Bson::Null;
use bson;
use bson::spec::{ElementType, BinarySubtype};
use serde_json::value::Value;
use serde_json::*;
use serde_json::map::*;
use serde::{Serialize, Deserialize};


/*
struct BItem {
    agency: u16,
    bureau: u8,
    acct:   u32,
    value:  u32,
}
struct Budget {
    bhash: String,
    budget: Vec<BItem>,
}
impl Budget {
    fn new() {
        Budget {
            bhash: '',
            budget<BItem>: Vec::new(),
        }
    }

}
*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Person {
  email: String,    // EMail
  salt: String,
  pass: String,
  fname: String,
  lname: String,
  age: i8,
  wealth: i8,
  fiscal: i8,
  social: i8,
  role: i32,
  id: String,
  first: i64,
  last: i64,
}

impl Person {
  pub fn new(val: Value, arg: Value) -> Person {
    let map = Map::new();
    let oarg = arg.as_object().unwrap_or(&map);
    let mut oval = val.as_object().unwrap().clone();
    for (k, v) in oarg.iter() {
      oval.insert(k.clone(),v.clone());
    };
    let person = Person {
      email: oval.get("email").unwrap_or(&json!("")).as_str().unwrap().to_string(),
      salt: oval.get("salt").unwrap_or(&json!("")).as_str().unwrap().to_string(),
      pass: oval.get("pass").unwrap_or(&json!("")).as_str().unwrap().to_string(),
      fname: oval.get("fname").unwrap_or(&json!("")).as_str().unwrap().to_string(),
      lname: oval.get("lname").unwrap_or(&json!("")).as_str().unwrap().to_string(),
      id: oval.get("id").unwrap_or(&json!("")).as_str().unwrap().to_string(),
      age: oval.get("age").unwrap_or(&json!(-1)).as_i64().unwrap() as i8,
      wealth: oval.get("wealth").unwrap_or(&json!(-1)).as_i64().unwrap() as i8,
      social: oval.get("social").unwrap_or(&json!(-1)).as_i64().unwrap() as i8,
      fiscal: oval.get("fiscal").unwrap_or(&json!(-1)).as_i64().unwrap() as i8,
      role: oval.get("role").unwrap_or(&json!(0)).as_i64().unwrap() as i32,
      first: oval.get("first").unwrap_or(&json!(0)).as_i64().unwrap(),
      last: oval.get("last").unwrap_or(&json!(0)).as_i64().unwrap(),
    };
    person
  }

  pub fn to_bson(&self) -> Document {
    let doc = doc! {
      "email" => (&self.email),
      "fname" => (&self.fname),
      "lname" => (&self.lname),
      "salt" => (&self.salt),
      "pass" => (&self.pass),
      "age" => (self.age as i32),
      "wealth" => (self.wealth as i32),
      "social" => (self.social as i32),
      "fiscal" => (self.fiscal as i32),
      "id" => (&self.id),
      "first" => (self.first),
      "last" => (self.last)
    };
    doc
  }
}

impl From<Document> for Person {
  fn from(doc: Document) -> Person {
    let person = Person {
      email: doc.get_str("email").unwrap().to_string(),
      fname: doc.get_str("fname").unwrap().to_string(),
      lname: doc.get_str("lname").unwrap().to_string(),
      salt: doc.get_str("salt").unwrap().to_string(),
      pass: doc.get_str("pass").unwrap().to_string(),
      age: doc.get_i32("email").unwrap() as i8,
      wealth: doc.get_i32("email").unwrap() as i8,
      social: doc.get_i32("email").unwrap() as i8,
      fiscal: doc.get_i32("email").unwrap() as i8,
      role: doc.get_i32("role").unwrap(),
      id: doc.get_str("id").unwrap().to_string(),
      first: doc.get_i64("first").unwrap(),
      last: doc.get_i64("last").unwrap(),
    };
    person
  }
}

impl From<Value> for Person {
  fn from(val: Value) -> Person {
    let id = String::new();     //
    let map = val.as_object().unwrap().clone();
    let person = Person {
      email: map.get("email").unwrap().as_str().unwrap().to_string(),    // EMail
      salt: map.get("salt").unwrap().as_str().unwrap().to_string(),
      pass: map.get("pass").unwrap().as_str().unwrap().to_string(),
      fname: map.get("fname").unwrap().as_str().unwrap().to_string(),
      lname: map.get("lname").unwrap().as_str().unwrap().to_string(),
      age: map.get("age").unwrap().as_i64().unwrap() as i8,
      wealth: map.get("wealth").unwrap().as_i64().unwrap() as i8,
      fiscal: map.get("fiscal").unwrap().as_i64().unwrap() as i8,
      social: map.get("social").unwrap().as_i64().unwrap() as i8,
      role: 0,
      id: id,
      first: DateTime::timestamp(&Utc::now()),
      last: DateTime::timestamp(&Utc::now()),
    };
    person
  }
}



/*
struct Person {
    fname: String,
    lname: String,
    email: String,
    hpass: String,
    salt: String,
    hphone: String,
    age: u8,
    wealth: u8,
    social: u8,
    fiscal: u8,
    saveBudg: Budget,
    submitBudg: Budget,
    temptName: String,
    recipt: String,
}

impl Person {
    fn new() -> Self {
        Person {
            fname: "",
            lname: "",
            email: "",
            hpass: "",
            salt: "",
            hphone: None,
            age: 0,
            wealth: 0,
            social: 0,
            fiscal: 0,
            saveBudg: None,
            submitBudg: None,
            temptName: "",
            recipt: "",
        }
    }
    initPerson(data: Value) {

    }
}
*/
