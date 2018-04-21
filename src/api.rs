use std::sync::{Arc, Mutex, RwLock};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use time;
use mysql::Pool;
use rocket::State;
use rocket::response::status;
use rocket_contrib::{Json, Value};
use regex::Regex;

use worker;
use models::{self, QueryString, Session, SmsFactory};
use error::E;

#[error(502)]
fn bad_gateway() -> E {
    E::Unknown
}

#[error(500)]
fn internal_server_error() -> E {
    E::Unknown
}

#[error(400)]
fn bad_request() -> E {
    E::Unknown
}

/// ### send authorization sms
/// - /api/sms
/// - Content-Type: application/json
/// - post
/// ```js
/// {
///     "mobile": "1xxxxxxxxxx"
/// }
/// ```
/// - http 200:
/// ```js
/// {
///     "interval": 123
/// }
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[post("/sms", format = "application/json", data = "<data>")]
fn sms(
    data: Json<Value>,
    mysql_pool: State<Pool>,
    sms_fac_lock: State<Mutex<SmsFactory>>,
) -> Result<Json<Value>, E> {
    let sms_fac = sms_fac_lock.lock()?;
    let mobile = data["mobile"].as_str()?;
    if !Regex::new(r"^1\d{10}$")?.is_match(mobile) {
        return Err(E::SmsMobileInvalid);
    }
    let interval = sms_fac.gen_code(&mysql_pool, mobile)?;

    Ok(Json(json!({
        "interval": interval,
    })))
}

/// ### verify authorization sms code
/// - /api/sms/auth
/// - Content-Type: application/json
/// - post
/// ```js
/// {
///     "mobile": "1xxxxxxxxxx",
///     "code": 1234
/// }
/// ```
/// - http 200:
/// ```js
/// {
///     "user": {
///         "id": 123,
///         "name": "user name",
///         "is_signup": true
///     },
///     "access_token": "xxxx"
/// }
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[post("/sms/auth", format = "application/json", data = "<data>")]
fn sms_auth(
    data: Json<Value>,
    mysql_pool: State<Pool>,
    sms_fac_lock: State<Mutex<SmsFactory>>,
) -> Result<Json<Value>, E> {
    let sms_fac = sms_fac_lock.lock()?;
    let mobile = data["mobile"].as_str()?;
    let code = data["code"].as_i64()?;
    if !Regex::new(r"^1\d{10}$")?.is_match(mobile) {
        return Err(E::SmsMobileInvalid);
    }

    sms_fac
        .check_code(&mysql_pool, mobile, code as u32)
        .and_then(|_| {
            // create session
            let sess = Session::new(&mysql_pool, mobile)?;
            Ok(Json(json!({
            "access_token": Session::id_to_access_token(&sess.id)?
        })))
        })
}

/// ### fetch session owner's info
/// - /api/me?access_token={access_token}
/// - Content-Type: application/json
/// - get
/// - http 200:
/// ```js
/// {
///     "id": 123,
///     "name": "abc",
///     "created": 123
/// }
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[get("/me")]
fn me_get(
    qs: QueryString,
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let worker_state = &*(worker_state_lock.read().unwrap());
    Ok(Json(json!({
        "id": user.id,
        "name": user.name,
        "created": user.created,
        "usd2cny_rate": worker_state.usd2cny_rate
    })))
}

/// ### register session owner
/// - /api/me?access_token={access_token}
/// - Content-Type: application/json
/// - post
/// ```js
/// {
///     "name": "abc"
/// }
/// ```
/// - http 200:
/// ```js
/// {
///     "id": 123,
///     "name": "abc",
///     "created": 123
/// }
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[post("/me", data = "<data>")]
fn me_post(qs: QueryString, mysql_pool: State<Pool>, data: Json<Value>) -> Result<Json<Value>, E> {
    let mut sess = Session::from_query_string(&mysql_pool, &qs)?;
    let name = data["name"].as_str()?;
    sess.signup(&mysql_pool, name)?;
    let user = sess.user()?;
    Ok(Json(json!({
        "id": user.id,
        "name": user.name,
        "created": user.created
    })))
}

