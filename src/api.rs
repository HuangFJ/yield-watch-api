use utils::request_json;
use rocket::State;
use rocket::config::Config;

#[get("/")]
fn index(config: State<Config>) -> String {
    // .to_string: &str -> String
    // .as_str: String -> &str

    println!("{:?}", config.get_str("mysql"));

    let ret = request_json("https://api.coinmarketcap.com/v1/global/", 5).unwrap();

    format!("{:?}", ret)
}