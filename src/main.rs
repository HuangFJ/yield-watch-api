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
extern crate mysql;

mod utils;
mod schema;
mod models;
mod api;
mod worker;

use std::{thread, time};

fn main() {
    let server = rocket::ignite();
    let config = server.config().clone();

    let mysql_uri = config.get_str("mysql").unwrap();
    let pool_mysql = mysql::Pool::new(mysql_uri).unwrap();
    let pool_tx = pool_mysql.clone();

    thread::spawn(move || loop {
        match worker::refresh_coins(&pool_tx) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing coins: {}", &*e.to_string()),
        }
        println!("Sleep 300 secs for next refreshing coins...");
        thread::sleep(time::Duration::from_secs(300));
    });
    server
        .manage(models::init_pool(mysql_uri))
        .manage(config.clone())
        .mount("/api", routes![api::index])
        .launch();
}
