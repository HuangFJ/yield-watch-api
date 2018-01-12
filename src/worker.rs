#[macro_use]
extern crate diesel;

pub mod utils;
pub mod schema;
pub mod models;

use diesel::prelude::*;
use models::{Coin};

fn mysql_conn()->MysqlConnection{
    let json = utils::json_from_tomlfile("Rocket.toml");
    let env = match option_env!("ENV") {
        Some(v) => v,
        None => "development",
    };
    let db_uri = json[env]["mysql"].as_str().unwrap();
    MysqlConnection::establish(&db_uri)
        .expect(&format!("Error connecting to {}", json[env]["mysql"]))
}

fn main() {
    use schema::coins::dsl::{id, coins};
    let conn=mysql_conn();
    let coin: Coin = coins.first(&conn).unwrap();
    println!("{:?}", coin);
}
