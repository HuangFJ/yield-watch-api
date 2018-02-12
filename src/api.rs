use std::sync::{Arc, Mutex, RwLock};
use mysql::Pool;
use rocket::State;
use rocket_contrib::{Json, Value};
use regex::Regex;

use worker;
use models::{self, QueryString, Session, Sms, SmsFactory, UserCoin};
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
    let (code, interval) = sms_fac.gen_code(&mysql_pool, mobile)?;
    sms_fac.send(Sms::Verification {
        phone: mobile.to_string(),
        code: code.to_string(),
    })?;
    println!("sms code: {}, interval: {}", code, interval);

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
fn me_get(qs: QueryString, mysql_pool: State<Pool>) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    Ok(Json(json!({
        "id": user.id,
        "name": user.name,
        "created": user.created
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
    let mut user_states = user.states(&mysql_pool, worker_state)?;
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
            });
            rt_states_list.push(json!({
                "coin_id": state.coin_id,
                "amount": state.amount,
                "created": state.created,
                "value_cny": coin.price_usd*state.amount*worker_state.usd2cny_rate,
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

use std::collections::{BTreeMap, HashMap};
use time;

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
    let user_states = user.states(&mysql_pool, worker_state)?;

    let now = time::get_time().sec;
    let mut bucket = 0i64;
    // all states group by coin type and map to state points {COIN => [(ASC TIME, AMOUNT)]}
    let mut coin_to_states = HashMap::<String, Vec<(i64, f64)>>::new();
    for state in user_states.iter() {
        let since_time = state.created;
        if bucket == 0i64 {
            // get 300 points sample
            bucket = (now - since_time) / models::POINTS_NUM;
        }

        if !coin_to_states.contains_key(&state.coin_id) {
            coin_to_states.insert(state.coin_id.clone(), vec![]);
        }
        let vec = coin_to_states.get_mut(&state.coin_id)?;
        vec.push((state.created / bucket, state.amount));
    }

    // {ASC TIME => VALUE}
    let mut points = BTreeMap::<i64, f64>::new();
    println!("USER STATES GROUP BY COIN: {:?}", coin_to_states);
    println!("INTERVAL TIME: {}s", bucket);
    // each type of coin
    for coin in coin_to_states.keys() {
        let states = coin_to_states.get(coin)?;
        // get the coin historical price since the first state point and interval with bucket
        // {ASC TIME, PRICE}
        let prices = models::historical_prices(&mysql_pool, coin, states[0].0, bucket)?;
        for (price_time, price_usd) in prices {
            for idx in 0..(states.len()) {
                let first = states[idx];
                let second = states.get(idx + 1);
                if second.is_none() || (price_time >= first.0 && price_time < second.unwrap().0) {
                    let value = price_usd * first.1 * worker_state.usd2cny_rate;

                    if !points.contains_key(&price_time) {
                        points.insert(price_time, 0.0);
                    }
                    let base = points[&price_time];
                    let val = points.get_mut(&price_time)?;
                    *val = base + value;
                    break;
                }
            }
        }
    }

    Ok(Json(json!(points)))
}
