pub mod models;
pub mod schema;

use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

use self::models::{Greeting, NewGreeting};

pub fn create_greeting(conn: &mut SqliteConnection, greeting: &str) -> Greeting {
    use crate::schema::greetings;

    let new_greeting = NewGreeting { greeting };

    diesel::insert_into(greetings::table)
        .values(&new_greeting)
        .returning(Greeting::as_returning())
        .get_result(conn)
        .expect("Error inserting new greeting")
}

pub fn list_greetings(conn: &mut SqliteConnection) {
    use crate::schema::greetings::dsl::greetings;

    let results = greetings
        .limit(5)
        .select(Greeting::as_select())
        .load(conn)
        .expect("Error loading posts");
    println!("Listing {} greetings:", results.len());
    for greeting in results {
        println!("  {} {}", greeting.id, greeting.greeting);
    }
}

pub fn get_id(conn: &mut SqliteConnection, id: i32) {
    use crate::schema::greetings::dsl::greetings;

    if let Ok(Some(greeting)) = greetings
        .find(id)
        .select(Greeting::as_select())
        .first(conn)
        .optional()
    {
        println!("Got greeting with ID {id}:");
        println!("  {} {}", greeting.id, greeting.greeting);
    }
}

pub fn get_text(conn: &mut SqliteConnection, greeting_text: &str) {
    use crate::schema::greetings::dsl::greeting;
    use crate::schema::greetings::dsl::greetings;

    if let Ok(Some(g)) = greetings
        .filter(greeting.eq(greeting_text))
        .select(Greeting::as_select())
        .first(conn)
        .optional()
    {
        println!("Got greeting with text \"{greeting_text}\":");
        println!("  {} {}", g.id, g.greeting);
    }
}

pub fn delete_id(conn: &mut SqliteConnection, greeting_id: i32) {
    use crate::schema::greetings::dsl::*;

    let num_deleted = diesel::delete(greetings.filter(id.eq(greeting_id)))
        .execute(conn)
        .expect("Error deleting greeting");
    println!("Deleted {num_deleted} greeting with ID {greeting_id}");
}
