use alacritty_terminal::config::{Config, PtyConfig};
use alacritty_terminal::event::{Event, WindowSize};
use alacritty_terminal::event_loop::{EventLoop as PtyEventLoop, Msg, Notifier};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::Term;
use alacritty_terminal::tty;
use pty_test::app::Command;
use pty_test::listener::MyEventListener;
use pty_test::signals;
use pty_test::tui;
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::sync::mpsc::channel;
use std::sync::Arc;
use {std::error::Error, std::os::unix::io::AsRawFd};

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let pty_config = PtyConfig::new();
    let config = Config::default();

    let term_columns = 40usize;
    let term_rows = 10usize;
    let window_size = WindowSize {
        num_lines: term_rows as u16,
        num_cols: term_columns as u16,
        cell_width: 1,
        cell_height: 1,
    };
    let term_size = TermSize::new(term_columns, term_rows);

    let (tx, rx) = channel();

    // create terminal
    let event_listener = MyEventListener::new(tx.clone());
    let terminal = Term::new(&config, &term_size, event_listener.clone());
    let terminal = Arc::new(FairMutex::new(terminal));

    let pty = tty::new(&pty_config, window_size, 0)?;
    let _master_fd = pty.file().as_raw_fd();
    let _shell_pid = pty.child().id();
    let event_loop = PtyEventLoop::new(
        Arc::clone(&terminal),
        event_listener,
        pty,
        pty_config.hold,
        false,
    );

    // The event loop channel allows write requests from the event processor
    // to be sent to the pty loop and ultimately written to the pty.
    let loop_tx = event_loop.channel();
    let notifier = Notifier(loop_tx.clone());
    let input_notifier = Notifier(loop_tx);

    // Kick off the I/O thread.
    let io_thread = event_loop.spawn();

    let signal_tx = tx.clone();
    let signal_thread = std::thread::spawn(move || {
        signals::signal_thread(signal_tx);
    });

    let input_thread = std::thread::spawn(move || {
        tui::input_thread(tx);
    });

    std::thread::scope(|s| {
        // start display thread
        s.spawn(|| {
            tui::display(Arc::clone(&terminal), rx).expect("Display error");
            log::info!("display thread exit");
        });
    });

    Ok(())
}
