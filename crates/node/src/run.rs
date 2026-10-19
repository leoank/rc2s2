//! `rc2s2 run`: Claude Code under a PTY this process owns, mirrored to the terminal it was
//! started from, so the daemon can interrupt or kill the session and learns when a person
//! types at it (EXP-0003). The wrapper never reads meaning from the terminal output; hooks
//! and the transcript stay the only sources of truth.
//!
//! Plain threads rather than async: every stream here is a blocking file descriptor.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use crate::wrapper::{FromWrapper, ToWrapper};

/// The interrupt sequence that EXP-0003 found to work: Escape stops the turn, Claude Code
/// then refills the input box with the interrupted prompt, Ctrl-U empties it, and a
/// follow-up prompt is typed and submitted. The pauses let the TUI settle between steps.
const AFTER_ESCAPE: Duration = Duration::from_secs(2);
const AFTER_CLEAR: Duration = Duration::from_secs(1);
const BEFORE_ENTER: Duration = Duration::from_secs(1);
/// Keystroke notices are rate-limited; one per burst of typing is enough.
const KEYSTROKE_EVERY: Duration = Duration::from_millis(250);
/// How long a killed process gets between SIGTERM and SIGKILL.
const KILL_GRACE: Duration = Duration::from_secs(5);

pub struct Options {
    pub command: String,
    pub args: Vec<String>,
    /// The daemon's wrapper socket.
    pub socket: PathBuf,
}

type Writer = Arc<Mutex<Box<dyn Write + Send>>>;

/// Runs the command under a PTY until it exits; returns its exit code.
pub fn run(opts: Options) -> Result<i32> {
    let pty = native_pty_system()
        .openpty(terminal_size())
        .context("opening a PTY")?;
    let mut cmd = CommandBuilder::new(&opts.command);
    cmd.args(&opts.args);
    cmd.cwd(std::env::current_dir()?);
    // A session started from inside another must not be taken for its child.
    for (name, _) in std::env::vars_os() {
        let name = name.to_string_lossy();
        if name.starts_with("CLAUDE") || name == "AI_AGENT" {
            cmd.env_remove(name.as_ref());
        }
    }
    let mut child = pty
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("starting {}", opts.command))?;
    drop(pty.slave);
    let pid = child.process_id().context("the child has no pid")?;
    let writer: Writer = Arc::new(Mutex::new(pty.master.take_writer()?));
    let mut reader = pty.master.try_clone_reader()?;
    let _raw = RawMode::enter();

    // The PTY's output goes to our terminal as it comes.
    std::thread::spawn(move || {
        let mut out = std::io::stdout();
        let mut buf = [0u8; 8192];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 || out.write_all(&buf[..n]).and_then(|()| out.flush()).is_err() {
                break;
            }
        }
    });

    let daemon = connect(&opts.socket, pid);
    if let Some(stream) = &daemon {
        let commands = stream.try_clone()?;
        let writer = writer.clone();
        std::thread::spawn(move || follow_commands(commands, writer, pid));
    }

    // Our terminal's input goes to the PTY; a person typing is reported to the daemon.
    {
        let writer = writer.clone();
        let mut notices = daemon;
        std::thread::spawn(move || {
            let mut input = std::io::stdin();
            let mut buf = [0u8; 4096];
            let mut last = Instant::now() - KEYSTROKE_EVERY;
            while let Ok(n) = input.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let typed = &buf[..n];
                if writer.lock().unwrap().write_all(typed).is_err() {
                    break;
                }
                if is_human(typed) && last.elapsed() >= KEYSTROKE_EVERY {
                    last = Instant::now();
                    if let Some(stream) = &mut notices
                        && send(stream, &FromWrapper::Keystroke).is_err()
                    {
                        notices = None;
                    }
                }
            }
        });
    }

    // Follow the terminal's size; polling avoids a signal handler for SIGWINCH.
    {
        let master = pty.master;
        std::thread::spawn(move || {
            let mut size = terminal_size();
            loop {
                std::thread::sleep(Duration::from_millis(250));
                let now = terminal_size();
                if (now.rows, now.cols) != (size.rows, size.cols) {
                    size = now;
                    let _ = master.resize(size);
                }
            }
        });
    }

    let status = child.wait()?;
    Ok(status.exit_code() as i32)
}

/// Says hello to the daemon. Without a daemon the session runs anyway, just unreachable.
fn connect(socket: &std::path::Path, pid: u32) -> Option<UnixStream> {
    let attempt = UnixStream::connect(socket).and_then(|mut s| {
        send(&mut s, &FromWrapper::Hello { pid })?;
        Ok(s)
    });
    match attempt {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!(
                "rc2s2 run: no node daemon at {} ({e}); this session cannot be interrupted remotely\r",
                socket.display()
            );
            None
        }
    }
}

fn send(stream: &mut UnixStream, msg: &FromWrapper) -> std::io::Result<()> {
    let mut line = serde_json::to_string(msg)?;
    line.push('\n');
    stream.write_all(line.as_bytes())
}

