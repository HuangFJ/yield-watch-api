#![feature(plugin, core_intrinsics)]
#![plugin(rocket_codegen)]
#![allow(dead_code)]

fn type_of<T>(_: &T) -> &'static str {
    unsafe { std::intrinsics::type_name::<T>() }
}

#[macro_use]
extern crate diesel;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate r2d2;
extern crate r2d2_diesel;
extern crate rocket;
extern crate serde_json;
extern crate tokio_core;

mod utils;
mod schema;
mod models;
mod api;
mod worker;

use std::{thread, time};

fn main() {
    let server = rocket::ignite();
    let config = server.config().clone();

    let pool = models::init_pool(config.get_str("mysql").unwrap());
    let pool_tx = pool.clone();

    thread::spawn(move || loop {
        match worker::refresh_coins(&pool_tx) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing coins: {}", &*e.to_string()),
        }
        println!("Sleep 300 secs for next refreshing coins...");
        thread::sleep(time::Duration::from_secs(300));
        break;
    });
    server
        .manage(pool)
        .manage(config)
        .mount("/api", routes![api::index])
        .launch();
}
