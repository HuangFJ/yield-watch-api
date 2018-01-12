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

impl<'a> NewCoin<'a> {
    pub fn save(&self, conn: &MysqlConnection) {
        let ret = diesel::insert_into(coins::table)
            .values(self)
            .execute(conn)
            .unwrap();
        println!("{:?}", ret);
    }
}
