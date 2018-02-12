#![feature(plugin, custom_derive, try_trait)]
#![plugin(rocket_codegen)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate mysql;
extern crate r2d2;
extern crate rand;
extern crate rocket;
extern crate rocket_contrib;
extern crate rustc_serialize;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate time;
extern crate tokio_core;
extern crate uuid;
extern crate regex;
extern crate crypto;

mod utils;
mod api;
mod worker;
mod error;
mod models;
mod alisms;
mod hmac_sha1;

use std::{thread, time as stdtime};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use rocket_contrib::Template;

fn main() {
    let server = rocket::ignite();
    let config = server.config().clone();

    let pool_mysql = mysql::Pool::new(config.get_str("mysql").unwrap()).unwrap();
    let pool_tx1 = pool_mysql.clone();
    let pool_tx2 = pool_mysql.clone();
    let pool_tx3 = pool_mysql.clone();

    let worker_state_lock = Arc::new(RwLock::new(worker::State::init(&pool_mysql)));
    let worker_state_lock_tx1 = worker_state_lock.clone();
    let worker_state_lock_tx2 = worker_state_lock.clone();
    // update all coins price every 5 minutes
    thread::spawn(move || loop {
        match worker::refresh_coins(&pool_tx1, &worker_state_lock_tx1) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing coins: {}", &*e.to_string()),
        }
        thread::sleep(stdtime::Duration::from_secs(300));
    });
    // update specific coin historical price every 7 seconds
    thread::spawn(move || loop {
        let sleep_secs = match worker::refresh_prices(&pool_tx2) {
            Ok(secs) => secs,
            Err(e) => {
                println!("Error while refreshing prices: {}", &*e.to_string());
                6
            }
        };
        thread::sleep(stdtime::Duration::from_secs(sleep_secs));
    });
    // update currency exchange rate every day
    thread::spawn(move || loop {
        match worker::refresh_rates(&pool_tx3, &worker_state_lock_tx2) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing rates: {}", &*e.to_string()),
        }
        thread::sleep(stdtime::Duration::from_secs(86400));
    });

    let (tx, rx) = mpsc::channel();
    let sms_fac_lock = Mutex::new(models::SmsFactory::new(
        config.get_str("ali_sms_key_id").unwrap(),
        config.get_str("ali_sms_key_secret").unwrap(),
        tx,
    ));
    // asynchronous sms sending
    thread::spawn(move || loop {
        let sms_body = rx.recv().unwrap();
        alisms::sms_api(sms_body);
    });

    server
        .manage(pool_mysql)
        .manage(config)
        .manage(worker_state_lock)
        .manage(sms_fac_lock)
        .attach(Template::fairing())
        .mount(
            "/api",
            routes![api::sms, api::sms_auth, api::me_get, api::me_post, api::states, api::states_history],
        )
        .catch(errors![api::bad_gateway, api::bad_request, api::internal_server_error])
        .launch();
}
