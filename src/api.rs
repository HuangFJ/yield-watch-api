use worker;
use rocket::State;
use std::sync::{Arc, RwLock};
use mysql::{self, Pool};
use rocket_contrib::{Json, Value};

#[derive(Serialize, Deserialize)]
struct UserCoin {
    coin_id: String,
    amount: f64,
    created: i32,
    balance_cny: f64,
}

#[get("/")]
fn index(
    mysql_pool: State<Pool>,
    coins_lock: State<Arc<RwLock<worker::SharedCoins>>>,
    rates_lock: State<Arc<RwLock<worker::SharedRates>>>,
) -> Json<Value> {
    let coins_shared = coins_lock.read().unwrap();
    let coins_json = &(*coins_shared).0;
    let coins_array = coins_json.as_array().unwrap();

    let rates_shared = rates_lock.read().unwrap();
    let rates_json = &(*rates_shared).0;
    let usd2cny = rates_json["rates"]["CNY"].as_f64().unwrap();

    let mut total_balance = 0f64;

    let user_states: Vec<UserCoin> = mysql_pool
        .prep_exec(
            "SELECT coin_id,amount,created FROM states WHERE user_id=? ORDER BY created ASC",
            (1,),
        )
        .map(|ret| {
            ret.map(|x| x.unwrap())
                .map(|row| {
                    let (coin_id, amount, created): (String, f64, i32) = mysql::from_row(row);
                    let seek = coins_array.into_iter().find(|&x| x["id"] == coin_id);
                    let coin = seek.unwrap();
                    let price_usd = coin["price_usd"].as_str().unwrap().parse::<f64>().unwrap();

                    let balance_cny = price_usd * usd2cny * amount;
                    total_balance += balance_cny;
                    UserCoin {
                        coin_id: coin_id,
                        amount: amount,
                        created: created,
                        balance_cny: balance_cny,
                    }
                })
                .collect()
        })
        .unwrap();

    Json(json!({
        "total_balance": total_balance,
        "detail": user_states
    }))
}
