use alacritty_terminal::tty;
use alacritty_terminal::event_loop::{EventLoop as PtyEventLoop, Msg, Notifier};
use alacritty_terminal::config::{Config, PtyConfig};
use alacritty_terminal::event::{VoidListener, WindowSize, EventListener, Event};
use alacritty_terminal::term::Term;
use alacritty_terminal::term::test::TermSize;
use std::sync::Arc;
use alacritty_terminal::sync::FairMutex;
use std::io;
use std::ffi::OsStr;
use std::process::{Command, Stdio};
use {
    std::error::Error,
    std::os::unix::process::CommandExt,
    std::os::unix::io::{AsRawFd, RawFd},
    std::path::PathBuf,
};
use libc::pid_t;
use std::fs;
use signal_hook::{consts::SIGINT, iterator::Signals};
use std::borrow::Cow;

#[derive(Clone)]
struct MyEventListener {}

impl MyEventListener {
    pub fn new() -> Self {
        Self {}
    }
}

impl EventListener for MyEventListener {
    fn send_event(&self, event: Event) {
        log::info!("Event: {:?}", event);
    }
}

/// Get working directory of controlling process.
pub fn foreground_process_path(
    master_fd: RawFd,
    shell_pid: u32,
) -> Result<PathBuf, Box<dyn Error>> {
    let mut pid = unsafe { libc::tcgetpgrp(master_fd) };
    if pid < 0 {
        pid = shell_pid as pid_t;
    }

    #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
    let link_path = format!("/proc/{}/cwd", pid);
    #[cfg(target_os = "freebsd")]
    let link_path = format!("/compat/linux/proc/{}/cwd", pid);

    #[cfg(not(target_os = "macos"))]
    let cwd = fs::read_link(link_path)?;

    #[cfg(target_os = "macos")]
    let cwd = macos::proc::cwd(pid)?;

    Ok(cwd)
}
/// Start a new process in the background.
pub fn spawn_daemon<I, S>(
    program: &str,
    args: I,
    master_fd: RawFd,
    shell_pid: u32,
) -> io::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Ok(cwd) = foreground_process_path(master_fd, shell_pid) {
        command.current_dir(cwd);
    }
    unsafe {
        command
            /*
            .pre_exec(|| {
                match libc::fork() {
                    -1 => return Err(io::Error::last_os_error()),
                    0 => (),
                    _ => libc::_exit(0),
                }

                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }

                Ok(())
            })
            */
            .spawn()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let pty_config = PtyConfig::new();
    let config = Config::default();

    let window_size = WindowSize { num_lines: 10, num_cols: 10, cell_width: 1, cell_height: 1 };
    let term_size = TermSize::new(10, 10);


    // create terminal
    let event_listener = MyEventListener::new();
    let terminal = Term::new(&config, &term_size, event_listener.clone());
    let terminal = Arc::new(FairMutex::new(terminal));

    let pty = tty::new(&pty_config, window_size, 0)?;
    let master_fd = pty.file().as_raw_fd();
    let shell_pid = pty.child().id();
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
    let notifier = Notifier(loop_tx);


    //std::thread::spawn(move || {
    //});

    //
    // Kick off the I/O thread.
    let io_thread = event_loop.spawn();

    // send stuff to the shell
    //let _ = notifier.0.send(Msg::Input(Cow::from("/usr/bin/ls -la\n".as_bytes())));

    // start a signal handler
    let mut signals = Signals::new(&[SIGINT])?;
    let signal_thread = std::thread::spawn(move || {
        for sig in signals.forever() {
            log::info!("Received signal {:?}", sig);
            break;
        }
        log::info!("shutdown");
        let _ = notifier.0.send(Msg::Shutdown);
    });



    /*
    let program = "top";
    let args = vec!["-b"];
    let result = spawn_daemon(program, args.clone(), master_fd, shell_pid);
    log::info!("result: {:?}", &result);
    match result {
        Ok(mut child) => {
            log::debug!("Launched {} with args {:?}", program, &args);
            log::info!("waiting for child to exit");
            let exit_status = child.wait().expect("command wasn't running");
            log::info!("child complete: {:?}", exit_status);
            match exit_status.code() {
                Some(code) => log::info!("Exited with status code: {code}"),
                None       => log::info!("Process terminated by signal")
            }
        }
        Err(e) => {
            log::warn!("Unable to launch {} with args {:?}, Error: {:?}", program, &args, e);
        }
    };
    */

    log::info!("waiting");
    //let _ = notifier.0.send(Msg::Shutdown);
    io_thread.join().expect("Thread panicked");
    signal_thread.join().expect("Signal thread error"); 

    //drop(event_loop);
    //
    //log::info!("{}", terminal.lock().renderable_content());

    for x in terminal.lock().grid().display_iter() {
        log::info!("term: {:?}", x);
    }

    //log::info!("term: {:?}", terminal.lock().grid().display_iter());

    Ok(())
}
