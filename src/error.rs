pub enum E {
    SmsSendLimit,
    SmsSendInterval(i64),
    SmsVerifyNotFound,
    SmsVerified,
    SmsVerifyLimit,
    SmsVerifyExpired,
    SmsVerifyInvalid,
    Unknown,
}

impl E {
    fn spec(&self) -> (i64, &'static str) {
        match *self {
            E::SmsSendLimit => (
                1,
                "过去24小时发送的短信已超过10条，请稍后再试。",
            ),
            E::SmsSendInterval(secs) => (
                2,
                &format!("两次短信发送时间需间隔{secs}秒。", secs = secs),
            ),
            E::SmsVerifyNotFound => (3, "无验证短信。"),
            E::SmsVerified => (4, "该验证码已使用。"),
            E::SmsVerifyLimit => (5, "验证码尝试已超过10次。"),
            E::SmsVerifyExpired => (6, "验证码已过期。"),
            E::SmsVerifyInvalid => (7, "验证码错误。"),
            E::Unknown => (999, "未知错误。"),
        }
    }
}
