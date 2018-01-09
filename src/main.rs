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
use futures::{Future, Stream};
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use tokio_core::reactor::Timeout;
use std::time::Duration;
use serde_json::Value;
use futures::future::Either;

fn request_json(url: &str, timeout: u64) -> Result<Value, Box<Error>> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let timeout = Timeout::new(Duration::from_secs(timeout), &handle)?;
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let get = client.get(url.parse()?).and_then(|res| {
        println!("Response: {}", res.status());

        res.body().concat2().and_then(move |body| {
            let v = serde_json::from_slice(&body).unwrap();
            Ok(v)
        })
    });

    let work = get.select2(timeout)
    .map_err(|res| match res {
        Either::A((get_error, _timeout)) => get_error,
        Either::B((timeout_error, _get)) => From::from(timeout_error),
    })
    .and_then(|res| match res {
        Either::A((got, _timeout)) => Ok(got),
        Either::B((_timeout_error, _get)) => Err(hyper::Error::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "Client timed out while connecting",
        ))),
    });

    core.run(work).map_err(From::from)
}

#[get("/")]
fn index() -> String {
    // .to_string: &str -> String
    // .as_str: String -> &str

    let ret = request_json("https://api.coinmarketcap.com/v1/global/", 5).unwrap();

    format!("{:?}", ret)
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}
