#![feature(core_intrinsics)]

#[allow(dead_code)]
fn type_of<T>(_: &T) -> &'static str {
    unsafe { std::intrinsics::type_name::<T>() }
}

#[macro_use]
extern crate diesel;

pub mod utils;
pub mod schema;
pub mod models;

use diesel::prelude::*;
use models::{Coin, NewCoin};
use diesel::result::Error;

fn mysql_uri() -> String {
    let json = utils::json_from_tomlfile("Rocket.toml");
    let env = match option_env!("ENV") {
        Some(v) => v,
        None => "development",
    };

    json[env]["mysql"].as_str().unwrap().to_string()
}

fn main() {
    let values = vec![
        NewCoin {
            id: "nim",
            name: "nimiq",
            symbol: "NET",
            rank: 12,
            available_supply: 13243,
            total_supply: 34343,
            max_supply: None,
            last_updated: 1,
        },
    ];

    let conn = models::db_conn(&mysql_uri());

    conn.test_transaction::<_, Error, _>(|| {
        models::insert_coins(&conn, &values);

        Ok(())
    })
}
