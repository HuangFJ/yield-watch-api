use utils::request_json;
use rocket::config::Config;
use std::ops::Deref;
use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};

use models::{Coin, Pool, PoolConn};
use diesel::prelude::*;

pub struct DbConn(pub PoolConn);

impl<'a, 'r> FromRequest<'a, 'r> for DbConn {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> request::Outcome<DbConn, ()> {
        let pool = request.guard::<State<Pool>>()?;
        match pool.get() {
            Ok(conn) => Outcome::Success(DbConn(conn)),
            Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
        }
    }
}

impl Deref for DbConn {
    type Target = MysqlConnection;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[get("/")]
fn index(config: State<Config>, conn: DbConn) -> String {
    use schema::coins::dsl::*;
    let coin_list = coins.load::<Coin>(&*conn);

    println!("{:?}", config);
    println!("{:?}", coin_list);

    let ret = request_json("https://api.coinmarketcap.com/v1/global/", Some(30)).unwrap();
    format!("{:?}", ret)
}
