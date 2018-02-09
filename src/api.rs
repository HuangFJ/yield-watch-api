use std::sync::{Arc, Mutex, RwLock};
use mysql::Pool;
use rocket::State;
use rocket_contrib::{Json, Value};
use regex::Regex;

use worker;
use models::{QueryString, Session, Sms, SmsFactory};
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

#[get("/")]
fn index(
    qs: QueryString,
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
) -> Result<Json<Value>, E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    let user = sess.user()?;
    let worker_state = &*(worker_state_lock.read().unwrap());
    let mut total_balance = 0f64;

    let user_states = user.states(&mysql_pool, worker_state)?;
    for state in user_states.iter() {
        total_balance += state.balance_cny;
    }

    Ok(Json(json!({
        "total_balance": total_balance,
        "states": user_states
    })))
}
