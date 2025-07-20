use diesel::prelude::*;
use greet::*;

fn main() -> diesel::QueryResult<()> {
    use greet::schema::greetings::dsl::greetings;

    let connection = &mut establish_connection();

    // delete existing greetings
    diesel::delete(greetings).execute(connection)?;

    // create greetings
    let mut last_id = 0;
    for g in vec!["hello", "hi", "good day", "greetings"] {
        let greeting = create_greeting(connection, g);
        println!("Created greeting {} {}", greeting.id, greeting.greeting);
        last_id = greeting.id;
    }

    // list greetings
    list_greetings(connection);

    // get greeting by id
    get_id(connection, last_id);

    // get greeting by text
    get_text(connection, "good day");

    // delete greeting by id
    delete_id(connection, last_id);

    // list greetings
    list_greetings(connection);

    Ok(())
}
