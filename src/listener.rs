use crate::app::Command;
use alacritty_terminal::event::{Event, EventListener};
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct MyEventListener {
    tx: Sender<Command>,
}

impl MyEventListener {
    pub fn new(tx: Sender<Command>) -> Self {
        Self { tx }
    }
}

impl EventListener for MyEventListener {
    fn send_event(&self, event: Event) {
        let _ = self.tx.send(Command::TerminalEvent(event));
    }
}
