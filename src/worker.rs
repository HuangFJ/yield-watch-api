#[macro_use]
extern crate diesel;

pub mod utils;
pub mod schema;
pub mod models;

use diesel::prelude::*;
use models::{Coin, NewCoin};

fn mysql_uri() -> String {
    let json = utils::json_from_tomlfile("Rocket.toml");
    let env = match option_env!("ENV") {
        Some(v) => v,
        None => "development",
    };

    json[env]["mysql"].as_str().unwrap().to_string()
}

fn main() {
    use schema::coins::dsl::coins;
    let conn = models::db_conn(&mysql_uri());

    let coin = NewCoin {
        id: "net",
        name: "nimiq",
        symbol: "NET",
        rank: 12,
        available_supply: 13243,
        total_supply: 34343,
        max_supply: None,
        last_updated: 1,
    };
    coin.save(&conn);

    let all_coins = coins.load::<Coin>(&conn).unwrap();
    println!("{:?}", all_coins);
}
