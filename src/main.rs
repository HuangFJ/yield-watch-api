#![feature(plugin, core_intrinsics, custom_derive)]
#![plugin(rocket_codegen)]
#![allow(dead_code)]

fn type_of<T>(_: &T) -> &'static str {
    unsafe { std::intrinsics::type_name::<T>() }
}

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate mysql;
extern crate r2d2;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate time;
extern crate tokio_core;

mod utils;
mod api;
mod worker;
mod models;
mod alisms;
mod hmac_sha1;

use std::{thread, time as stdtime};
use std::sync::{Arc, RwLock};
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
    // 每隔5分钟刷新一次币列表
    thread::spawn(move || loop {
        match worker::refresh_coins(&pool_tx1, &worker_state_lock_tx1) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing coins: {}", &*e.to_string()),
        }
        thread::sleep(stdtime::Duration::from_secs(300));
    });
    // 每隔6秒获取一次币的价格历史数据
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
    // 每隔1天刷新一次汇率
    thread::spawn(move || loop {
        match worker::refresh_rates(&pool_tx3, &worker_state_lock_tx2) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing rates: {}", &*e.to_string()),
        }
        thread::sleep(stdtime::Duration::from_secs(86400));
    });
    server
        .manage(pool_mysql)
        .manage(config)
        .manage(worker_state_lock)
        .attach(Template::fairing())
        .mount(
            "/api",
            routes![api::index, api::login_page, api::login_post],
        )
        .launch();
}
