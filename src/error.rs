pub type Error = (i64, &'static str);

pub const SMS_SEND_LIMIT: Error = (
    1,
    "过去24小时发送的短信已超过10条，请稍后再试。",
);
pub const SMS_SEND_INTERVAL: Error = (2, "两次短信发送时间需间隔{secs}秒。");
pub const SMS_VERIFY_NOT_FOUND: Error = (3, "无验证短信。");
pub const SMS_VERIFIED: Error = (4, "该验证码已使用。");
pub const SMS_VERIFY_LIMIT: Error = (5, "验证码尝试已超过10次。");
pub const SMS_VERIFY_EXPIRED: Error = (6, "验证码已过期。");
pub const SMS_VERIFY_INVALID: Error = (7, "验证码错误。");

macro_rules! fill {
    ($d:expr) => {
        $d
    };
    (
        $d:expr
        ,
        // Start a repetition:
        $(
            // each repeat 
            $k:ident = $v:expr
        )
        // separated by commas
        ,
        // one or more times
        +
    ) => {
        // Enclose the expansion in a block so that we can use multiple statements.
        {
            // Start a repetition:
            $(
                // each repeat will contain the following statement
                $d.1.replace(&format!("{{{}}}", stringify!($k)), &format!("{}", $v));
            )+
            $d
        }
    }
}
