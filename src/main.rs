#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate serde_json;
extern crate tokio_core;
extern crate rocket;

pub mod api;
mod utils;

use rocket::fairing::AdHoc;

fn main() {
    rocket::ignite()
        .attach(AdHoc::on_attach(|rocket|{
            println!("Attaching local config.");
            let config = rocket.config().clone();
            Ok(rocket.manage(config))
        }))
        .mount("/api", routes![api::index])
        .launch();
}
