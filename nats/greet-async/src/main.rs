use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::select;
use tokio::time::{self, Duration, Instant};

/// Greeting message format
#[derive(Serialize, Deserialize, Debug)]
struct Greeting {
    from: String,
    to: String,
    text: String,
}

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
    // get name from command line arguments
    let name = std::env::args().nth(1).unwrap_or("client".to_string());

    // connect to server
    let client = async_nats::connect("nats://localhost:4222").await?;

    // subscribe to subject
    let mut subscriber = client.subscribe("hi").await?;

    // timer
    let timer = time::sleep(Duration::new(5, 0));
    tokio::pin!(timer);

    loop {
        select! {
            // handle timer events
            _ = &mut timer => {
                // publish message
                let g = Greeting {
                    from: name.clone(),
                    to: "".to_owned(),
                    text: "hi".to_owned(),
                };
                let j = serde_json::to_string(&g)?;
                client.publish("hi", j.into()).await?;

                // reset timer
                timer.as_mut().reset(Instant::now() + Duration::new(5,0));
            }

            // handle messages
            Some(message) = subscriber.next() => {
                let g: Greeting = serde_json::from_slice(&message.payload)?;
                println!("Received message {:?}", g);

                // reply
                if g.from != name && g.to == "" {
                    let r = Greeting {
                        from: name.clone(),
                        to: g.from,
                        text: "hi".to_owned(),
                    };
                    let j = serde_json::to_string(&r)?;
                    client.publish("hi", j.into()).await?;
                }
            }
        }
    }
}
