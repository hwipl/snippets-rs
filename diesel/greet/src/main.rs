use diesel::prelude::*;
use greet::*;

fn print_usage() {
    print! {"Usage:
  run                   run some test commands
  list                  list greetings
  id <id>               get greeting by id
  text <greeting>       get greeting by text
"}
}

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

    // command line arguments
    match std::env::args().nth(1).as_deref() {
        Some("run") => {
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
        }
        Some("list") => {
            list_greetings(connection);
        }
        Some("id") => {
            if let Some(s) = std::env::args().nth(2)
                && let Ok(id) = s.parse::<i32>()
            {
                get_id(connection, id);
            } else {
                print_usage()
            }
        }
        Some("text") => {
            if let Some(text) = std::env::args().nth(2) {
                get_text(connection, &text);
            } else {
                print_usage()
            }
        }
        _ => print_usage(),
    }

    Ok(())
}
