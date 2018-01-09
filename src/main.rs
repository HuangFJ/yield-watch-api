#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate serde_json;
extern crate tokio_core;
extern crate rocket;

pub mod api;

fn main() {
    rocket::ignite()
        .mount("/api", routes![api::index])
        .launch();
}
