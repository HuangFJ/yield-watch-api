use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use rocket::request::{self, FormItems, FromRequest, Request};
use rocket::Outcome;
use alisms::SmsBody;
use std::sync::mpsc::Sender;
use mysql::{self, Pool};
use rand::{self, Rng};
use time;
use crypto::digest::Digest;
use crypto::md5::Md5;
use rustc_serialize::base64::{FromBase64, ToBase64, URL_SAFE};
use rocket::http::Status;
use uuid::Uuid;
use std::io::{Error, ErrorKind};

use error::E;
use utils;
use worker;

pub enum Sms {
    Verification { phone: String, code: String },
}

pub struct SmsFactory {
    pub key_id: String,
    pub key_secret: String,
    pub tx: Sender<SmsBody>,
}

impl SmsFactory {
    pub fn gen_code(&self, mysql_pool: &Pool, mobile: &str) -> Result<i64, E> {
        let now = time::get_time().sec;
        let row = mysql_pool
            .prep_exec(
                "SELECT COUNT(0) AS send_times,MAX(created) AS last_created FROM sms \
                 WHERE mobile=? AND created>? ORDER BY created DESC",
                (mobile, now - 86400),
            )?
            .next()??;
        let (send_times, last_created): (i64, Option<i64>) = mysql::from_row(row);
        let last_created = last_created.unwrap_or(0);

        if send_times > 10 {
            return Err(E::SmsSendLimit);
        } else if last_created + send_times * 60 > now {
            return Err(E::SmsSendInterval(send_times * 60));
        }
        let code = rand::thread_rng().gen_range(1000, 9999);
        mysql_pool
            .start_transaction(false, None, None)
            .and_then(|mut t| {
                t.prep_exec(
                    "INSERT INTO sms (mobile,code,err_times,created) VALUES (?,?,?,?)",
                    (mobile, code, 0, now),
                )?;
                self.send(Sms::Verification {
                    phone: mobile.to_string(),
                    code: code.to_string(),
                }).map_err(|_| Error::new(ErrorKind::Other, "sms worker was down"))?;
                t.commit()
            })
            .map_err(|_| E::SmsSendError)?;
        let interval = (send_times + 1) * 60;
        println!("sms code: {}, interval: {}", code, interval);

        Ok(interval)
    }

    pub fn check_code(&self, mysql_pool: &Pool, mobile: &str, code_input: u32) -> Result<(), E> {
        let ret = mysql_pool
        .prep_exec("SELECT id,code,err_times,created FROM sms WHERE mobile=? ORDER BY created DESC LIMIT 1", (mobile,))?
        .next();
        if ret.is_none() {
            return Err(E::SmsVerifyNotFound);
        }
        let (id, code, err_times, created): (i64, u32, i64, i64) = mysql::from_row(ret??);

        if err_times < 0 {
            Err(E::SmsVerified)
        } else if err_times > 10 {
            Err(E::SmsVerifyLimit)
        } else if created + 600 < time::get_time().sec {
            Err(E::SmsVerifyExpired)
        } else if code != code_input {
            mysql_pool.prep_exec("UPDATE sms SET err_times=err_times+1 WHERE id=?", (id,))?;
            Err(E::SmsVerifyInvalid)
        } else {
            mysql_pool.prep_exec("UPDATE sms SET err_times=-1 WHERE id=?", (id,))?;
            Ok(())
        }
    }

    pub fn new(key_id: &str, key_secret: &str, tx: Sender<SmsBody>) -> Self {
        SmsFactory {
            key_id: key_id.to_string(),
            key_secret: key_secret.to_string(),
            tx: tx,
        }
    }

    pub fn send(&self, sms: Sms) -> Result<(), E> {
        match sms {
            Sms::Verification { phone, code } => {
                self.tx.send(SmsBody {
                    key_id: self.key_id.clone(),
                    key_secret: self.key_secret.clone(),
                    sign_name: "yield助手".to_string(),
                    template_code: "SMS_123673246".to_string(),
                    phone_numbers: phone.to_string(),
                    template_param: format!("{{\"code\":\"{code}\"}}", code = code),
                    out_id: "".to_string(),
                })?;
            }
        }

        Ok(())
    }
}

