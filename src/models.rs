#[derive(Queryable, Debug)]
pub struct Coin {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: i32,
    pub available_supply: i32,
    pub total_supply: i32,
    pub max_supply: i32,
    pub last_updated: i32,
}