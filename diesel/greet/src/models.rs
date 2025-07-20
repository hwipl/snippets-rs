use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::greetings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Greeting {
    pub id: i32,
    pub greeting: String,
}

use crate::schema::greetings;

#[derive(Insertable)]
#[diesel(table_name = greetings)]
pub struct NewGreeting<'a> {
    pub greeting: &'a str,
}
