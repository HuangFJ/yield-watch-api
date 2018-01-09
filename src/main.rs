#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate serde_json;
extern crate tokio_core;

use std::error::Error;
use futures::{Future, Stream};
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use serde_json::Value;

fn request_json(url: &str) -> Result<Value, Box<Error>> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let work = client.get(url.parse()?).and_then(|res| {
        println!("Response: {}", res.status());

        res.body()
            .concat2()
            .and_then(move |body| {
                let v = serde_json::from_slice(&body).unwrap();
                Ok(v)
            })
    });

    core.run(work).map_err(From::from)
}

#[get("/")]
fn index() -> String {
    // .to_string: &str -> String 
    // .as_str: String -> &str
    
    let ret = request_json("https://api.coinmarketcap.com/v1/global/").unwrap();

    format!("{:?}", ret)
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
