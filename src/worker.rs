#[macro_use]
extern crate diesel;

pub mod utils;
pub mod schema;
pub mod models;

use diesel::prelude::*;
use models::Coin;

fn mysql_uri() -> String{
    let json = utils::json_from_tomlfile("Rocket.toml");
    let env = match option_env!("ENV") {
        Some(v) => v,
        None => "development",
    };

    json[env]["mysql"].to_string()
}

fn mysql_conn(uri: &str) -> MysqlConnection {
    MysqlConnection::establish(uri).expect(&format!("Error connecting to {}", uri))
}

fn main() {
    use schema::coins::dsl::{coins, id};
    let conn = mysql_conn(&mysql_uri());
    let coin: Coin = coins.first(&conn).unwrap();
    println!("{:?}", coin);
}
