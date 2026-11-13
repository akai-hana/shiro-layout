//! Terminal presentation helpers for `generate`: colors, progress bars, boxes
//! and side-by-side layout cards.  No dependencies: plain ANSI escapes.
//!
//! Color is only emitted when stdout is a terminal, `NO_COLOR` is unset and
//! `TERM` is not `dumb`.  Progress bars are drawn on stderr, and only when
//! stderr is a terminal, so redirecting or piping output stays clean.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug)]
pub struct Style {
    color: bool,
}

impl Style {
    pub fn detect() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty());
        let dumb = std::env::var("TERM").is_ok_and(|t| t == "dumb");
        Self {
            color: std::io::stdout().is_terminal() && !no_color && !dumb,
        }
    }

    pub fn plain() -> Self {
        Self { color: false }
    }

    fn paint(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}\x1b[0m")
        } else {
            s.to_string()
        }
    }

    pub fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }
    pub fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }
    pub fn green(&self, s: &str) -> String {
        self.paint("32", s)
    }
    pub fn yellow(&self, s: &str) -> String {
        self.paint("33", s)
    }
    pub fn cyan(&self, s: &str) -> String {
        self.paint("36", s)
    }
    pub fn bold_green(&self, s: &str) -> String {
        self.paint("1;32", s)
    }

    /// `▸ Label      ████████ 28/28   detail` (bar optional).
    pub fn section(&self, label: &str, bar: Option<(usize, usize)>, detail: &str) -> String {
        let mut out = format!("{} {}", self.cyan("▸"), self.bold(&format!("{label:<10}")));
        if let Some((done, total)) = bar {
            out.push_str(&format!(
                " {} {}/{}",
                bar_string(done, total, 20),
                done,
                total
            ));
        }
        if !detail.is_empty() {
            out.push_str("  ");
            out.push_str(detail);
        }
        out
    }

    /// Draw a rounded box around plain-text body lines.
    pub fn boxed(&self, title: &str, body: &[String]) -> Vec<String> {
        let body_w = body.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let title_w = title.chars().count() + 4;
        let inner = body_w.max(title_w) + 2;
        let mut out = Vec::new();
        let top_fill = "─".repeat(inner.saturating_sub(title.chars().count() + 3));
        out.push(format!(
            "{}{}{}{}",
            self.dim("╭─ "),
            self.bold(title),
            self.dim(" "),
            self.dim(&format!("{top_fill}╮"))
        ));
        for line in body {
            let pad = inner - 1 - line.chars().count();
            out.push(format!(
                "{} {}{}{}",
                self.dim("│"),
                line,
                " ".repeat(pad),
                self.dim("│")
            ));
        }
        out.push(self.dim(&format!("╰{}╯", "─".repeat(inner))));
        out
    }
}

pub fn bar_string(done: usize, total: usize, width: usize) -> String {
    let filled = (done.min(total) * width)
        .checked_div(total)
        .unwrap_or(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// Live progress bar on stderr (a plain counter when `total` is 0).  Safe to
/// tick from rayon worker threads.
pub struct Progress {
    label: String,
    total: usize,
    done: AtomicUsize,
    live: bool,
    lock: Mutex<()>,
}

impl Progress {
    pub fn new(label: &str, total: usize) -> Self {
        Self {
            label: label.to_string(),
            total,
            done: AtomicUsize::new(0),
            live: std::io::stderr().is_terminal(),
            lock: Mutex::new(()),
        }
    }

    pub fn tick(&self) {
        let n = self.done.fetch_add(1, Ordering::SeqCst) + 1;
        if !self.live {
            return;
        }
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut err = std::io::stderr().lock();
        if self.total == 0 {
            // Unknown total: show a running round counter instead of a bar.
            let _ = write!(
                err,
                "\r\x1b[2K\x1b[36m▸\x1b[0m {:<10} {} rounds so far",
                self.label, n
            );
        } else {
            let _ = write!(
                err,
                "\r\x1b[2K\x1b[36m▸\x1b[0m {:<10} {} {}/{}",
                self.label,
                bar_string(n, self.total, 20),
                n.min(self.total),
                self.total
            );
        }
        let _ = err.flush();
    }

    /// Clear the live line (idempotent).
    pub fn finish(&self) {
        if !self.live {
            return;
        }
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "\r\x1b[2K");
        let _ = err.flush();
    }
}

pub fn term_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map(|w| w.max(40))
        .unwrap_or(80)
}

/// Lay blocks of plain-text lines out side by side, wrapping to `width`.
pub fn columns(blocks: &[Vec<String>], gap: usize, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let widest = blocks
        .iter()
        .flat_map(|b| b.iter())
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    if widest == 0 {
        return out;
    }
    let per_row = ((width + gap) / (widest + gap)).max(1);
    for chunk in blocks.chunks(per_row) {
        let height = chunk.iter().map(|b| b.len()).max().unwrap_or(0);
        for line_idx in 0..height {
            let mut line = String::new();
            for (i, block) in chunk.iter().enumerate() {
                let cell = block.get(line_idx).map(String::as_str).unwrap_or("");
                line.push_str(cell);
                if i + 1 < chunk.len() {
                    line.push_str(&" ".repeat(widest - cell.chars().count() + gap));
                }
            }
            out.push(line.trim_end().to_string());
        }
        out.push(String::new());
    }
    out.pop();
    out
}

/// Shorten long digit runs in start names: `rand312482214` -> `rand…214`.
pub fn short_name(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let run = &chars[start..i];
            if run.len() >= 6 {
                out.push('…');
                out.extend(&run[run.len() - 3..]);
            } else {
                out.extend(run);
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Compact count: 200000 -> `200k`, 1000000 -> `1M`.
pub fn compact(n: usize) -> String {
    if n >= 1_000_000 && n.is_multiple_of(1_000_000) {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 && n.is_multiple_of(1_000) {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names() {
        assert_eq!(short_name("rand312482214"), "rand…214");
        assert_eq!(short_name("ILS1:rand312482214"), "ILS1:rand…214");
        assert_eq!(short_name("shiro-312481206"), "shiro-…206");
        assert_eq!(short_name("colemak-dh"), "colemak-dh");
        assert_eq!(short_name("polish1"), "polish1");
    }

    #[test]
    fn compacts() {
        assert_eq!(compact(200_000), "200k");
        assert_eq!(compact(1_000_000), "1M");
        assert_eq!(compact(8), "8");
        assert_eq!(compact(1_500), "1500");
    }

    #[test]
    fn bars() {
        assert_eq!(bar_string(0, 4, 8), "░░░░░░░░");
        assert_eq!(bar_string(2, 4, 8), "████░░░░");
        assert_eq!(bar_string(4, 4, 8), "████████");
    }

    #[test]
    fn boxes_are_rectangular() {
        let s = Style::plain();
        let lines = s.boxed("Best layout · clear winner", &["a".into(), "bbbb".into()]);
        let w = lines[0].chars().count();
        assert!(lines.iter().all(|l| l.chars().count() == w), "{lines:#?}");
        let wide = s.boxed("t", &["x".repeat(40)]);
        let w = wide[0].chars().count();
        assert!(wide.iter().all(|l| l.chars().count() == w));
    }

    #[test]
    fn columns_wrap() {
        let blocks: Vec<Vec<String>> = (0..3)
            .map(|i| vec![format!("card{i}"), "xx".into()])
            .collect();
        let out = columns(&blocks, 2, 12);
        assert_eq!(out[0], "card0  card1");
        assert_eq!(out[2], "");
        assert_eq!(out[3], "card2");
    }
}
