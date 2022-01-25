table! {
    leaders (address) {
        address -> Bpchar,
    }
}

table! {
    transactions (id) {
        id -> Bpchar,
        epoch -> Int8,
        block_promised -> Int8,
        block_actual -> Nullable<Int8>,
        signature -> Bytea,
        validated -> Bool,
    }
}

table! {
    validators (address) {
        address -> Bpchar,
        url -> Nullable<Varchar>,
    }
}

joinable!(leaders -> validators (address));

allow_tables_to_appear_in_same_query!(
    leaders,
    transactions,
    validators,
);
