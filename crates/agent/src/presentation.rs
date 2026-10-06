//! CLI presentation for the agent binaries: an animated startup banner and
//! live status lines. Presentation only — no behavior changes. Every output
//! path falls back to plain single-line text on a non-TTY stream, `NO_COLOR`,
//! `CLICOLOR=0` or a CI environment. Status lines write to stderr so stdout
//! stays a clean contract (JSON reports, MCP stdio protocol, host terminals).

use std::io::{IsTerminal, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Wordmark reveal frames. Hand-authored pure ASCII; each frame extends the
/// previous one and the last spells the full wordmark.
pub const BANNER_FRAMES: &[&str] = &[
    "r_",
    "rec__",
    "recol___",
    "recollec____",
    "recollect________",
];
pub const TAGLINE: &str = "Recollect · a place for what you know";
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
pub(crate) const FRAME_DELAY: Duration = Duration::from_millis(80);

/// Output mode for one stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Interactive color terminal: ANSI output and animation.
    Ansi,
    /// Not a TTY, or color disabled: plain text, no animation.
    Plain,
    /// Machine-owned stream: no presentation output at all.
    Quiet,
}

/// Environment facts that select the presentation mode for one stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvFlags {
    pub tty: bool,
    pub no_color: bool,
    pub clicolor_off: bool,
    pub ci: bool,
}

impl EnvFlags {
    pub fn from_env(tty: bool) -> Self {
        Self {
            tty,
            no_color: std::env::var_os("NO_COLOR").is_some(),
            clicolor_off: std::env::var("CLICOLOR").ok().as_deref() == Some("0"),
            ci: std::env::var_os("CI").is_some(),
        }
    }
    /// Plain fallback when the stream is not an interactive color terminal.
    pub fn plain(&self) -> bool {
        !self.tty || self.no_color || self.clicolor_off || self.ci
    }
    pub fn mode(&self) -> Mode {
        if self.plain() {
            Mode::Plain
        } else {
            Mode::Ansi
        }
    }
}

pub fn stdout_mode() -> Mode {
    EnvFlags::from_env(std::io::stdout().is_terminal()).mode()
}
pub fn stderr_mode() -> Mode {
    EnvFlags::from_env(std::io::stderr().is_terminal()).mode()
}

/// The single static line printed instead of the animated banner.
pub fn plain_banner_line() -> String {
    format!("recollect-agent {}", env!("CARGO_PKG_VERSION"))
}

fn clear_line(writer: &mut impl Write, width: usize) -> std::io::Result<()> {
    write!(writer, "\r{padding}\r", padding = " ".repeat(width))
}

/// Animated wordmark reveal plus tagline on an interactive color terminal;
/// one plain line on any fallback. Writes stdout.
pub fn banner() {
    match stdout_mode() {
        Mode::Ansi => {
            let width = BANNER_FRAMES
                .iter()
                .map(|frame| frame.chars().count())
                .max()
                .unwrap_or(0);
            let mut out = std::io::stdout();
            for frame in BANNER_FRAMES {
                if clear_line(&mut out, width).is_err() {
                    return;
                }
                if write!(out, "{frame}").is_err() {
                    return;
                }
                let _ = out.flush();
                std::thread::sleep(FRAME_DELAY);
            }
            let _ = clear_line(&mut out, width);
            let _ = writeln!(out, "\u{1b}[1mrecollect\u{1b}[0m");
            let _ = writeln!(out, "{TAGLINE}");
        }
        Mode::Plain => println!("{}", plain_banner_line()),
        Mode::Quiet => {}
    }
}

/// Banner for binaries whose stdout is a protocol or machine contract: it
/// appears only when a human terminal owns stdout.
pub fn banner_quiet() {
    if std::io::stdout().is_terminal() {
        banner();
    }
}

pub mod status {
    use super::*;

