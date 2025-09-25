use serde::{Deserialize, Serialize};

/// JSON Message format
#[derive(Serialize, Deserialize, Debug)]
struct Message {
    from: String,
    to: String,
    text: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let m1 = Message {
        from: "me".to_owned(),
        to: "you".to_owned(),
        text: "hi".to_owned(),
    };
    println!("{:?}", m1);

    // string
    let j = serde_json::to_string(&m1)?;
    println!("{}", j);
    let m2: Message = serde_json::from_str(&j)?;
    println!("{:?}", m2);

    // vec
    let j = serde_json::to_vec(&m1)?;
    println!("{:?}", j);
    let m2: Message = serde_json::from_slice(&j)?;
    println!("{:?}", m2);

    Ok(())
}
