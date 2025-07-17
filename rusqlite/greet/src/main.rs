use rusqlite::{Connection, Result};

struct Greeting {
    id: i32,
    greeting: String,
}

fn insert(conn: &Connection, greeting: Greeting) -> Result<()> {
    conn.execute(
        "INSERT INTO greetings (greeting) VALUES (?1)",
        (&greeting.greeting,),
    )?;
    Ok(())
}

fn list(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT id, greeting FROM greetings")?;
    let greeting_iter = stmt.query_map([], |row| {
        Ok(Greeting {
            id: row.get(0)?,
            greeting: row.get(1)?,
        })
    })?;

    for greeting in greeting_iter {
        let g = greeting.unwrap();
        println!("list: {} {}", g.id, g.greeting);
    }
    Ok(())
}

fn get_id(conn: &Connection, id: i32) -> Result<()> {
    let mut stmt = conn.prepare("SELECT id, greeting FROM greetings WHERE id = ?1")?;
    let greeting_iter = stmt.query_map((id,), |row| {
        Ok(Greeting {
            id: row.get(0)?,
            greeting: row.get(1)?,
        })
    })?;

    for greeting in greeting_iter {
        let g = greeting.unwrap();
        println!("get_id {}: {} {}", id, g.id, g.greeting);
    }
    Ok(())
}

fn get_greeting(conn: &Connection, greeting: String) -> Result<()> {
    let mut stmt = conn.prepare("SELECT id, greeting FROM greetings WHERE greeting = ?1")?;
    let greeting_iter = stmt.query_map((&greeting,), |row| {
        Ok(Greeting {
            id: row.get(0)?,
            greeting: row.get(1)?,
        })
    })?;

    for g in greeting_iter {
        let g = g.unwrap();
        println!("get_greeting {}: {} {}", greeting, g.id, g.greeting);
    }
    Ok(())
}

fn delete_all(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM greetings", ())?;
    Ok(())
}

fn print_usage() {
    print! {"Usage:
  run                       run some test commands
  list                      list greetings
  id <id>                   get greeting by id
  greeting <greeting>       get greeting
"}
}

fn main() -> Result<()> {
    // open in memory db
    let conn = Connection::open_in_memory()?;

    // create table
    conn.execute(
        "CREATE TABLE greetings (
            id        INTEGER PRIMARY KEY,
            greeting  TEXT NOT NULL
        )",
        (),
    )?;

    // insert greetings
    let greetings = vec![
        Greeting {
            id: 0,
            greeting: "hello".to_string(),
        },
        Greeting {
            id: 0,
            greeting: "hi".to_string(),
        },
        Greeting {
            id: 0,
            greeting: "good day".to_string(),
        },
    ];
    for g in greetings {
        insert(&conn, g)?;
    }

    // command line arguments
    match std::env::args().nth(1).as_deref() {
        Some("run") => {
            // list
            list(&conn)?;

            // get id
            get_id(&conn, 1)?;

            // get greeting
            get_greeting(&conn, "hi".to_string())?;

            // delete all
            delete_all(&conn)?;

            // list
            list(&conn)?;
        }
        Some("list") => {
            list(&conn)?;
        }
        Some("id") => {
            if let Some(s) = std::env::args().nth(2)
                && let Ok(id) = s.parse::<i32>()
            {
                get_id(&conn, id)?;
            } else {
                print_usage()
            }
        }
        Some("greeting") => {
            if let Some(greeting) = std::env::args().nth(2) {
                get_greeting(&conn, greeting)?;
            } else {
                print_usage()
            }
        }
        _ => print_usage(),
    }

    Ok(())
}