    /// A live operation line. Dropping the handle stops the spinner thread if
    /// `finish` was not called, so a missing finish never hangs the process.
    pub struct Handle {
        label: String,
        started: Instant,
        mode: Mode,
        stop: Option<Arc<AtomicBool>>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Drop for Handle {
        fn drop(&mut self) {
            if let Some(stop) = &self.stop {
                stop.store(true, Ordering::Relaxed);
            }
        }
    }

    /// Spinner frame at a tick index; cycles through the braille sequence.
    pub(crate) fn frame_at(index: usize) -> char {
        SPINNER_FRAMES[index % SPINNER_FRAMES.len()]
    }

    /// Plain-mode start line, e.g. `Capture run…`.
    pub fn plain_start_line(label: &str) -> String {
        format!("{label}…")
    }

    /// Plain-mode finish lines: `<label> done` / `<label> failed: <detail>`.
    pub fn plain_finish_line(label: &str, ok: bool, detail: &str) -> String {
        if ok {
            format!("{label} done")
        } else {
            format!("{label} failed: {detail}")
        }
    }

    /// ANSI finish line with a colored outcome mark and elapsed time.
    pub fn ansi_finish_line(label: &str, ok: bool, detail: &str, elapsed: &str) -> String {
        let mark = if ok {
            "\u{1b}[32m✓\u{1b}[0m"
        } else {
            "\u{1b}[31m✗\u{1b}[0m"
        };
        if detail.is_empty() {
            format!("{mark} {label} ({elapsed})")
        } else {
            format!("{mark} {label} · {detail} ({elapsed})")
        }
    }

    /// Elapsed time with one decimal second, e.g. `1.2s`.
    pub fn format_elapsed(duration: Duration) -> String {
        format!("{:.1}s", duration.as_secs_f64())
    }

    fn spinner(stop: Arc<AtomicBool>, label: String) {
        let width = label.chars().count() + 2;
        let mut index = 0usize;
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(FRAME_DELAY);
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let line = format!("{} {label}", frame_at(index));
            index += 1;
            let mut err = std::io::stderr();
            let _ = write!(
                err,
                "\r{line}{padding}\r",
                padding = " ".repeat(width.saturating_sub(line.chars().count()))
            );
            let _ = err.flush();
        }
    }

    fn quiet_handle(label: &str) -> Handle {
        Handle {
            label: label.to_owned(),
            started: Instant::now(),
            mode: Mode::Quiet,
            stop: None,
            thread: None,
        }
    }

    /// Start a live status line on stderr. Animated on an interactive color
    /// terminal; one plain line on any fallback.
    pub fn start(label: &str) -> Handle {
        match stderr_mode() {
            Mode::Ansi => {
                let stop = Arc::new(AtomicBool::new(false));
                eprintln!("⏺ {label}");
                let owned_label = label.to_owned();
                let worker = stop.clone();
                let thread = std::thread::spawn(move || spinner(worker, owned_label));
                Handle {
                    label: label.to_owned(),
                    started: Instant::now(),
                    mode: Mode::Ansi,
                    stop: Some(stop),
                    thread: Some(thread),
                }
            }
            Mode::Plain => {
                eprintln!("{}", plain_start_line(label));
                Handle {
                    label: label.to_owned(),
                    started: Instant::now(),
                    mode: Mode::Plain,
                    stop: None,
                    thread: None,
                }
            }
            Mode::Quiet => quiet_handle(label),
        }
    }

    /// Start for binaries whose stdout is a protocol or machine contract: the
    /// line appears only when a human terminal owns the session.
    pub fn start_quiet(label: &str) -> Handle {
        if std::io::stdout().is_terminal() {
            start(label)
        } else {
            quiet_handle(label)
        }
    }

