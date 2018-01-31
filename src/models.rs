use alisms;

pub struct SmsNotify {
    key_id: String,
    key_secret: String,
    phone_numbers: String,
    template_param: String,
    out_id: String,
}

impl SmsNotify {
    pub fn new(key_id: &str, key_secret: &str) -> Self {
        SmsNotify {
            key_id: key_id.to_string(),
            key_secret: key_secret.to_string(),
            phone_numbers: "".to_string(),
            template_param: "".to_string(),
            out_id: "".to_string(),
        }
    }

    pub fn send(&mut self, phone_numbers: &str, code: &str) {
        self.phone_numbers = phone_numbers.to_string();
        self.template_param = format!("{{\"code\":\"{code}\"}}", code = code);
        alisms::sms_api(self);
    }
}

impl alisms::Sms for SmsNotify {
    fn ready(&self) -> (&str, &str, &str, &str, &str, &str, &str) {
        (
            &self.key_id,
            &self.key_secret,
            "yield助手",
            "SMS_123673246",
            &self.phone_numbers,
            &self.template_param,
            &self.out_id,
        )
    }
}