/// QueryString is used for parsing query string. Although Rocket supply a FromForm strait,
/// but it's behavior is strange. QueryString is more like Flask's `request.args`. It used a
/// HashMap to store items. And you don't need to place a param name in uri.
#[derive(Debug)]
pub struct QueryString<'a>(HashMap<&'a str, String>);

impl<'a, 'r> FromRequest<'a, 'r> for QueryString<'a> {
    type Error = E;
    fn from_request(req: &'a Request<'r>) -> request::Outcome<QueryString<'a>, E> {
        let mut qs = QueryString(HashMap::new());
        match req.uri().query() {
            Some(s) => {
                let items = FormItems::from(s);
                for (key, value) in items {
                    let key = key.as_str();
                    match value.url_decode() {
                        Ok(v) => qs.0.insert(key, v),
                        Err(_) => return Outcome::Failure((Status::BadRequest, E::Unknown)),
                    };
                }
            }
            // not found query string
            None => (),
        }

        Outcome::Success(qs)
    }
}

impl<'a> Deref for QueryString<'a> {
    type Target = HashMap<&'a str, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub mobile: String,
    user: Option<User>,
    pub accessed: i64,
    pub created: i64,
}

/// This Session is based on mysql database. For reduce db queries, I encrypt session's id
/// as a url-safe access token. When it come in with request, it will be decrypted. If success,
/// then fetch session and user's infomation from db.
impl Session {
    pub const KEY: &'static str = "j0n";
    const EXPIRES_IN: i64 = 2592000;

    /// encrypt session id to access_token form
    pub fn id_to_access_token(sess_id: &str) -> Result<String, E> {
        let mut sh = Md5::new();
        sh.input_str(Self::KEY);
        let key = sh.result_str();
        let enc = utils::encrypt(sess_id.as_bytes(), &key.as_bytes())?;

        Ok(enc.to_base64(URL_SAFE))
    }

    /// decrypt access_token to session id
    pub fn access_token_to_id(access_token: &str) -> Result<String, E> {
        let mut sh = Md5::new();
        sh.input_str(Session::KEY);
        let key = sh.result_str();
        let enc = access_token
            .from_base64()
            .map_err(|_| E::AccessTokenInvalid)?;
        let dec = utils::decrypt(&enc, &key.as_bytes()).map_err(|_| E::AccessTokenInvalid)?;
        let sess_id = String::from_utf8(dec).map_err(|_| E::AccessTokenInvalid)?;

        Ok(sess_id)
    }

    /// borrow user from session if user exist or raise user not found error
    pub fn user(&self) -> Result<&User, E> {
        match self.user {
            Some(ref u) => Ok(u),
            None => Err(E::UserNotFound),
        }
    }

    /// signup user
    pub fn signup(&mut self, mysql_pool: &Pool, name: &str) -> Result<(), E> {
        if !self.user.is_none() {
            return Err(E::SessionIsOwned);
        }
        let now = time::get_time().sec;

        mysql_pool
            .start_transaction(false, None, None)
            .and_then(|mut t| {
                let user_id = {
                    let ret = t.prep_exec(
                        "INSERT INTO users (name,mobile,created) VALUES (?,?,?)",
                        (name, &self.mobile, now),
                    )?;
                    ret.last_insert_id() as i64
                };
                {
                    t.prep_exec(
                        "UPDATE _session SET user_id=? WHERE id=?",
                        (user_id, &self.id),
                    )?;
                }
                self.user = Some(User {
                    id: user_id,
                    name: name.to_string(),
                    mobile: self.mobile.clone(),
                    created: now,
                });
                t.commit()
            })?;
        Ok(())
    }

