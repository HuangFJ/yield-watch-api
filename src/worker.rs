#[macro_use]
extern crate diesel;

pub mod utils;
pub mod schema;
pub mod models;

use diesel::prelude::*;
use models::Coin;

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

    Coin::add(&conn, "net", "nimiq", "NET", 12, 13243, 34343, None, 1);

    let all_coins = coins.load::<Coin>(&conn).unwrap();
    println!("{:?}", all_coins);
}
