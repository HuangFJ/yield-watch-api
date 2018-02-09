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
use crypto::{aes, blockmodes, buffer, symmetriccipher};
use crypto::buffer::{ReadBuffer, WriteBuffer};

pub fn request_json(url: &str, timeout: Option<u64>) -> Result<Json, Box<Error>> {
    println!("Request: {}", url);
    let mut core = Core::new()?;
    let handle = core.handle();
    let timeout = Timeout::new(Duration::from_secs(timeout.unwrap_or(60u64)), &handle)?;
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn json_from_tomlfile(filename: &str) -> Json {
    let mut input = String::new();
    File::open(filename)
        .and_then(|mut file| {
            file.read_to_string(&mut input)
                .expect("Error reading file content");
            Ok(())
        })
        .expect(&format!("Error opening {}", filename));

    input
        .parse()
        .map(toml2json)
        .expect("Error converting toml to json")
}

pub fn query_quote(str: &str) -> String {
    rfc3986_encode(str, false)
}

pub fn rfc3986_encode(str: &str, full_url: bool) -> String {
    str.as_bytes().iter().fold(String::new(), |mut out, &b| {
        match b as char {
            // unreserved:
            'A' ... 'Z'
            | 'a' ... 'z'
            | '0' ... '9'
            | '-' | '.' | '_' | '~' => out.push(b as char),

            // gen-delims:
            ':' | '/' | '?' | '#' | '[' | ']' | '@' |
            // sub-delims:
            '!' | '$' | '&' | '"' | '(' | ')' | '*' |
            '+' | ',' | ';' | '='
                if full_url => out.push(b as char),

            ch => out.push_str(&format!("%{:02X}", ch as u8)),
        };

        out
    })
}

pub fn encrypt(data: &[u8], key: &[u8]) -> Result<Vec<u8>, symmetriccipher::SymmetricCipherError> {
    let mut encryptor: Box<symmetriccipher::Encryptor> = aes::cbc_encryptor(
        aes::KeySize::KeySize256,
        key,
        &[0; 16],
        blockmodes::PkcsPadding,
    );

    let mut final_result = Vec::<u8>::new();
    let mut buffer = [0; 4096];
    let mut read_buffer = buffer::RefReadBuffer::new(data);
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = try!(encryptor.encrypt(&mut read_buffer, &mut write_buffer, true));
        final_result.extend(write_buffer.take_read_buffer().take_remaining());
        match result {
            buffer::BufferResult::BufferUnderflow => break,
            buffer::BufferResult::BufferOverflow => {}
        }
    }

    Ok(final_result)
}

pub fn decrypt(
    encrypted_data: &[u8],
    key: &[u8],
) -> Result<Vec<u8>, symmetriccipher::SymmetricCipherError> {
    let mut decryptor: Box<symmetriccipher::Decryptor> = aes::cbc_decryptor(
        aes::KeySize::KeySize256,
        key,
        &[0; 16],
        blockmodes::PkcsPadding,
    );

    let mut final_result = Vec::<u8>::new();
    let mut buffer = [0; 4096];
    let mut read_buffer = buffer::RefReadBuffer::new(encrypted_data);
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = try!(decryptor.decrypt(&mut read_buffer, &mut write_buffer, true));
        final_result.extend(write_buffer.take_read_buffer().take_remaining());
        match result {
            buffer::BufferResult::BufferUnderflow => break,
            buffer::BufferResult::BufferOverflow => {}
        }
    }

    Ok(final_result)
}
