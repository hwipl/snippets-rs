use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

/// run thread that periodically writes num to sender
fn run_thread(sender: Sender<u8>, num: u8) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        sender.send(num).unwrap();
    });
}

fn main() {
    let (sender, receiver) = channel();

    // run threads
    for num in 0..=5 {
        run_thread(sender.clone(), num);
    }

    // read from threads
    while let Ok(num) = receiver.recv() {
        println!("{}", num);
    }
}
