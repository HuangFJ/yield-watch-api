use utils::request_json;

#[get("/")]
fn index() -> String {
    // .to_string: &str -> String
    // .as_str: String -> &str

    let ret = request_json("https://api.coinmarketcap.com/v1/global/", 5).unwrap();

    format!("{:?}", ret)
}