    /// create session for mobile
    pub fn new(mysql_pool: &Pool, mobile: &str) -> Result<Self, E> {
        let now = time::get_time().sec;
        let user = User::find(mysql_pool, None, Some(mobile)).ok();
        let id = Uuid::new_v4().hyphenated().to_string();
        let mut sess = Session {
            id: id,
            mobile: mobile.to_string(),
            user: None,
            accessed: now,
            created: now,
        };
        let user_id = if user.is_none() {
            0
        } else {
            let user = user.unwrap();
            let user_id = user.id;
            sess.user = Some(user);
            user_id
        };

        mysql_pool.prep_exec(
            "INSERT INTO _session (id,mobile,user_id,created,accessed) VALUES (?,?,?,?,?)",
            (&sess.id, mobile, user_id, now, now),
        )?;
        Ok(sess)
    }

    /// fetch session from id
    pub fn init(mysql_pool: &Pool, sess_id: &str) -> Result<Self, E> {
        let now = time::get_time().sec;
        let ret = mysql_pool
            .prep_exec(
                "SELECT mobile,user_id,accessed,created FROM _session WHERE id=?",
                (sess_id,),
            )?
            .next();
        if ret.is_none() {
            return Err(E::SessionExpired);
        }
        let (mobile, user_id, accessed, created): (String, i64, i64, i64) = mysql::from_row(ret??);
        if accessed + Self::EXPIRES_IN < now {
            return Err(E::SessionExpired);
        }
        let user = if user_id != 0 {
            User::find(mysql_pool, Some(user_id), None).ok()
        } else {
            None
        };

        mysql_pool.prep_exec("UPDATE _session SET accessed=? WHERE id=?", (now, sess_id))?;

        Ok(Session {
            id: sess_id.to_string(),
            mobile: mobile,
            user: user,
            created: created,
            accessed: accessed,
        })
    }

    /// search access_token in query string and create session if it exists and is correct
    pub fn from_query_string(mysql_pool: &Pool, qs: &QueryString) -> Result<Self, E> {
        match qs.get("access_token") {
            Some(access_token) => {
                let sess_id = Session::access_token_to_id(access_token)?;
                Session::init(mysql_pool, &sess_id)
            }
            None => Err(E::AccessTokenNotFound), // not found access_token in query string
        }
    }
}

#[derive(Debug)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub mobile: String,
    pub created: i64,
}

impl<'a> User {
    /// fetch user by user id or mobile
    fn find(mysql_pool: &Pool, user_id: Option<i64>, mobile: Option<&str>) -> Result<Self, E> {
        let mut sql = String::from("SELECT id,name,mobile,created FROM users WHERE ");
        let mut params = Vec::new();
        if !user_id.is_none() {
            sql.push_str("id=?");
            params.push(mysql::Value::from(user_id.unwrap()));
        } else if !mobile.is_none() {
            sql.push_str("mobile=?");
            params.push(mysql::Value::from(mobile.unwrap()));
        } else {
            return Err(E::Unknown);
        }
        let ret = mysql_pool.prep_exec(sql, params)?.next();
        if ret.is_none() {
            return Err(E::UserNotFound);
        }
        let (id, name, mobile, created): (i64, String, String, i64) = mysql::from_row(ret??);
        Ok(User {
            id: id,
            name: name,
            mobile: mobile,
            created: created,
        })
    }

    pub fn balance(&self, mysql_pool: &Pool) -> Result<f64, E> {
        let ret = mysql_pool
            .prep_exec(
                "SELECT amount FROM balance WHERE user_id=? ORDER BY created DESC LIMIT 1",
                (self.id,),
            )?
            .next();
        if ret.is_none() {
            Ok(0.0)
        } else {
            let balance: f64 = mysql::from_row(ret??);
            Ok(balance)
        }
    }

    pub fn put_states(
        &self,
        mysql_pool: &Pool,
        id: i64,
        coin_id: &str,
        created: i64,
        amount: f64,
    ) -> Result<(), E> {
        if id > 0 {
            mysql_pool.prep_exec(
                "UPDATE states SET coin_id=?,amount=?,created=? WHERE id=? AND user_id=?",
                (coin_id, amount, created, id, self.id),
            )?;
        } else {
            mysql_pool.prep_exec(
                "INSERT INTO states (user_id,coin_id,amount,created) VALUES (?,?,?,?)",
                (self.id, coin_id, amount, created),
            )?;
        }

        Ok(())
    }

    pub fn del_states(&self, mysql_pool: &Pool, id: i64) -> Result<(), E> {
        mysql_pool.prep_exec("DELETE FROM states WHERE user_id=? AND id=?", (self.id, id))?;

        Ok(())
    }

    pub fn states(
        &self,
        mysql_pool: &Pool,
        worker_state: &'a worker::State,
        coin_id: Option<&str>,
    ) -> Result<Vec<UserCoin<'a>>, E> {
        let ret = if coin_id.is_none() {
            mysql_pool.prep_exec(
                "SELECT id,coin_id,amount,created FROM states WHERE user_id=? ORDER BY created ASC",
                (self.id,),
            )?
        } else {
            mysql_pool.prep_exec(
                "SELECT id,coin_id,amount,created FROM states WHERE user_id=? AND coin_id=? ORDER BY created ASC",
                (self.id, coin_id.unwrap()),
            )?
        };
        let mut states: Vec<UserCoin> = vec![];

        for row in ret {
            match row {
                Ok(row) => {
                    let (id, coin_id, amount, created): (i64, String, f64, i64) =
                        mysql::from_row(row);
                    let coin = worker_state.coins.iter().find(|&x| x.id == coin_id);
                    states.push(UserCoin {
                        id: id,
                        coin_id: coin_id,
                        amount: amount,
                        created: created,
                        coin: coin,
                    });
                }
                Err(_) => (),
            }
        }

        Ok(states)
    }
}

