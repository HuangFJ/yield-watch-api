use std::collections::HashMap;
use std::ops::Deref;
use rocket::request::{FormItems, FromForm};
use alisms::SmsBody;
use std::sync::mpsc::Sender;
use mysql::{self, Pool};
use rand::{self, Rng};
use time;
use crypto::digest::Digest;
use crypto::md5::Md5;
use rustc_serialize::base64::FromBase64;

use error::E;
use utils;

pub enum Sms {
    Verification { phone: String, code: String },
}

pub struct SmsFactory {
    pub key_id: String,
    pub key_secret: String,
    pub tx: Sender<SmsBody>,
}

impl SmsFactory {
    pub fn gen_code(&self, mysql_pool: &Pool, mobile: &str) -> Result<(i64, i64), E> {
        let now = time::get_time().sec;
        let row = mysql_pool
            .prep_exec(
                "SELECT COUNT(0) AS send_times,MAX(created) AS last_created FROM sms \
                 WHERE mobile=? AND created>? ORDER BY created DESC",
                (mobile, now - 86400),
            )
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let (send_times, last_created): (i64, Option<i64>) = mysql::from_row(row);
        let last_created = last_created.unwrap_or(0);

        if send_times > 10 {
            return Err(E::SmsSendLimit);
        } else if last_created + send_times * 60 > now {
            return Err(E::SmsSendInterval(send_times * 60));
        }
        let code = rand::thread_rng().gen_range(1000, 9999);
        mysql_pool
            .prep_exec(
                "INSERT INTO sms (mobile,code,err_times,created) VALUES (?,?,?,?)",
                (mobile, code, 0, now),
            )
            .unwrap();

        Ok((code, (send_times + 1) * 60))
    }

    pub fn check_code(&self, mysql_pool: &Pool, mobile: &str, code_input: u32) -> Result<(), E> {
        let ret = mysql_pool
        .prep_exec("SELECT id,code,err_times,created FROM sms WHERE mobile=? ORDER BY created DESC LIMIT 1", (mobile,))
        .unwrap()
        .next();
        if ret.is_none() {
            return Err(E::SmsVerifyNotFound);
        }
        let (id, code, err_times, created): (i64, u32, i64, i64) =
            mysql::from_row(ret.unwrap().unwrap());

        if err_times < 0 {
            Err(E::SmsVerified)
        } else if err_times > 10 {
            Err(E::SmsVerifyLimit)
        } else if created + 600 < time::get_time().sec {
            Err(E::SmsVerifyExpired)
        } else if code != code_input {
            mysql_pool
                .prep_exec("UPDATE sms SET err_times=err_times+1 WHERE id=?", (id,))
                .unwrap();
            Err(E::SmsVerifyInvalid)
        } else {
            mysql_pool
                .prep_exec("UPDATE sms SET err_times=-1 WHERE id=?", (id,))
                .unwrap();
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

    pub fn send(&self, sms: Sms) {
        match sms {
            Sms::Verification { phone, code } => {
                self.tx
                    .send(SmsBody {
                        key_id: self.key_id.clone(),
                        key_secret: self.key_secret.clone(),
                        sign_name: "yield助手".to_string(),
                        template_code: "SMS_123673246".to_string(),
                        phone_numbers: phone.to_string(),
                        template_param: format!("{{\"code\":\"{code}\"}}", code = code),
                        out_id: "".to_string(),
                    })
                    .unwrap();
            }
        }
    }
}

#[derive(Debug)]
pub struct QueryString<'a>(HashMap<&'a str, String>);

impl<'f> FromForm<'f> for QueryString<'f> {
    type Error = E;
    fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<QueryString<'f>, E> {
        let mut qs = QueryString(HashMap::new());

        for (key, value) in items {
            let key = key.as_str();
            let value = value.url_decode().unwrap();
            match key {
                "access_token" => {
                    let mut sh = Md5::new();
                    sh.input_str("j0n");
                    let key = sh.result_str();
                    let enc = value.as_str().from_base64().unwrap();
                    let dec =
                        utils::decrypt(&enc, &key.as_bytes()).map_err(|_| E::AccessTokenInvalid)?;
                    let sess_id = String::from_utf8(dec).unwrap();
                    qs.0.insert("_sess_id", sess_id);
                }
                _ => (),
            }

            qs.0.insert(key, value);
        }
        Ok(qs)
    }
}

impl<'a> Deref for QueryString<'a> {
    type Target = HashMap<&'a str, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
