use alacritty_terminal::event::Event as TerminalEvent;
use alacritty_terminal::event_loop::Msg;

#[derive(Debug)]
pub enum Command {
    TerminalEvent(TerminalEvent),
    Msg(Msg),
    Suspend,
    Resume,
    Toggle,
    Exit,
    NextWindow,
    PrevWindow
}
