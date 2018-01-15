extern crate time;

use mysql::{self, Pool, Value};
use utils;
use std::error::Error;

pub fn refresh_rates(pool: &Pool) -> Result<(), Box<Error>> {
    let json = utils::request_json("https://api.fixer.io/latest?base=USD", None)?;

    pool.prep_exec("REPLACE INTO _cache (k,v,created) VALUES (?,?,?)", ("rates", json.to_string(), time::get_time().sec))?;
    Ok(())
}

pub fn refresh_coins(pool: &Pool) -> Result<(), Box<Error>> {
    let json = utils::request_json("https://api.coinmarketcap.com/v1/ticker/?convert=CNY&limit=10000", None)?;

    let mut sql_string = String::from("INSERT INTO coins (id,name,symbol,rank,available_supply,total_supply,max_supply) VALUES ");
    let mut params = vec![];
    for item in json.as_array().unwrap() {
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

    pool.prep_exec(sql_string, params)?;
    pool.prep_exec("REPLACE INTO _cache (k,v,created) VALUES (?,?,?)", ("coins", json.to_string(), time::get_time().sec))?;

    Ok(())
}

pub fn refresh_prices(pool: &Pool) -> Result<u64, Box<Error>> {
    let mut result = pool.prep_exec(
        "SELECT t1.id,t1.last_updated,t2.max_updated \
        FROM coins t1 \
        INNER JOIN \
            (SELECT MIN(last_updated) min_updated,MAX(last_updated) max_updated \
            FROM coins) t2 \
        ON t1.last_updated=t2.min_updated \
        LIMIT 1", ())?;
    let (id, last_updated, max_updated): (String, i64, i64) = mysql::from_row(result.next().unwrap()?);
    let mut max_updated = max_updated;

    let now = time::get_time().sec;
    if now - max_updated >= 8 {
        println!("Fetching {} between {} and {}", id, last_updated, now);
        let start = last_updated * 1000;
        let end = now * 1000;
        let json = utils::request_json(&format!("https://graphs.coinmarketcap.com/currencies/{}/{}/{}/", id, start, end), None)?;

        let mut sql_string = String::from("INSERT IGNORE INTO prices (coin_id,price_usd,volume_usd,price_btc,price_platform,created) VALUES ");
        let mut params = vec![];

        for (idx, price_usd) in json["price_usd"].as_array().unwrap().iter().enumerate(){
            sql_string.push_str("(?,?,?,?,?,?),");

            let ts = price_usd[0].as_u64().unwrap()/1000;
            let p_usd = price_usd[1].as_f64().unwrap();
            let p_btc = json["price_btc"][idx][1].as_f64().unwrap();
            let v_usd = json["volume_usd"][idx][1].as_f64().unwrap();
            let p_platform = if json["price_platform"].is_array(){
                Value::from(json["price_platform"][idx][1].as_f64().unwrap())
            }else{
                Value::from("null")
            };

            params.push(Value::from(&id));
            params.push(Value::from(p_usd));
            params.push(Value::from(v_usd));
            params.push(Value::from(p_btc));
            params.push(p_platform);
            params.push(Value::from(ts));
        }
        sql_string.pop();

        pool.prep_exec(sql_string, params)?;
        pool.prep_exec("UPDATE coins SET last_updated=? WHERE id=?", (now, id))?;
        max_updated = now;
    }

    let mut sleep_secs = 8 - (time::get_time().sec - max_updated);
    if sleep_secs < 0 {
        sleep_secs = 0;
    }
    println!("Waiting {} secs more", sleep_secs);
    Ok(sleep_secs as u64)
}
