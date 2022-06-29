use diesel::table;

table! {
    transactions (id, bundler) {
        id -> Text,
        bundler -> Text,
        epoch -> BigInt,
        block_promised -> BigInt,
        block_actual -> Nullable<BigInt>,
        signature -> Binary,
        validated -> Bool,
    }
}

table! {
    validators (address) {
        address -> Text,
        url -> Text,
    }
}

table! {
    leaders (address) {
        address -> Text,
    }
}