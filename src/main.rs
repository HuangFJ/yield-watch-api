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
    let pool_tx1 = pool_mysql.clone();
    let pool_tx2 = pool_mysql.clone();
    let pool_tx3 = pool_mysql.clone();
    // 每隔5分钟刷新一次币列表
    thread::spawn(move || loop {
        match worker::refresh_coins(&pool_tx1) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing coins: {}", &*e.to_string()),
        }
        thread::sleep(time::Duration::from_secs(300));
    });
    // 每隔6秒获取一次币的价格历史数据
    thread::spawn(move||loop {
        let sleep_secs = match worker::refresh_prices(&pool_tx2) {
            Ok(secs) => secs,
            Err(e) => {println!("Error while refreshing prices: {}", &*e.to_string()); 6},
        };
        thread::sleep(time::Duration::from_secs(sleep_secs));
    });
    // 每隔1天刷新一次汇率
    thread::spawn(move||loop {
        match worker::refresh_rates(&pool_tx3) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing rates: {}", &*e.to_string()),
        }
        thread::sleep(time::Duration::from_secs(86400));
    });
    server
        .manage(models::init_pool(mysql_uri))
        .manage(config.clone())
        .mount("/api", routes![api::index])
        .launch();
}
