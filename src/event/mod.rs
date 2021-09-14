mod io;

pub use io::IoEvent;

use std::{
    io::stdin,
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

use termion::{event::Key, input::TermRead};

pub enum Event<I> {
    Input(I),
    Tick,
}

pub fn poll(tick_rate: Duration) -> Receiver<Event<Key>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let std = stdin();
        for res in std.keys() {
            if let Ok(e) = res {
                tx.send(Event::Input(e)).unwrap();
            }
        }
    });

    let (tx1, rx1) = mpsc::channel();
    let _tx1 = tx1.clone();
    thread::spawn(move || loop {
        if let Ok(e) = rx.recv() {
            _tx1.send(e).unwrap();
        }
    });

    // another thread to generate tick events
    thread::spawn(move || loop {
        tx1.send(Event::Tick).unwrap();
        thread::sleep(tick_rate);
    });

    rx1
}

#[cfg(test)]
mod tests {
    use super::{poll, Event};
    use std::time::Duration;

    #[test]
    fn test_poll() {
        let evts = poll(Duration::from_millis(2000));
        while let Ok(e) = evts.recv() {
            match e {
                Event::Input(key) => {
                    println!("{:#?}", key);
                }
                Event::Tick => println!("tick!"),
            }
        }
    }
}
