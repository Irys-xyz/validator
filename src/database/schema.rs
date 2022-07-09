table! {
    bundle (id) {
        id -> Bpchar,
        owner_address -> Bpchar,
        block_height -> Bytea,
    }
}

table! {
    leaders (address) {
        address -> Bpchar,
    }
}

table! {
    transactions (id) {
        id -> Bpchar,
        epoch -> Bytea,
        block_promised -> Bytea,
        block_actual -> Nullable<Bytea>,
        signature -> Bytea,
        validated -> Bool,
        bundle_id -> Nullable<Bpchar>,
    }
}

table! {
    validators (address) {
        address -> Bpchar,
        url -> Nullable<Varchar>,
    }
}

joinable!(leaders -> validators (address));
joinable!(transactions -> bundle (bundle_id));

allow_tables_to_appear_in_same_query!(
    bundle,
    leaders,
    transactions,
    validators,
);
