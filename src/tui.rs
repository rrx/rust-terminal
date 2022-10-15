use crate::app::Command;
use crate::term::*;
use crate::listener::MyEventListener;
use alacritty_terminal::event::Event as TerminalEvent;
use alacritty_terminal::event::WindowSize;
use alacritty_terminal::event_loop::Msg;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
pub use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue, style,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::error::Error;
use std::io::Write;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub fn input_event_to_command() -> Result<Vec<Command>, Box<dyn Error>> {
    let commands = match event::read()? {
        Event::Resize(width, height) => vec![Command::Msg(Msg::Resize(WindowSize {
            num_lines: height,
            num_cols: width,
            cell_width: 1,
            cell_height: 1,
        }))],
        Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::CONTROL,
            kind: _,
            state: _,
        }) => vec![Command::Exit],
        Event::Key(KeyEvent {
            code: KeyCode::Char('z'),
            modifiers: KeyModifiers::CONTROL,
            kind: _,
            state: _,
        }) => vec![Command::Suspend],
        Event::Key(KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::CONTROL,
            kind: _,
            state: _,
        }) => vec![Command::NextWindow, Command::TerminalEvent(TerminalEvent::Wakeup)],
        Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::CONTROL,
            kind: _,
            state: _,
        }) => vec![Command::PrevWindow, Command::TerminalEvent(TerminalEvent::Wakeup)],
        _ => vec![],
    };
    Ok(commands)
}

fn dump_term<W>(
    terminal: &Arc<FairMutex<Term<MyEventListener>>>,
    w: &mut W,
) -> Result<(), Box<dyn Error>>
where
    W: Write,
{
    let t = terminal.lock();
    let grid = t.grid();
    let columns = grid.columns();
    let rows = grid.screen_lines();
    let content = t.renderable_content();

    let length = columns * rows;
    let mut array: Vec<char> = vec![" ".chars().next().unwrap(); length];

    for x in content.display_iter {
        let p = x.point;
        let i = p.line.0 as usize * columns + p.column.0;
        array[i as usize] = x.cell.c
    }
    drop(t);
    for (line_no, chunk) in array.chunks(columns).enumerate() {
        let line = chunk.into_iter().collect::<String>();
        execute!(w, cursor::MoveTo(0, line_no as u16), Print(line))?;
    }
    Ok(())
}

pub fn display(
    mut terms: TerminalList<ManagedTerminal>,
    rx: Receiver<Command>,
) -> Result<(), Box<dyn Error>> {
    // handle panic, by cleaning up the terminal
    use std::panic;
    panic::set_hook(Box::new(move |w| {
        let mut stdout = std::io::stdout();
        let _ = exit(&mut stdout);
        log::info!("Custom panic hook: {:?}", w);
        log::info!("{:?}", backtrace::Backtrace::new());
    }));

    let mut stdout = std::io::stdout();
    enter(&mut stdout)?;
    let mut has_terminal = true;

    let mut terminal = terms.get().terminal.clone();

    loop {
        let result = rx.recv();
        match result {
            Ok(Command::Exit) => {
                log::info!("rx exit");
                break;
            }
            Ok(Command::TerminalEvent(TerminalEvent::Exit)) => {
                log::info!("rx exit");
                break;
            }
            Ok(Command::TerminalEvent(TerminalEvent::Wakeup)) => {
                log::info!("rx wakeup");
                dump_term(&terminal, &mut stdout)?;
            }
            Ok(Command::Suspend) => {
                log::info!("suspend");
                exit(&mut stdout)?;
                has_terminal = false;
                signal_hook::low_level::raise(signal_hook::consts::signal::SIGTSTP).unwrap();
            }
            Ok(Command::Resume) => {
                enter(&mut stdout)?;
                has_terminal = true;
            }
            Ok(Command::Toggle) => {
                if has_terminal {
                    exit(&mut stdout)?;
                } else {
                    enter(&mut stdout)?;
                }
                has_terminal = !has_terminal;
            }
    
            Ok(Command::NextWindow) => {
                terms.next();
                terminal = terms.get().terminal.clone();
            }

            Ok(Command::PrevWindow) => {
                terms.prev();
                terminal = terms.get().terminal.clone();
            }

            Ok(event) => {
                log::info!("rx event: {:?}", event);
            }
            Err(_) => {
                log::info!("error on receive, exiting");
                break;
            }
        }
    }
    exit(&mut stdout)?;
    Ok(())
}

pub fn input_thread(tx: Sender<Command>) {
    let mut done = false;
    loop {
        match input_event_to_command() {
            Ok(commands) => {
                for command in commands {
                    match command {
                        Command::Exit => {
                            log::info!("Shutdown");
                            done = true;
                        }
                        _ => {
                            let _ = tx.send(command);
                        }
                    };
                }
            }
            Err(e) => {
                log::error!("Error: {}", e);
                done = true;
            }
        }

        if done {
            break;
        }
    }
    let _ = tx.send(Command::Exit);
    log::info!("input thread exit");
}

pub fn enter<W>(w: &mut W) -> Result<(), Box<dyn Error>>
where
    W: Write,
{
    execute!(w, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    Ok(())
}

pub fn exit<W>(_w: &mut W) -> Result<(), Box<dyn Error>>
where
    W: Write,
{
    terminal::disable_raw_mode()?;
    Ok(())
}