#[derive(Debug, Clone)]
pub struct UserCoin<'a> {
    pub id: i64,
    pub coin_id: String,
    pub amount: f64,
    pub created: i64,
    pub coin: Option<&'a worker::Coin>,
}

pub const POINTS_NUM: i64 = 100;

pub fn coin_history(
    mysql_pool: &Pool,
    coin_id: &String,
    origin_ts: i64,
    end_ts: i64,
    states: &Vec<(i64, f64)>, // [(ASC TIMESTAMP, AMOUNT)]
) -> Result<BTreeMap<i64, (f64, f64)>, E> {
    let mut bucket_size = (end_ts - origin_ts) / POINTS_NUM;
    bucket_size = if bucket_size > 0 { bucket_size } else { 1 };
    let bucket_since = states[0].0 / bucket_size;
    // backward (POINTS_NUM / 10) buckets
    let bucket_since_time = (bucket_since - POINTS_NUM / 10) * bucket_size;
    // {ASC TIMESTAMP => (PRICE, AMOUNT)}
    let mut points = BTreeMap::<i64, (f64, f64)>::new();
    let mut pre_price_usd = 0.0;
    for row in mysql_pool.prep_exec(
        "SELECT FLOOR(created/?) bucket_value,AVG(price_usd) avg_price_usd 
        FROM prices WHERE coin_id=? AND created>? 
        GROUP BY bucket_value ORDER BY bucket_value ASC",
        (bucket_size, coin_id, bucket_since_time),
    )? {
        let (bucket_value, avg_price_usd): (i64, f64) = mysql::from_row(row?);
        if bucket_value < bucket_since {
            pre_price_usd = avg_price_usd;
        } else {
            let bucket_time = bucket_value * bucket_size;
            // fill amount
            let mut item = (avg_price_usd, 0.0);
            for idx in 0..(states.len()) {
                let first = states[idx];
                let second = states.get(idx + 1);
                if second.is_none() || (bucket_time >= first.0 && bucket_time < second.unwrap().0) {
                    item.1 = first.1;
                    break;
                }
            }

            points.insert(bucket_time, item);
        }
    }

    // align and fullfil points
    let mut pre_point = (pre_price_usd, 0.0);
    // {ASC TIMESTAMP => (PRICE, AMOUNT)}
    let mut full_points = BTreeMap::<i64, (f64, f64)>::new();
    for point_value in (origin_ts / bucket_size)..(end_ts / bucket_size) {
        let point_time = point_value * bucket_size;
        if points.contains_key(&point_time) {
            pre_point = points[&point_time];
        }
        full_points.insert(point_time, pre_point);
    }

    Ok(full_points)
}
