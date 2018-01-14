use schema::coins;
use diesel;
use diesel::prelude::*;
use diesel::result::Error;

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

// single_column
//     name.eq("Sean")
// multiple_columns
//     (name.eq("Tess"), hair_color.eq("Brown"))
// insertable_struct
//     r#"{ "name": "Sean", "hair_color": "Black" }"#
// single_column_batch
//     &vec![name.eq("Sean"), name.eq("Tess")]
// tuple_batch
//     &vec![
//         (name.eq("Sean"), hair_color.eq("Black")),
//         (name.eq("Tess"), hair_color.eq("Brown")),
//     ]
// insertable_struct_batch
//     r#"[
//         { "name": "Sean", "hair_color": "Black" },
//         { "name": "Tess", "hair_color": "Brown" }
//     ]"#
pub fn insert_coins(conn: &MysqlConnection, records: &Vec<NewCoin>) -> Result<(), Box<Error>> {
    use schema::coins::dsl::*;

    conn.transaction::<_, Error, _>(|| {
        diesel::insert_into(coins).values(records).execute(conn)?;

        let result = coins.load::<Coin>(conn)?;
        println!("{:?}", result);
        Ok(())
    });
    Ok(())
}
