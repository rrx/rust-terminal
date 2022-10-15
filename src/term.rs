use crate::app::Command;
use crate::listener::MyEventListener;
use alacritty_terminal::config::{Config, PtyConfig, Program};
use alacritty_terminal::event::{self, WindowSize};
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::event_loop::{EventLoop as PtyEventLoop, Notifier};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::Term;
use alacritty_terminal::tty;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use {std::error::Error, std::os::unix::io::AsRawFd};
use std::collections::VecDeque;

pub struct ManagedTerminal {
    //pub event_loop: PtyEventLoop<tty::Pty, MyEventListener>,
    pub terminal: Arc<FairMutex<Term<MyEventListener>>>,
    pub notifier: Notifier
}

pub struct TerminalList<T> {
    elements: VecDeque<T>
}

impl<T> TerminalList<T> {
    pub fn new() -> Self {
        Self {
            elements: VecDeque::new()
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.elements.iter_mut().next().unwrap()
    }

    pub fn get(&mut self) -> &T {
        self.elements.iter().next().unwrap()
    }

    pub fn add(&mut self, b: T) -> &mut Self {
        self.elements.push_front(b);
        self
    }

    pub fn next(&mut self) -> &mut Self {
        if let Some(b) = self.elements.pop_front() {
            self.elements.push_back(b);
        }
        self
    }

    pub fn prev(&mut self) -> &mut Self {
        if let Some(b) = self.elements.pop_back() {
            self.elements.push_front(b);
        }
        self
    }
}

impl ManagedTerminal {
    pub fn start(tx: Sender<Command>, program: String, args: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut config = Config::default();
        config.pty_config.shell = Some(Program::WithArgs {program, args});

        let term_columns = 40usize;
        let term_rows = 10usize;

        let window_size = WindowSize {
            num_lines: term_rows as u16,
            num_cols: term_columns as u16,
            cell_width: 1,
            cell_height: 1,
        };
        let term_size = TermSize::new(term_columns, term_rows);

        // create terminal
        let event_listener = MyEventListener::new(tx.clone());
        let terminal = Term::new(&config, &term_size, event_listener.clone());
        let terminal = Arc::new(FairMutex::new(terminal));

        let pty = tty::new(&config.pty_config, window_size, 0)?;
        let _master_fd = pty.file().as_raw_fd();
        let _shell_pid = pty.child().id();

        // We have one event loop for each terminal
        // Eventually we can consolidate this into a single event loop, but alacrity
        // assumes 1:1
        let event_loop = PtyEventLoop::new(
            Arc::clone(&terminal),
            event_listener,
            pty,
            config.pty_config.hold,
            false,
        );

        // The event loop channel allows write requests from the event processor
        // to be sent to the pty loop and ultimately written to the pty.
        let loop_tx = event_loop.channel();

        event_loop.spawn();

        let notifier = Notifier(loop_tx);
        Ok(Self {
            //event_loop,
            terminal,
            notifier
        })
    }

    pub fn spawn(&mut self) {
        //self.event_loop.spawn();
    }
}