/// ### user current coins states
/// - /api/states?access_token={access_token}
/// - Content-Type: application/json
/// - get
/// - http 200:
/// ```js
/// {
///     "balance": 123,
///     "states":
///     [
///       {
///         "coin_id": "abc",
///         "amount": 12.3,
///         "created": 123,
///         "value_cny": 12.3, //invalid state if this is None
///         "coin": { //invalid state if this is None
///             "id": "abc",
///             "name": "abc",
///             "symbol": "abc",
///             "price_usd": 12.3,
///             "volume_usd": 12.3,
///             "market_cap_usd": 12.3,
///             "percent_change_24h": 12.3, //percent
///             "rank": 123
///         }
///       },
///       ...
///     ]
/// }
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[get("/states")]
fn states(
    qs: QueryString,
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let worker_state = &*(worker_state_lock.read().unwrap());
    let balance = user.balance(&mysql_pool)?;
    let mut user_states = user.states(&mysql_pool, worker_state, None)?;
    user_states.reverse();

    let mut rt_states_list = vec![];
    let mut got_coins = vec![];
    for state in user_states {
        if !got_coins.contains(&state.coin_id) {
            got_coins.push(state.coin_id.clone());
            if state.coin.is_none() {
                rt_states_list.push(json!({
                    "coin_id": state.coin_id,
                    "amount": state.amount,
                    "created": state.created,
                }));
                continue;
            }
            let coin = state.coin.unwrap();
            let coin_json = json!({
                "id": coin.id.clone(),
                "name": coin.name.clone(),
                "symbol": coin.symbol.clone(),
                "price_usd": coin.price_usd,
                "volume_usd": coin.volume_usd,
                "market_cap_usd": coin.market_cap_usd,
                "percent_change_24h": coin.percent_change_24h,
                "rank": coin.rank,
                "no": coin.no,
            });
            rt_states_list.push(json!({
                "coin_id": state.coin_id,
                "amount": state.amount,
                "created": state.created,
                "value_cny": coin.price_usd * state.amount * worker_state.usd2cny_rate,
                "coin": coin_json
            }));
        }
    }

    let sum = rt_states_list
        .iter()
        .fold(0.0, |acc, x| acc + x["value_cny"].as_f64().unwrap());
    println!("Sum: ￥{}", sum);

    Ok(Json(json!({
        "balance": balance,
        "states": rt_states_list,
    })))
}

#[get("/states/<coin_id>")]
fn coin_states(
    qs: QueryString,
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
    coin_id: String,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let worker_state = &*(worker_state_lock.read().unwrap());
    let ret = user.states(&mysql_pool, worker_state, Some(&coin_id))?;
    let data: Vec<Value> = ret.iter().map(|record| {
        json!({
            "id": record.id,
            "amount": record.amount,
            "created": record.created,
        })
    }).collect();

    Ok(Json(json!(data)))
}

#[put("/states", format = "application/json", data = "<data>")]
fn put_states(
    qs: QueryString,
    mysql_pool: State<Pool>,
    data: Json<Value>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let id = data["id"].as_i64()?;
    let coin_id = data["coin_id"].as_str()?;
    let created = data["created"].as_i64()?;
    let amount = data["amount"].as_f64()?;
    user.put_states(&mysql_pool, id, coin_id, created, amount)?;

    Ok(Json(json!(null)))
}

#[delete("/states", format = "application/json", data = "<data>")]
fn delete_states(
    qs: QueryString,
    mysql_pool: State<Pool>,
    data: Json<Value>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let id = data["id"].as_i64()?;
    user.del_states(&mysql_pool, id)?;

    Ok(Json(json!(null)))
}

#[put("/balance", format = "application/json", data = "<data>")]
fn put_balance(
    qs: QueryString,
    mysql_pool: State<Pool>,
    data: Json<Value>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let id = data["id"].as_i64()?;
    let created = data["created"].as_i64()?;
    let amount = data["amount"].as_f64()?;
    user.put_balance(&mysql_pool, id, created, amount)?;

    Ok(Json(json!(null)))
}

#[delete("/balance", format = "application/json", data = "<data>")]
fn delete_balance(
    qs: QueryString,
    mysql_pool: State<Pool>,
    data: Json<Value>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let id = data["id"].as_i64()?;
    user.del_balance(&mysql_pool, id)?;

    Ok(Json(json!(null)))
}

