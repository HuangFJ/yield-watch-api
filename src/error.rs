use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket::http::{ContentType, Status};
use rocket::Response;
use std::io::Cursor;

#[derive(Debug)]
pub enum E {
    SmsSendLimit,
    SmsSendInterval(i64),
    SmsVerifyNotFound,
    SmsVerified,
    SmsVerifyLimit,
    SmsVerifyExpired,
    SmsVerifyInvalid,
    SmsMobileInvalid,
    AccessTokenNotFound,
    AccessTokenInvalid,
    SessionExpired,
    UserNotFound,
    SessionIsOwned,
    CoinNotFound,
    Unknown,
}

impl E {
    pub fn spec(&self) -> (u32, String) {
        match *self {
            E::SmsSendLimit => (
                1,
                "过去24小时发送的短信已超过10条，请稍后再试。".into(),
            ),
            E::SmsSendInterval(secs) => (
                2,
                format!("两次短信发送时间需间隔{secs}秒。", secs = secs),
            ),
            E::SmsVerifyNotFound => (3, "无验证短信。".into()),
            E::SmsVerified => (4, "该验证码已使用。".into()),
            E::SmsVerifyLimit => (5, "验证码尝试已超过10次。".into()),
            E::SmsVerifyExpired => (6, "验证码已过期。".into()),
            E::SmsVerifyInvalid => (7, "验证码错误。".into()),
            E::SmsMobileInvalid => (8, "手机号格式错误。".into()),
            E::AccessTokenNotFound => (9, "没有令牌。".into()),
            E::AccessTokenInvalid => (10, "无效的令牌。".into()),
            E::SessionExpired => (11, "会话已过期。".into()),
            E::UserNotFound => (12, "该用户不存在。".into()),
            E::SessionIsOwned => (13, "会话已有所属。".into()),
            E::CoinNotFound => (14, "加密币不存在。".into()),
            E::Unknown => (999, "未知错误。".into()),
        }
    }
}

impl<'r> Responder<'r> for E {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        let (err, msg): (u32, String) = self.spec();
        let body = json!({
            "err": err,
            "msg": msg,
        }).to_string();
        let mut res = Response::new();
        res.set_status(Status::BadRequest);
        res.set_sized_body(Cursor::new(body));
        res.set_header(ContentType::JSON);

        Ok(res)
    }
}

use std::option;
use regex;
use std::sync;
use mysql;
use std::sync::mpsc;
use std::str;
use crypto::symmetriccipher;

impl From<option::NoneError> for E {
    fn from(_: option::NoneError) -> Self {
        E::Unknown
    }
}

impl From<regex::Error> for E {
    fn from(_: regex::Error) -> Self {
        E::Unknown
    }
}

impl<T> From<sync::PoisonError<T>> for E {
    fn from(_: sync::PoisonError<T>) -> Self {
        E::Unknown
    }
}

impl<T> From<mpsc::SendError<T>> for E {
    fn from(_: mpsc::SendError<T>) -> Self {
        E::Unknown
    }
}

impl From<mysql::Error> for E {
    fn from(e: mysql::Error) -> Self {
        println!("{:?}", e);
        E::Unknown
    }
}

impl From<str::Utf8Error> for E {
    fn from(_: str::Utf8Error) -> Self {
        E::Unknown
    }
}

impl From<symmetriccipher::SymmetricCipherError> for E {
    fn from(_: symmetriccipher::SymmetricCipherError) -> Self {
        E::Unknown
    }
}