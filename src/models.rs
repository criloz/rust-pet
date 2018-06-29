use super::schema::tasks;

use chrono::{ NaiveDateTime};

#[derive(Queryable, Debug, Identifiable)]
#[table_name="tasks"]
pub struct Task {
    pub id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub done_at: Option<NaiveDateTime>,
    pub done: bool,
}


#[derive(Insertable, Debug)]
#[table_name="tasks"]
pub struct NewTask<'a> {
    pub name: &'a str,
    pub created_at: NaiveDateTime,
}

#[derive(AsChangeset, Debug)]
#[table_name="tasks"]
pub struct UpdateTask<'a> {
    pub name: Option<&'a str>,
    pub done_at: Option<Option<NaiveDateTime>>,
    pub done: Option<bool>,
}