fn follow_commands(stream: UnixStream, writer: Writer, pid: u32) {
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { return };
        match serde_json::from_str::<ToWrapper>(&line) {
            Ok(ToWrapper::Interrupt { then_prompt }) => {
                let type_ = |bytes: &[u8]| {
                    let mut w = writer.lock().unwrap();
                    let _ = w.write_all(bytes).and_then(|()| w.flush());
                };
                type_(b"\x1b");
                std::thread::sleep(AFTER_ESCAPE);
                type_(b"\x15");
                if let Some(text) = then_prompt.filter(|t| !t.is_empty()) {
                    std::thread::sleep(AFTER_CLEAR);
                    type_(text.as_bytes());
                    std::thread::sleep(BEFORE_ENTER);
                    type_(b"\r");
                }
            }
            Ok(ToWrapper::Kill) => {
                let pid = pid as libc::pid_t;
                unsafe { libc::kill(pid, libc::SIGTERM) };
                std::thread::spawn(move || {
                    std::thread::sleep(KILL_GRACE);
                    if unsafe { libc::kill(pid, 0) } == 0 {
                        unsafe { libc::kill(pid, libc::SIGKILL) };
                    }
                });
            }
            Err(_) => {}
        }
    }
}

/// Whether terminal input holds anything a person typed. Terminals answer a program's
/// queries (cursor position, device attributes, version, mode reports) and announce focus
/// changes on the same input stream; those must not count as a keystroke, or every held
/// request would be released as soon as Claude Code redraws.
pub fn is_human(input: &[u8]) -> bool {
    let mut i = 0;
    while i < input.len() {
        if input[i] != 0x1b {
            return true;
        }
        let Some(&kind) = input.get(i + 1) else {
            return true; // the Escape key
        };
        match kind {
            b'[' => {
                let start = i + 2;
                let mut j = start;
                while j < input.len() && (0x20..=0x3f).contains(&input[j]) {
                    j += 1;
                }
                let Some(&last) = input.get(j) else {
                    return true;
                };
                let params = &input[start..j];
                let report = matches!(params.first(), Some(b'?' | b'>' | b'='))
                    || matches!(last, b'R' | b'n' | b't' | b'c')
                    || (matches!(last, b'I' | b'O') && params.is_empty());
                if !report {
                    return true; // arrows, function keys, pastes, mouse
                }
                i = j + 1;
            }
            // OSC, DCS, APC and PM strings run to BEL or ESC \ and are always replies.
            b']' | b'P' | b'_' | b'^' => {
                let mut j = i + 2;
                while j < input.len()
                    && input[j] != 0x07
                    && !(input[j] == 0x1b && input.get(j + 1) == Some(&b'\\'))
                {
                    j += 1;
                }
                i = if input.get(j) == Some(&0x07) {
                    j + 1
                } else {
                    j + 2
                };
            }
            _ => return true, // Alt with a key
        }
    }
    false
}

fn terminal_size() -> PtySize {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let ok = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) } == 0;
    if ok && ws.ws_row > 0 && ws.ws_col > 0 {
        PtySize {
            rows: ws.ws_row,
            cols: ws.ws_col,
            pixel_width: ws.ws_xpixel,
            pixel_height: ws.ws_ypixel,
        }
    } else {
        PtySize::default()
    }
}

/// The terminal in raw mode while the wrapper runs, restored on drop. Nothing to do when
/// stdin is not a terminal.
struct RawMode(Option<libc::termios>);

impl RawMode {
    fn enter() -> Self {
        let mut saved: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(libc::STDIN_FILENO, &mut saved) } != 0 {
            return Self(None);
        }
        let mut raw = saved;
        unsafe {
            libc::cfmakeraw(&mut raw);
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
        }
        Self(Some(saved))
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if let Some(saved) = &self.0 {
            unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, saved) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_human;

    #[test]
    fn typing_counts_and_terminal_replies_do_not() {
        for typed in [
            &b"y"[..],
            b"\r",
            b"\x1b",                 // Escape
            b"\x1b[A",               // arrow up
            b"\x1b[15~",             // F5
            b"\x1b[200~hi\x1b[201~", // a paste
            b"\x1b[<0;10;5M",        // a mouse click
            b"\x1bb",                // Alt-b
            b"\x1b[I\x1b[A",         // focus, then an arrow
            b"\x1b[97;5u",           // a key in the kitty keyboard protocol
        ] {
            assert!(is_human(typed), "{typed:?}");
        }
        for reply in [
            &b"\x1b[12;40R"[..],               // cursor position
            b"\x1b[?62;22c",                   // device attributes
            b"\x1b[>1;10;0c",                  // secondary device attributes
            b"\x1b[0n",                        // status
            b"\x1b[I",                         // focus in
            b"\x1b[O",                         // focus out
            b"\x1b[?1u",                       // kitty keyboard flags
            b"\x1b[?2026;2$y",                 // mode report
            b"\x1bP>|iTerm2 3.6\x1b\\",        // terminal version
            b"\x1b]11;rgb:1e1e/1e1e/1e1e\x07", // background colour
            b"\x1b[?62;22c\x1b[12;40R\x1b[I",  // several at once
        ] {
            assert!(!is_human(reply), "{reply:?}");
        }
    }
}
