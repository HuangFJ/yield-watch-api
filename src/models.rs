use schema::coins;
use diesel::prelude::*;
use r2d2_diesel::ConnectionManager;
use r2d2;

pub type Pool = r2d2::Pool<ConnectionManager<MysqlConnection>>;

pub fn init_pool(uri: &str) -> Pool {
    r2d2::Pool::new(ConnectionManager::<MysqlConnection>::new(uri)).expect("Failed to create pool")
}

#[derive(Queryable, Debug)]
pub struct Coin {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: i16,
    pub available_supply: i64,
    pub total_supply: i64,
    pub max_supply: Option<i64>,
    pub last_updated: i32,
}

#[derive(Insertable)]
#[table_name = "coins"]
pub struct NewCoin<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub symbol: &'a str,
    pub rank: i16,
    pub available_supply: i64,
    pub total_supply: i64,
    pub max_supply: Option<i64>,
    pub last_updated: i32,
}
