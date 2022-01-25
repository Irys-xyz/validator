table! {
    leaders (address) {
        address -> Bpchar,
    }
}

table! {
    transactions (id) {
        id -> Bpchar,
        epoch -> Nullable<Int8>,
        block_promised -> Nullable<Int8>,
        block_actual -> Nullable<Int8>,
        signature -> Nullable<Bytea>,
        validated -> Nullable<Bool>,
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
