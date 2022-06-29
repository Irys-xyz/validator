use diesel::table;

table! {
    transactions (id) {
        id -> Text,
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
