table! {
    coins (id) {
        id -> Varchar,
        name -> Varchar,
        symbol -> Varchar,
        rank -> Smallint,
        available_supply -> Bigint,
        total_supply -> Bigint,
        max_supply -> Nullable<Bigint>,
        last_updated -> Integer,
    }
}

table! {
    prices (coin_id, created) {
        coin_id -> Varchar,
        price_usd -> Decimal,
        volume_usd -> Decimal,
        price_cny -> Decimal,
        price_btc -> Decimal,
        price_platform -> Nullable<Decimal>,
        created -> Integer,
    }
}

table! {
    states (id) {
        id -> Integer,
        user_id -> Integer,
        coin_id -> Varchar,
        amount -> Decimal,
        created -> Integer,
    }
}

table! {
    users (id) {
        id -> Integer,
        name -> Varchar,
    }
}

allow_tables_to_appear_in_same_query!(
    coins,
    prices,
    states,
    users,
);