/// ### user portfolio historical value
/// - /api/states/history?access_token={access_token}
/// - Content-Type: application/json
/// - get
/// - http 200:
/// ```js
/// [
///     [123, 12.3],
///     ...
/// ]
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[get("/states/history")]
fn states_history(
    qs: QueryString,
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let worker_state = &*(worker_state_lock.read().unwrap());
    // all states order by created time asc
    let user_states = user.states(&mysql_pool, worker_state, None)?;

    let end_ts = time::get_time().sec;
    let mut origin_ts = 0i64;
    // all states group by coin type and map to state points {COIN => [(ASC TIMESTAMP, AMOUNT)]}
    let mut coin_to_states = HashMap::<String, Vec<(i64, f64)>>::new();
    for state in user_states.iter() {
        if origin_ts == 0i64 {
            origin_ts = state.created;
        }

        if !coin_to_states.contains_key(&state.coin_id) {
            coin_to_states.insert(state.coin_id.clone(), vec![]);
        }
        let vec = coin_to_states.get_mut(&state.coin_id)?;
        vec.push((state.created, state.amount));
    }

    println!("USER STATES GROUP BY COIN: {:?}", coin_to_states);
    // {ASC TIMESTAMP => (TIMESTAMP, VALUE)}
    let mut mix_points = BTreeMap::<i64, (i64, f64)>::new();
    // each type of coin
    for (coin_id, states) in coin_to_states {
        // get the coin historical points among timestamp window
        // {ASC TIMESTAMP => (PRICE, AMOUNT)}
        let coin_points = models::coin_history(&mysql_pool, &coin_id, origin_ts, end_ts, &states)?;
        for (ts, item) in coin_points {
            let value_cny = item.0 * item.1 * worker_state.usd2cny_rate;
            if !mix_points.contains_key(&ts) {
                mix_points.insert(ts, (ts, value_cny));
            } else {
                let exist_item = mix_points.get_mut(&ts).unwrap();
                exist_item.1 = exist_item.1 + value_cny;
            }
        }
    }

    let ret: Vec<(i64, f64)> = mix_points.values().cloned().collect();
    Ok(Json(json!(ret)))
}

/// ### get coin detail
/// - /api/coins/<coin_id>?access_token={access_token}
/// - Content-Type: application/json
/// - get
/// - http 200:
/// ```js
/// {
///     "id": "abc",
///     "name": "abc",
///     "symbol": "abc",
///     "rank": 123,
///     "price_usd": 12.3,
///     "volume_usd": 12.3,
///     "market_cap_usd": 12.3,
///     "percent_change_24h": 12.3,
///     "percent_change_1h": 12.3,
///     "history": [
///         [123, 12.3],
///         ...
///     ]
/// }
/// ```
/// - http 400:
/// ```js
/// {
///     "err": 123,
///     "msg": "error message"
/// }
/// ```
#[get("/coins/<coin_id>")]
fn coin(
    qs: QueryString,
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
    coin_id: String,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    sess.user()?;
    let worker_state = &*(worker_state_lock.read().unwrap());

    let coin = worker_state.coins.iter().find(|&x| x.id == coin_id);
    if coin.is_none() {
        return Err(E::CoinNotFound);
    }
    let coin = coin.unwrap();

    let end_ts = time::get_time().sec;
    let origin_ts = end_ts - 30 * 24 * 3600;
    let states = vec![(origin_ts, 0.0)];
    let points = models::coin_history(&mysql_pool, &coin_id, origin_ts, end_ts, &states)?;

    let history: Vec<(&i64, f64)> = points
        .iter()
        .map(|(k, item)| (k, item.0 * worker_state.usd2cny_rate))
        .collect();

    Ok(Json(json!({
        "id": coin.id,
        "name": coin.name,
        "symbol": coin.symbol,
        "rank": coin.rank,
        "price_usd": coin.price_usd,
        "price_cny": coin.price_cny,
        "volume_usd": coin.volume_usd,
        "market_cap_usd": coin.market_cap_usd,
        "percent_change_24h": coin.percent_change_24h,
        "percent_change_7d": coin.percent_change_7d,
        "history": history,
        "no": coin.no,
    })))
}

#[get("/coins")]
fn coins(worker_state_lock: State<Arc<RwLock<worker::State>>>) -> Result<Json<Value>, E> {
    let worker_state = &*(worker_state_lock.read().unwrap());
    let mut arr = vec![];
    for coin in worker_state.coins.iter() {
        arr.push(json!({
            "id": coin.id,
            "name": coin.name,
            "symbol": coin.symbol,
            "rank": coin.rank,
            "price_usd": coin.price_usd,
            "volume_usd": coin.volume_usd,
            "market_cap_usd": coin.market_cap_usd,
            "percent_change_24h": coin.percent_change_24h,
            "percent_change_1h": coin.percent_change_1h,
            "no": coin.no,
        }))
    }
    Ok(Json(json!(arr)))
}

#[options("/<_path..>", rank = 1)]
fn options_all(_path: PathBuf) -> status::NoContent {
    status::NoContent
}
