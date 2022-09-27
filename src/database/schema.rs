// @generated automatically by Diesel CLI.

diesel::table! {
    bundle (id) {
        id -> Bpchar,
        owner_address -> Bpchar,
        block_height -> Bytea,
    }
}

diesel::table! {
    leaders (address) {
        address -> Bpchar,
    }
}

diesel::table! {
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

diesel::table! {
    validators (address) {
        address -> Bpchar,
        url -> Nullable<Varchar>,
    }
}

diesel::joinable!(leaders -> validators (address));
diesel::joinable!(transactions -> bundle (bundle_id));

diesel::allow_tables_to_appear_in_same_query!(
    bundle,
    leaders,
    transactions,
    validators,
);
