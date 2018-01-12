extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate serde_json;
extern crate tokio_core;
extern crate toml;

use std::io::{self, Read};
use std::fs::File;
use std::error::Error;
use std::time::Duration;
use self::futures::{Future, Stream};
use self::futures::future::Either;
use self::hyper::Client;
use self::hyper_tls::HttpsConnector;
use self::tokio_core::reactor::{Core, Timeout};
use self::serde_json::Value as Json;
use self::toml::Value as Toml;

pub fn request_json(url: &str, timeout: u64) -> Result<Json, Box<Error>> {
    let mut core = Core::new()?;
    let handle = core.handle();
    let timeout = Timeout::new(Duration::from_secs(timeout), &handle)?;
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &handle)?)
        .build(&handle);

    let get = client.get(url.parse()?).and_then(|res| {
        println!("Response: {}", res.status());

        res.body().concat2().and_then(move |body| {
            serde_json::from_slice(&body).map_err(|_| {
                hyper::Error::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Error converting to json",
                ))
            })
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

pub fn toml2json(toml: Toml) -> Json {
    match toml {
        Toml::String(s) => Json::String(s),
        Toml::Integer(i) => Json::Number(i.into()),
        Toml::Float(f) => {
            let n = serde_json::Number::from_f64(f).expect("Float infinite and nan not allowed");
            Json::Number(n)
        }
        Toml::Boolean(b) => Json::Bool(b),
        Toml::Array(arr) => Json::Array(arr.into_iter().map(toml2json).collect()),
        Toml::Table(table) => {
            Json::Object(table.into_iter().map(|(k, v)| (k, toml2json(v))).collect())
        }
        Toml::Datetime(dt) => Json::String(dt.to_string()),
    }
}

pub fn json_from_tomlfile(filename: &str) -> Json {
    let mut input = String::new();
    File::open(filename)
        .and_then(|mut file| {
            file.read_to_string(&mut input)
                .expect("Error reading file content");
            Ok(())
        })
        .expect(&format!("Error opening {}", filename);

    input
        .parse()
        .map(toml2json)
        .expect("Error converting toml to json")
}
