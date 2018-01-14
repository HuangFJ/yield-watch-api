#![feature(core_intrinsics)]

#[allow(dead_code)]
fn type_of<T>(_: &T) -> &'static str {
    unsafe { std::intrinsics::type_name::<T>() }
}

#[macro_use]
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;

mod utils;
mod schema;
mod models;

use diesel::prelude::*;
use diesel::result::Error;
use diesel::expression::sql_literal;
use models::Coin;

fn mysql_uri() -> String {
    let json = utils::json_from_tomlfile("Rocket.toml");
    let env = match option_env!("ENV") {
        Some(v) => v,
        None => "development",
    };

    json[env]["mysql"].as_str().unwrap().to_string()
}

fn get_coins_from_cmc() {
    let data = utils::request_json("https://api.coinmarketcap.com/v1/ticker/?convert=CNY", None)
        .and_then(|arr| {
            let sql_pre = "INSERT INTO t1 (a,b,c) VALUES ";
            // for val in arr {
                
            // }
            let sql_post = "ON DUPLICATE KEY UPDATE \
                            name=VALUES(name)\
                            ,symbol=VALUES(symbol)\
                            ,rank=VALUES(rank)\
                            ,available_supply=VALUES(available_supply)\
                            ,total_supply=VALUES(total_supply)\
                            ,max_supply=VALUES(max_supply)";
            sql(format!("(1,2,3),(4,5,6)"))
                .execute(conn)
                .expect("Error executing raw SQL");
        });
    println!("{:?}", data);
}

fn main() {
    use schema::coins::dsl::*;

    let conn = models::db_conn(&mysql_uri());

    let coin = conn.test_transaction::<Vec<Coin>, Error, _>(|| {
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
        println!("Mysql insert: {:?}", inserted_count);

        coins.load(&conn)
    });

    println!("{:?}", coin);
    get_coins_from_cmc();
}
