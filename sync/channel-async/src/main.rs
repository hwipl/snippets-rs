use async_std::{prelude::*, task};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use std::time::Duration;

type Sender<T> = mpsc::UnboundedSender<T>;

/// run task that periodically writes num to sender
async fn run_task(mut sender: Sender<u8>, num: u8) {
    loop {
        task::sleep(Duration::from_secs(1)).await;
        if let Err(e) = sender.send(num).await {
            eprintln!("error sending to channel: {}", e);
            return;
        }
    }
}

fn main() {
    let (sender, mut receiver) = mpsc::unbounded();

    // run tasks
    for num in 0..=5 {
        task::spawn(run_task(sender.clone(), num));
    }

    // read from tasks
    task::block_on(async {
        while let Some(num) = receiver.next().await {
            println!("{}", num);
        }
    });
}
