table! {
    tasks (id) {
        id -> Int4,
        name -> Varchar,
        created_at -> Timestamp,
        done_at -> Nullable<Timestamp>,
        done -> Bool,
    }
}
