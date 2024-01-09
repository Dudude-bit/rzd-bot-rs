// @generated automatically by Diesel CLI.

diesel::table! {
    tasks (id) {
        id -> Nullable<Integer>,
        #[sql_name = "type"]
        type_ -> Text,
        data -> Binary,
    }
}
