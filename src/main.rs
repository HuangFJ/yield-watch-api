#![feature(plugin)]
#![plugin(rocket_codegen)]
#![allow(dead_code)]

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

use rocket::fairing::AdHoc;

fn main() {
    rocket::ignite()
        .attach(AdHoc::on_attach(|rocket| {
            let config = rocket.config().clone();
            Ok(rocket
                .manage(models::init_pool(config.get_str("mysql").unwrap()))
                .manage(config))
        }))
        .mount("/api", routes![api::index])
        .launch();
}
