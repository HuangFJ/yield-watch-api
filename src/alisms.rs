use time;
use std::collections::BTreeMap;
use uuid::Uuid;
use rustc_serialize::base64::{ToBase64, STANDARD};
use hmac_sha1::hmac_sha1;
use utils;

pub struct SmsBody {
    pub key_id: String,
    pub key_secret: String,
    pub sign_name: String,
    pub template_code: String,
    pub phone_numbers: String,
    pub template_param: String,
    pub out_id: String,
}

pub fn sms_api(sms: SmsBody) {
    let SmsBody {
        key_id,
        key_secret,
        sign_name,
        template_code,
        phone_numbers,
        template_param,
        out_id,
    } = sms;
    let tm = time::now_utc();
    let tm_string = time::strftime("%Y-%m-%dT%H:%M:%SZ", &tm).unwrap();

    let uuid_string = Uuid::new_v4().hyphenated().to_string();

    let mut params = BTreeMap::new();
    // system parameters
    params.insert("SignatureMethod", "HMAC-SHA1");
    params.insert("SignatureNonce", &uuid_string);
    params.insert("AccessKeyId", &key_id);
    params.insert("SignatureVersion", "1.0");
    params.insert("Timestamp", &tm_string);
    params.insert("Format", "JSON");

    // business parameters
    params.insert("Action", "SendSms");
    params.insert("Version", "2017-05-25");
    params.insert("RegionId", "cn-hangzhou");
    params.insert("PhoneNumbers", &phone_numbers);
    params.insert("SignName", &sign_name);
    params.insert("TemplateParam", &template_param);
    params.insert("TemplateCode", &template_code);
    if &out_id != "" {
        params.insert("OutId", &out_id);
    }

    if params.contains_key("Signature") {
        params.remove("Signature");
    }

    let mut query_string = String::new();

    for (&k, &v) in params.iter() {
        query_string.push_str(&format!(
            "{}={}&",
            utils::query_quote(k),
            utils::query_quote(v)
        ));
    }
    query_string.pop();

    let sig = sign(&format!("{}&", key_secret), &query_string);
    let signature = utils::query_quote(&sig);

    let url = &format!(
        "http://dysmsapi.aliyuncs.com/?Signature={signature}&{query_string}",
        signature = signature,
        query_string = query_string
    );

    let ret = utils::request_json(url, None).unwrap();
    println!("{:?}", ret);
}

fn sign(secret: &str, query_string: &str) -> String {
    let mut string_to_sign = String::new();
    string_to_sign.push_str("GET&");
    string_to_sign.push_str(&utils::query_quote("/"));
    string_to_sign.push_str("&");
    string_to_sign.push_str(&utils::query_quote(&query_string));

    let hash = hmac_sha1(secret.as_bytes(), string_to_sign.as_bytes());
    hash.to_base64(STANDARD)
}
