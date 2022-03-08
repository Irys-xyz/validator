table! {
    bundle (id) {
        id -> Nullable<Text>,
        owner_address -> Nullable<Text>,
        block_height -> Integer,
    }
}

table! {
    leaders (address) {
        address -> Nullable<Text>,
    }
}

table! {
    transactions (id) {
        id -> Nullable<Text>,
        epoch -> Integer,
        block_promised -> Integer,
        block_actual -> Nullable<Integer>,
        signature -> Binary,
        validated -> Integer,
        bundle_id -> Nullable<Text>,
        sent_to_leader -> Integer,
    }
}

table! {
    validators (address) {
        address -> Nullable<Text>,
        url -> Nullable<Text>,
    }
}

joinable!(leaders -> validators (address));
joinable!(transactions -> bundle (bundle_id));

allow_tables_to_appear_in_same_query!(bundle, leaders, transactions, validators,);
