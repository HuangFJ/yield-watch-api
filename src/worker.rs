use diesel::prelude::*;
use models::Pool;
use utils;
use std::error::Error as StdError;

pub fn refresh_coins(pool: &Pool) -> Result<(), Box<StdError>> {
    let json = utils::request_json("https://api.coinmarketcap.com/v1/ticker/?convert=CNY", None)?;
    let arr = json.as_array().unwrap();

    let mut sql_string = String::from("INSERT INTO coins (id,name,symbol,rank,available_supply,total_supply,max_supply,last_updated) VALUES ");
    for item in arr {
        sql_string.push_str(&format!(
            "('{}','{}','{}',{},{},{},{},1367107200),",
            item["id"].as_str().unwrap_or(""),
            item["name"].as_str().unwrap_or(""),
            item["symbol"].as_str().unwrap_or(""),
            item["rank"].as_str().unwrap_or("0"),
            item["available_supply"].as_str().unwrap_or("0"),
            item["total_supply"].as_str().unwrap_or("0"),
            item["max_supply"].as_str().unwrap_or("null"),
        ));
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
    println!("{}", sql_string);

    pool.get()?.execute(&sql_string)?;

    Ok(())
}
