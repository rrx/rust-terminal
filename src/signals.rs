use crate::app::Command;
use alacritty_terminal::event::Event;
use signal_hook::consts::signal::*;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::iterator::Signals;
use std::sync::mpsc::Sender;

pub fn signal_thread(tx: Sender<Command>) {
    use signal_hook::consts::signal::*;
    let mut sigs = vec![SIGCONT, SIGWINCH, SIGHUP, SIGUSR1];
    sigs.extend(TERM_SIGNALS);
    let mut signals = signal_hook::iterator::Signals::new(&sigs).unwrap();

    for info in &mut signals {
        log::info!("Received a signal {:?}", info);
        match info {
            SIGCONT => {
                tx.send(Command::Resume).unwrap();
            }

            //SIGWINCH => {
            //tx.send(Command::TerminalEvent().unwrap();
            //}
            SIGHUP => {
                log::info!("SIGHUP");
                break;
            }
            SIGUSR1 => {
                log::info!("SIGUSR1");
                break;
            }

            SIGINT => {
                log::info!("SIGINT");
                tx.send(Command::TerminalEvent(Event::Exit)).unwrap();
                break;
            }

            // panic
            SIGALRM => {
                log::info!("ALARM");
                tx.send(Command::TerminalEvent(Event::Exit)).unwrap();
                break;
            }

            _ => {
                log::info!("other sig {}", info);
                break;
            }
        }
    }

    log::info!("signals thread exit");
}
