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
use diesel::result::Error;
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
    use schema::coins::dsl::*;

    let conn = models::db_conn(&mysql_uri());

    conn.test_transaction::<Coin, Error, _>(|| {
        let inserted_count = diesel::insert_into(coins)
            .values((
                id.eq("nim"),
                name.eq("nimiq"),
                symbol.eq("NET"),
                rank.eq(12),
                available_supply.eq(13243),
                total_supply.eq(34343),
                last_updated.eq(1),
            ))
            .execute(&conn);
        println!("{:?}", inserted_count);

        coins.first(&conn)
    });
}
