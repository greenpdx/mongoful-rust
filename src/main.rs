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
#[macro_use]
extern crate serde;
extern crate env_logger;
extern crate config;
extern crate mongodb;
extern crate unicase;
extern crate chrono;
extern crate sodiumoxide;
extern crate base64;

use std::error::Error as ErrorTrait;

use std::env;
use config::Config;
use rustful::{
    Server,
};
use rustful::server::Global;
use sodiumoxide::init;

mod api;
mod filter;
mod tnvdata;
mod tnvconfig;

use tnvdata::{TnvData};
use api::{build_router};

fn main() {
    env_logger::init().unwrap();
    let args: Vec<String> = env::args().collect();

    init();

    let config = tnvconfig::rd_config(args);


    println!("Visit http://localhost:8080 to try this example.");

    let router = build_router();

    //Our imitation of a database
    let tnvdata: Box<TnvData> = Box::new(TnvData::new(config)).into();

    //let mut gbl: Global = budget;
    let gbl: Global = tnvdata.into();

    //The ContentFactory wrapper allows simplified handlers that return their
    let server_result = Server {
        handlers: router,
        threads: Some(1),
        server: "tnv".to_string(),
        host: 3030.into(),
        content_type: content_type!(Application / Json; Charset = Utf8),
        global: gbl,
        context_filters: vec![
          Box::new(filter::ChkRequest::new()),
        ],
        ..Server::default()
    }.run();

    //Check if the server started successfully
    match server_result {
        Ok(_server) => {},
        Err(e) => error!("could not start server: {}", e.description())
    }
}
