use time;
use std::sync::{Arc, Mutex, RwLock};
use mysql::{self, Pool};
use rocket::{Config, State};
use rocket::http::{Cookie, Cookies};
use rocket::request::Form;
use rocket::response::{Flash, Redirect};
use rocket_contrib::{Json, Value};
use regex::Regex;
use crypto::digest::Digest;
use crypto::md5::Md5;
use rustc_serialize::base64::{ToBase64, URL_SAFE};

use worker;
use models::{QueryString, Sms, SmsFactory};
use error::E;
use utils;

/// ### send authorization sms
/// - /api/sms
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
    let sms_fac = sms_fac_lock.lock().unwrap();
    let mobile = data["mobile"].as_str().unwrap();
    if !Regex::new(r"^1\d{10}$").unwrap().is_match(mobile) {
        return Err(E::SmsMobileInvalid);
    }
    let (code, interval) = sms_fac.gen_code(&mysql_pool, mobile)?;
    sms_fac.send(Sms::Verification {
        phone: mobile.to_string(),
        code: code.to_string(),
    });
    println!("sms code: {}, interval: {}", code, interval);

    Ok(Json(json!({
        "interval": interval,
    })))
}

/// ### verify authorization sms code
/// - /api/sms/auth
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
    let sms_fac = sms_fac_lock.lock().unwrap();
    let mobile = data["mobile"].as_str().unwrap();
    let code = data["code"].as_str().unwrap().parse::<u32>().unwrap();
    if !Regex::new(r"^1\d{10}$").unwrap().is_match(mobile) {
        return Err(E::SmsMobileInvalid);
    }

    sms_fac.check_code(&mysql_pool, mobile, code).and_then(|_| {
        let sess_id = "hello";

        let mut sh = Md5::new();
        sh.input_str("jon");
        let key = sh.result_str();
        let access_token = utils::encrypt(sess_id.as_bytes(), &key.as_bytes())
            .ok()
            .unwrap();

        Ok(Json(json!({
            "uid": "",
            "access_token": ""
        })))
    })
}

#[get("/me?<qs>")]
fn me_get(qs: QueryString) -> Result<(), E> {
    let sess_id = match qs.get("_sess_id") {
        Some(v) => v,
        None => return Err(E::AccessTokenNotFound),
    };

    println!("{}", sess_id);
    println!("{:?}", qs);

    Ok(())
}

#[get("/me")]
fn me_get_default() {
    let sess_id = "hello";

    let mut sh = Md5::new();
    sh.input_str("j0n");
    let key = sh.result_str();
    let enc = utils::encrypt(sess_id.as_bytes(), &key.as_bytes())
        .ok()
        .unwrap();
    println!("{:?}", enc.to_base64(URL_SAFE))
}

#[derive(Serialize, Deserialize)]
struct UserCoin {
    coin_id: String,
    amount: f64,
    created: i32,
    balance_cny: f64,
}

#[derive(FromForm)]
struct Login {
    username: String,
    password: String,
}

struct User {
    id: i64,
}

#[get("/")]
fn index(
    mysql_pool: State<Pool>,
    worker_state_lock: State<Arc<RwLock<worker::State>>>,
) -> Json<Value> {
    let worker_state = &*(worker_state_lock.read().unwrap());
    println!("Original address: {:p}", worker_state);
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
                    let seek = worker_state.coins.iter().find(|&x| x.id == coin_id);
                    let coin = seek.unwrap();

                    let balance_cny = coin.price_usd * worker_state.usd2cny_rate * amount;
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
        "balance": total_balance,
        "states": user_states
    }))
}

#[post("/login", data = "<login>")]
fn login_post(mut cookies: Cookies, login: Form<Login>, cnf: State<Config>) -> Flash<Redirect> {
    // let mut context = HashMap::new();
    // context.insert("flash", "hi!");
    // Template::render("login", &context)
    let cookie_max_age_hours =
        time::Duration::hours(cnf.get_int("cookie_max_age_hours").unwrap_or(24));
    let cookie_domain = cnf.get_str("cookie_domain").unwrap();

    if login.get().username == "jon" && login.get().password == "" {
        cookies.add_private(
            Cookie::build("sess_id", 1.to_string())
                .domain(cookie_domain.to_string())
                .path("/")
                .secure(true)
                .http_only(true)
                .max_age(cookie_max_age_hours)
                .finish(),
        );
        Flash::success(Redirect::to("/api"), "Successfully logged in.")
    } else {
        Flash::error(Redirect::to("/api/login"), "Invalid username/password.")
    }
}

#[post("/logout")]
fn logout(mut cookies: Cookies) -> Flash<Redirect> {
    cookies.remove_private(Cookie::named("sess_id"));

    Flash::success(Redirect::to("/api/login"), "Successfully logged out.")
}
