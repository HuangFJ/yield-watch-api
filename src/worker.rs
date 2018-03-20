use std::error::Error;
use std::sync::{Arc, RwLock};
use serde_json;
use mysql::{self, Pool, Value};
use time;
use std::collections::HashMap;

use utils;

#[derive(Debug)]
pub struct Coin {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: i64,
    pub price_usd: f64,
    pub price_btc: f64,
    pub volume_usd: f64,
    pub market_cap_usd: f64,
    pub available_supply: f64,
    pub total_supply: f64,
    pub max_supply: f64,
    pub percent_change_1h: f64,
    pub percent_change_24h: f64,
    pub percent_change_7d: f64,
    pub last_updated: i64,
    pub price_cny: f64,
    pub volume_cny: f64,
    pub market_cap_cny: f64,
    pub no: i64,
}

impl Coin {
    fn from_json(j: &serde_json::Value) -> Coin {
        Coin {
            id: j["id"].as_str().unwrap().into(),
            name: j["name"].as_str().unwrap().into(),
            symbol: j["symbol"].as_str().unwrap().into(),
            rank: j["rank"].as_str().unwrap().parse().unwrap(),
            price_usd: j["price_usd"].as_str().unwrap_or("0").parse().unwrap(),
            price_btc: j["price_btc"].as_str().unwrap_or("0").parse().unwrap(),
            volume_usd: j["24h_volume_usd"].as_str().unwrap_or("0").parse().unwrap(),
            market_cap_usd: j["market_cap_usd"].as_str().unwrap_or("0").parse().unwrap(),
            available_supply: j["available_supply"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap(),
            total_supply: j["total_supply"].as_str().unwrap_or("0").parse().unwrap(),
            max_supply: j["max_supply"].as_str().unwrap_or("0").parse().unwrap(),
            percent_change_1h: j["percent_change_1h"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap(),
            percent_change_24h: j["percent_change_24h"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap(),
            percent_change_7d: j["percent_change_7d"]
                .as_str()
                .unwrap_or("0")
                .parse()
                .unwrap(),
            last_updated: j["last_updated"].as_str().unwrap_or("0").parse().unwrap(),
            price_cny: j["price_cny"].as_str().unwrap_or("0").parse().unwrap(),
            volume_cny: j["24h_volume_cny"].as_str().unwrap_or("0").parse().unwrap(),
            market_cap_cny: j["market_cap_cny"].as_str().unwrap_or("0").parse().unwrap(),
            no: j["no"].as_i64().unwrap_or(0),
        }
    }
}

pub struct State {
    pub usd2cny_rate: f64,
    pub coins: Vec<Coin>,
}

impl State {
    pub fn init(mysql_pool: &Pool) -> State {
        let mut state = State {
            usd2cny_rate: 0.0,
            coins: vec![],
        };
        let ret = mysql_pool
            .prep_exec("SELECT k,v FROM _cache WHERE k IN ('coins','rates')", ())
            .unwrap();
        for row in ret {
            let (k, v): (String, String) = mysql::from_row(row.unwrap());
            let value: serde_json::Value = serde_json::from_str(v.as_str()).unwrap();
            match k.as_str() {
                "coins" => for row in value.as_array().unwrap().iter() {
                    let item = Coin::from_json(row);
                    state.coins.push(item);
                },
                "rates" => {
                    state.usd2cny_rate = value["rates"]["CNY"].as_f64().unwrap();
                }
                _ => (),
            }
        }

        state
    }
}

pub fn refresh_rates(pool: &Pool, lock: &Arc<RwLock<State>>) -> Result<(), Box<Error>> {
    // fetch exchange rate
    let value = utils::request_json("https://api.fixer.io/latest?base=USD", None)?;

    {
        let mut state = lock.write().unwrap();
        (*state).usd2cny_rate = value["rates"]["CNY"].as_f64().unwrap();
    }
    pool.prep_exec(
        "REPLACE INTO _cache (k,v,created) VALUES (?,?,?)",
        ("rates", value.to_string(), time::get_time().sec),
    )?;

    Ok(())
}

pub fn refresh_coins(pool: &Pool, lock: &Arc<RwLock<State>>) -> Result<(), Box<Error>> {
    // fetch all coins price instantly
    let mut value = utils::request_json(
        "https://api.coinmarketcap.com/v1/ticker/?convert=CNY&limit=10000",
        None,
    )?;

    let quick_value = utils::request_json(
        "https://s2.coinmarketcap.com/generated/search/quick_search.json",
        None,
    )?;

    let mut quick_map = HashMap::<&str, serde_json::Value>::new();
    for row in quick_value.as_array().unwrap() {
        quick_map.insert(row["slug"].as_str().unwrap(), row["id"].clone());
    }

    for row in value.as_array_mut().unwrap() {
        row["no"] = quick_map[row["id"].as_str().unwrap()].clone();
    }

    let mut sql_string = String::from(
        "INSERT INTO coins (id,name,symbol,rank,available_supply,total_supply,max_supply,no) VALUES ",
    );
    let mut data: Vec<Coin> = vec![];
    let mut params = vec![];

    for row in value.as_array().unwrap() {
        let item = Coin::from_json(row);
        sql_string.push_str("(?,?,?,?,?,?,?,?),");
        // Vec store only similar type, so we wrap the raw type with mysql Value
        params.push(Value::from(item.id.clone()));
        params.push(Value::from(item.name.clone()));
        params.push(Value::from(item.symbol.clone()));
        params.push(Value::from(item.rank));
        params.push(Value::from(item.available_supply));
        params.push(Value::from(item.total_supply));
        params.push(Value::from(item.max_supply));
        params.push(Value::from(item.no));
        data.push(item);
    }
    sql_string.pop();
    sql_string.push_str(
        " ON DUPLICATE KEY UPDATE \
         name=VALUES(name)\
         ,symbol=VALUES(symbol)\
         ,rank=VALUES(rank)\
         ,available_supply=VALUES(available_supply)\
         ,total_supply=VALUES(total_supply)\
         ,max_supply=VALUES(max_supply)\
         ,no=VALUES(no)\
         ,score=score+1",
    );

    pool.prep_exec(sql_string, params)?;
    {
        let mut state = lock.write().unwrap();
        (*state).coins = data;
    }
    pool.prep_exec(
        "REPLACE INTO _cache (k,v,created) VALUES (?,?,?)",
        ("coins", value.to_string(), time::get_time().sec),
    )?;

    Ok(())
}

pub fn refresh_prices(pool: &Pool) -> Result<u64, Box<Error>> {
    // fetch the specific coin historical price
    let mut result = pool.prep_exec(
        "SELECT t1.id,t1.last_updated,t2.max_updated \
         FROM coins t1 \
         INNER JOIN \
         (SELECT MIN(last_updated) min_updated,MAX(last_updated) max_updated \
         FROM coins) t2 \
         ON t1.last_updated!=t2.max_updated \
         ORDER BY t1.score DESC,t1.last_updated ASC \
         LIMIT 1",
        (),
    )?;
    let (id, last_updated, max_updated): (String, i64, i64) =
        mysql::from_row(result.next().unwrap()?);
    let mut max_updated = max_updated;

    let now = time::get_time().sec;
    // the interval between requests should be large than 7 seconds
    if now - max_updated >= 7 {
        println!("Fetching {} between {} and {}", id, last_updated, now);
        let start = last_updated * 1000;
        let end = now * 1000;
        // only fetch the historical data since last fetching
        let json = match utils::request_json(
            &format!(
                "https://graphs2.coinmarketcap.com/currencies/{}/{}/{}/", // graphs.coinmarketcap.com
                id, start, end
            ),
            None,
        ) {
            Ok(v) => v,
            Err(e) => {
                // request failed, skip for next cycle
                pool.prep_exec("UPDATE coins SET score=score-1 WHERE id=?", (id,))?;
                return Err(e);
            }
        };

        let mut sql_string = String::from(
            "INSERT IGNORE INTO prices \
             (coin_id,price_usd,volume_usd,price_btc,price_platform,created) VALUES ",
        );
        let mut params = vec![];

        for (idx, price_usd) in json["price_usd"].as_array().unwrap().iter().enumerate() {
            sql_string.push_str("(?,?,?,?,?,?),");

            let ts = price_usd[0].as_u64().unwrap() / 1000;
            let p_usd = price_usd[1].as_f64().unwrap();
            if p_usd == 0f64 {
                continue;
            }
            let p_btc = json["price_btc"][idx][1].as_f64().unwrap_or(0.0);
            let v_usd = json["volume_usd"][idx][1].as_f64().unwrap_or(0.0);
            let p_platform = if json["price_platform"].is_array() {
                Value::from(json["price_platform"][idx][1].as_f64().unwrap_or(0.0))
            } else {
                Value::NULL
            };

            params.push(Value::from(&id));
            params.push(Value::from(p_usd));
            params.push(Value::from(v_usd));
            params.push(Value::from(p_btc));
            params.push(p_platform);
            params.push(Value::from(ts));
        }
        sql_string.pop();

        // no data, skip db writing
        if !params.is_empty() {
            pool.prep_exec(sql_string, params)?;
        }

        pool.prep_exec(
            "UPDATE coins SET last_updated=?,score=0 WHERE id=?",
            (now, id),
        )?;
        max_updated = now;
    }

    let mut sleep_secs = 7 - (time::get_time().sec - max_updated);
    if sleep_secs < 0 {
        sleep_secs = 0;
    }
    println!("Waiting {} secs more", sleep_secs);
    Ok(sleep_secs as u64)
}
