use worker;
use rocket::{Config, State};
use std::sync::{Arc, RwLock};
use mysql::{self, Pool};
use rocket_contrib::{Json, Value};
use std::collections::HashMap;
use rocket_contrib::Template;
use rocket::http::{Cookie, Cookies};
use rocket::request::Form;
use rocket::response::{Flash, Redirect};
use time;

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
    coins_lock: State<Arc<RwLock<worker::SharedCoins>>>,
    rates_lock: State<Arc<RwLock<worker::SharedRates>>>,
) -> Json<Value> {
    let coins_shared = coins_lock.read().unwrap();
    let coins_array = &(*coins_shared).0;

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

#[get("/login")]
fn login_page() -> Template {
    let mut context = HashMap::new();
    context.insert("flash", "hi!");

    Template::render("login", &context)
}

#[post("/login", data = "<login>")]
fn login_post(mut cookies: Cookies, login: Form<Login>, cnf: State<Config>) -> Flash<Redirect> {
    let cookie_max_age_hours =
        time::Duration::hours(cnf.get_int("cookie_max_age_hours").unwrap_or(24));
    let cookie_domain = cnf.address.clone();

    if login.get().username == "jon" && login.get().password == "" {
        cookies.add_private(
            Cookie::build("sess_id", 1.to_string())
                .domain(cookie_domain)
                .path("/")
                .secure(true)
                .http_only(true)
                .max_age(cookie_max_age_hours)
                .finish(),
        );
        Flash::success(Redirect::to("/api"), "Successfully logged in.")
    } else {
        Flash::error(Redirect::to("/login"), "Invalid username/password.")
    }
}
