use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
  let (sender, receiver) = channel();

  let receiver = Arc::new(Mutex::new(receiver));

  //TODO test that when receiver is wrapped in arc

  sender.send("Whoop whoop").expect("send error");

  thread::spawn(move || {
    let received = receiver.lock().unwrap().recv().expect("recv error");

    println!("received: {}", received);
  })
  .join()
  .expect("join error");
}
