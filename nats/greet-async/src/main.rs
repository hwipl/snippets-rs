use futures_util::StreamExt;
use tokio::select;
use tokio::time::{self, Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), async_nats::Error> {
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
                client.publish("hi", "hi".into()).await?;

                // reset timer
                timer.as_mut().reset(Instant::now() + Duration::new(5,0));
            }

            // handle messages
            Some(message) = subscriber.next() => {
                println!("Received message {:?}", message);
            }
        }
    }
}
