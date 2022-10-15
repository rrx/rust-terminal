use terminal::signals;
use terminal::tui;
use terminal::term;
use std::error::Error;
use std::sync::mpsc::channel;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let (tx, rx) = channel();

    let mut terms = term::TerminalList::new();

    let t = term::ManagedTerminal::start(tx.clone(), "top".into(), vec!["-b".into()])?;
    terms.add(t);

    let t = term::ManagedTerminal::start(tx.clone(), "zsh".into(), vec![])?;
    terms.add(t);

    let signal_tx = tx.clone();
    let _signal_thread = std::thread::spawn(move || {
        signals::signal_thread(signal_tx);
    });

    let _input_thread = std::thread::spawn(move || {
        tui::input_thread(tx);
    });

    std::thread::scope(|s| {
        // start display thread
        s.spawn(|| {
            tui::display(terms, rx).expect("Display error");
            log::info!("display thread exit");
        });
    });

    Ok(())
}
