#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate serde_json;
extern crate tokio_core;

use std::io;
use std::error::Error;
use std::fmt;
use futures::{Future, Stream};
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use serde_json::Value;

#[derive(Debug)]
struct MyError {
    details: String
}

impl MyError {
    fn new(msg: &str)->MyError{
        MyError{details: msg.to_string()}
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{
        write!(f, "{}", self.details)
    }
}

impl Error for MyError {
    fn description(&self) -> &str{
        &self.details
    }
}

fn http() -> Result<Value, Box<Error>> {
    let mut core = Core::new()?;
    let client = Client::new(&core.handle());

    let work = client
        .get("https://api.coinmarketcap.com/v1/global/".parse()?)
        .and_then(|res| {
            println!("Response: {}", res.status());

            res.body().concat2().and_then(move |body| {
                let v = serde_json::from_slice(&body).unwrap();
                Ok(v)
            })
        });
    core.run(work).map_err(|e|{
        From::from(e)
    })
}

fn hi() -> Result<(), Box<Error>> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let work = client
        .get("https://api.coinmarketcap.com/v1/global/".parse()?)
        .and_then(|res| {
            println!("Response: {}", res.status());

            res.body().concat2().and_then(move |body| {
                let v: Value = serde_json::from_slice(&body).unwrap();
                Ok(v)
            })
        });
    let got = core.run(work)?;
    println!("{:?}", &got);
    Ok(())
}

#[get("/")]
fn index() -> &'static str {
    match http() {
        Ok(_) => (),
        Err(e) => println!("{:?}", e),
    }
    "ok"
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
