use schema::coins;
use diesel;
use diesel::prelude::*;

pub fn db_conn(uri: &str) -> MysqlConnection {
    MysqlConnection::establish(uri).expect(&format!("Error connecting to {}", uri))
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

impl Coin {
    pub fn add<'a>(
        conn: &MysqlConnection,
        id: &str,
        name: &str,
        symbol: &str,
        rank: i16,
        available_supply: i64,
        total_supply: i64,
        max_supply: Option<i64>,
        last_updated: i32,
    ) {
        let new = NewCoin {
            id: id,
            name: name,
            symbol: symbol,
            rank: rank,
            available_supply: available_supply,
            total_supply: total_supply,
            max_supply: max_supply,
            last_updated: last_updated,
        };
        let ret = diesel::insert_into(coins::table)
            .values(&new)
            .execute(conn)
            .unwrap();
        println!("{:?}", ret);
    }
}
