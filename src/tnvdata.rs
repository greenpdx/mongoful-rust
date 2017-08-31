extern crate config;
extern crate mongodb;

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

use chrono::{DateTime, Utc};
use sodiumoxide::crypto::box_;
use base64::{encode, decode};
use sodiumoxide::crypto::box_::curve25519xsalsa20poly1305::*;

use tnvconfig;


use config::Config;

pub type Database = TnvData;

pub struct TnvData {
    pub mongo: Client,
    pub budget: Value,
    pub key: (PublicKey, SecretKey),
}

impl TnvData {
    pub fn new(config: Config) -> Self {
        TnvData {
            mongo: init_mongo(&config),
            budget: get_budget(&config),
            key: box_::gen_keypair(),
        }
    }
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
pub fn json_value_from_cursor(cursor: Cursor) -> mongodb::Result<Value> {
    let jsons: mongodb::Result<Vec<Value>> = cursor
        .map(|doc| {
            let json: Value = Bson::Document(doc?).into();
            Ok(json)
        })
        .collect();

    Ok(jsons.map(Value::Array)?)
}
