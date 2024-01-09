use std::collections::HashMap;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Post {
    pub(crate) id: Option<isize>,
    pub(crate) _type: String,
    pub(crate) data: HashMap<String, String>
}