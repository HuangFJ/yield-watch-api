use utils::request_json;
use rocket::config::Config;
use std::ops::Deref;
use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};

use models::{Pool, Coin};
use diesel::prelude::*;
use r2d2;
use r2d2_diesel::ConnectionManager;

pub struct DbConn(pub r2d2::PooledConnection<ConnectionManager<MysqlConnection>>);

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
