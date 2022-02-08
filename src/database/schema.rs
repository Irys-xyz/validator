table! {
    bundle (id) {
        id -> Bpchar,
        owner_address -> Nullable<Bpchar>,
        block_height -> Int8,
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
        epoch -> Int8,
        block_promised -> Int8,
        block_actual -> Nullable<Int8>,
        signature -> Bytea,
        validated -> Bool,
        bundle_id -> Nullable<Bpchar>,
        sent_to_leader -> Bool,
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

allow_tables_to_appear_in_same_query!(bundle, leaders, transactions, validators,);
