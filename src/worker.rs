extern crate time;

use mysql::{self, Pool};
use utils;
use std::error::Error as StdError;

pub fn refresh_coins(pool: &Pool) -> Result<(), Box<StdError>> {
    let json = utils::request_json("https://api.coinmarketcap.com/v1/ticker/?convert=CNY&limit=10000", None)?;
    let arr = json.as_array().unwrap();

    let mut sql_string = String::from("INSERT INTO coins (id,name,symbol,rank,available_supply,total_supply,max_supply) VALUES ");
    let mut params = vec![];
    for item in arr {
        sql_string.push_str("(?,?,?,?,?,?,?),");
        params.push(item["id"].as_str().unwrap_or(""));
        params.push(item["name"].as_str().unwrap_or(""));
        params.push(item["symbol"].as_str().unwrap_or(""));
        params.push(item["rank"].as_str().unwrap_or("0"));
        params.push(item["available_supply"].as_str().unwrap_or("0"));
        params.push(item["total_supply"].as_str().unwrap_or("0"));
        params.push(item["max_supply"].as_str().unwrap_or("null"));
    }
    sql_string.pop();
    sql_string.push_str(
        " ON DUPLICATE KEY UPDATE \
         name=VALUES(name)\
         ,symbol=VALUES(symbol)\
         ,rank=VALUES(rank)\
         ,available_supply=VALUES(available_supply)\
         ,total_supply=VALUES(total_supply)\
         ,max_supply=VALUES(max_supply)",
    );
    let mut stmt = pool.prepare(sql_string)?;
    stmt.execute(params)?;

    let mut stmt = pool.prepare("REPLACE INTO _cache (k,v,created) VALUES (?,?,?)")?;
    stmt.execute(("coins", json.to_string(), time::get_time().sec))?;

    let ret: Vec<(String, String, i32)> = pool.prep_exec("SELECT k,v,created FROM _cache", ()).map(|result|{
        result.map(|x| x.unwrap()).map(|row|{
            let (k, v, created) = mysql::from_row(row);
            (k, v, created)
        }).collect()
    })?;
    println!("{:?}", ret);
    Ok(())
}
