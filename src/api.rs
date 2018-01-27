use time;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use mysql::{self, Pool};

use rocket::{Config, State};
use rocket::outcome::IntoOutcome;
use rocket::http::{Cookie, Cookies};
use rocket::request::{Form, FromRequest, Outcome as ReqOutcome, Request};
use rocket::response::{Flash, Redirect};
use rocket_contrib::{Json, Template, Value};

use worker;

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

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> ReqOutcome<User, ()> {
        let mysql_pool = request.guard::<State<Pool>>()?;
        request
            .cookies()
            .get_private("sess_id")
            .and_then(|cookie| cookie.value().parse().ok())
            .map(|sess_id| User { id: sess_id })
            .or_forward(())
    }
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
        Flash::error(Redirect::to("/api/login"), "Invalid username/password.")
    }
}

#[post("/logout")]
fn logout(mut cookies: Cookies) -> Flash<Redirect> {
    cookies.remove_private(Cookie::named("sess_id"));

    Flash::success(Redirect::to("/api/login"), "Successfully logged out.")
}
