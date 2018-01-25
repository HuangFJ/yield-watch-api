#![feature(plugin, core_intrinsics)]
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
#[macro_use] extern crate serde_json;
#[macro_use] extern crate serde_derive;
extern crate tokio_core;

mod utils;
mod api;
mod worker;

use std::{thread, time};
use std::sync::{Arc, RwLock};

fn main() {
    let server = rocket::ignite();
    let config = server.config().clone();

    let pool_mysql = mysql::Pool::new(config.get_str("mysql").unwrap()).unwrap();
    let pool_tx1 = pool_mysql.clone();
    let pool_tx2 = pool_mysql.clone();

    // 每隔5分钟刷新一次币列表
    let coins_lock = Arc::new(RwLock::new(worker::SharedCoins(json!(null))));
    let coins_lock_tx1 = coins_lock.clone();
    thread::spawn(move || loop {
        match worker::refresh_coins(&pool_tx1, &coins_lock_tx1) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing coins: {}", &*e.to_string()),
        }
        thread::sleep(time::Duration::from_secs(300));
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
        thread::sleep(time::Duration::from_secs(sleep_secs));
    });
    // 每隔1天刷新一次汇率
    let rates_lock = Arc::new(RwLock::new(worker::SharedRates(json!(null))));
    let rates_lock_tx = rates_lock.clone();
    thread::spawn(move || loop {
        match worker::refresh_rates(&rates_lock_tx) {
            Ok(_) => (),
            Err(e) => println!("Error while refreshing rates: {}", &*e.to_string()),
        }
        thread::sleep(time::Duration::from_secs(86400));
    });
    server
        .manage(pool_mysql)
        .manage(config)
        .manage(coins_lock)
        .manage(rates_lock)
        .mount("/api", routes![api::index])
        .launch();
}
