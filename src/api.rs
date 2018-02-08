use time;
use std::sync::{Arc, Mutex, RwLock};
use mysql::{self, Pool};
use rocket::{Config, State};
use rocket::http::{Cookie, Cookies};
use rocket::request::Form;
use rocket::response::{Flash, Redirect};
use rocket_contrib::{Json, Value};
use regex::Regex;
use uuid::Uuid;

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

    sms_fac.check_code(&mysql_pool, mobile, code as u32).and_then(|_| {
        let sess_id =Uuid::new_v4().hyphenated().to_string();

        Ok(Json(json!({
            "access_token": Session::id_to_access_token(&sess_id)
        })))
    })
}

#[get("/me")]
fn me_get(qs: QueryString, mysql_pool: State<Pool>) -> Result<(), E> {
    let sess = Session::from_query_string(&mysql_pool, &qs)?;
    println!("{:?}", sess);
    println!("{:?}", qs);

    let sess_id = "hello";

    println!("{}", Session::id_to_access_token(sess_id));

    Ok(())
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