    /// Stop the line and print the outcome with elapsed time (ANSI mode) or a
    /// plain done/failed line (fallback mode).
    pub fn finish(mut handle: Handle, ok: bool, detail: &str) {
        let elapsed = format_elapsed(handle.started.elapsed());
        match handle.mode {
            Mode::Ansi => {
                if let Some(stop) = handle.stop.take() {
                    stop.store(true, Ordering::Relaxed);
                }
                if let Some(thread) = handle.thread.take() {
                    let _ = thread.join();
                }
                let mut err = std::io::stderr();
                let _ = clear_line(&mut err, handle.label.chars().count() + 2);
                eprintln!("{}", ansi_finish_line(&handle.label, ok, detail, &elapsed));
            }
            Mode::Plain => {
                eprintln!("{}", plain_finish_line(&handle.label, ok, detail));
            }
            Mode::Quiet => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(tty: bool, no_color: bool, clicolor_off: bool, ci: bool) -> EnvFlags {
        EnvFlags {
            tty,
            no_color,
            clicolor_off,
            ci,
        }
    }

    #[test]
    fn banner_fallback_selection() {
        assert_eq!(flags(true, false, false, false).mode(), Mode::Ansi);
        assert_eq!(flags(false, false, false, false).mode(), Mode::Plain);
        assert_eq!(flags(true, true, false, false).mode(), Mode::Plain);
        assert_eq!(flags(true, false, true, false).mode(), Mode::Plain);
        assert_eq!(flags(true, false, false, true).mode(), Mode::Plain);
    }

    #[test]
    fn plain_banner_line_is_static_and_ansi_free() {
        let line = plain_banner_line();
        assert_eq!(
            line,
            format!("recollect-agent {}", env!("CARGO_PKG_VERSION"))
        );
        assert!(!line.contains('\u{1b}'));
    }

    #[test]
    fn banner_frames_are_a_bounded_ascii_reveal() {
        assert!((3..=5).contains(&BANNER_FRAMES.len()));
        let words: Vec<String> = BANNER_FRAMES
            .iter()
            .map(|frame| frame.split('_').next().unwrap().to_owned())
            .collect();
        for pair in words.windows(2) {
            assert!(pair[1].starts_with(&pair[0]));
        }
        assert_eq!(words.last().unwrap(), "recollect");
        for frame in BANNER_FRAMES {
            assert!(frame.chars().all(|c| c.is_ascii_graphic()));
        }
    }

    #[test]
    fn spinner_frame_cycling() {
        assert_eq!(status::frame_at(0), '⠋');
        assert_eq!(status::frame_at(9), '⠏');
        assert_eq!(status::frame_at(10), status::frame_at(0));
        assert_eq!(status::frame_at(25), status::frame_at(5));
    }

    #[test]
    fn plain_status_lines() {
        assert_eq!(status::plain_start_line("Capture run"), "Capture run…");
        assert_eq!(
            status::plain_finish_line("Capture run", true, "ignored"),
            "Capture run done"
        );
        assert_eq!(
            status::plain_finish_line("MCP bridge", false, "timeout"),
            "MCP bridge failed: timeout"
        );
    }

    #[test]
    fn ansi_status_lines_are_colored() {
        let ok = status::ansi_finish_line("MCP runner", true, "", "1.2s");
        assert!(ok.contains("\u{1b}[32m✓"));
        assert!(ok.contains("(1.2s)"));
        let failed = status::ansi_finish_line("MCP bridge", false, "timeout", "0.4s");
        assert!(failed.contains("\u{1b}[31m✗"));
        assert!(failed.contains("timeout"));
    }

    #[test]
    fn status_plain_round_trip() {
        // Test harness stderr is not a TTY, so this exercises the plain path:
        // one start line, one finish line, no background thread.
        let handle = status::start("Test operation");
        status::finish(handle, true, "");
        let failed = status::start("Failed operation");
        status::finish(failed, false, "boom");
    }

    #[test]
    fn elapsed_formatting() {
        assert_eq!(status::format_elapsed(Duration::from_millis(0)), "0.0s");
        assert_eq!(status::format_elapsed(Duration::from_millis(1234)), "1.2s");
        assert_eq!(status::format_elapsed(Duration::from_millis(999)), "1.0s");
        assert_eq!(status::format_elapsed(Duration::from_secs(61)), "61.0s");
    }
}
