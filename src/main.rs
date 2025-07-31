use std::fs::read_to_string;
use std::io::{BufRead, BufWriter, Error, Result, Write};
use std::process::Command;

use ahash::AHashSet as HashSet;
use anstyle::{AnsiColor, Color, Style};
use clap::{Parser, ValueEnum};

const LOG_FILE: &str = "/var/log/pacman.log";
const STYLE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightGreen)));

#[derive(Parser)]
#[command(version, about, long_about = None)]
/// A tool to filter pacman log
struct Args {
    /// Maximum number of most recent lines to output
    #[arg(short, default_value_t = usize::MAX)]
    n: usize,

    #[arg(value_enum, default_value_t = Filter::All)]
    filter: Filter,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Filter {
    A,
    All,
    I,
    Installed,
    E,
    Explicitly,
    U,
    Upgraded,
    R,
    Removed,
    Uninstalled,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.filter {
        Filter::A | Filter::All => show_all_logs(),
        Filter::I | Filter::Installed => filter_logs("installed", args.n),
        Filter::E | Filter::Explicitly => explicitly_installed(args.n),
        Filter::U | Filter::Upgraded => filter_logs("upgraded", args.n),
        Filter::R | Filter::Removed | Filter::Uninstalled => filter_logs("removed", args.n),
    }
}

fn show_all_logs() -> Result<()> {
    let programs = ["nvim", "vim", "bat", "cat"];
    for p in &programs {
        if let Ok(mut child) = Command::new(p).args([LOG_FILE]).spawn() {
            let s = child.wait()?;
            if s.success() {
                return Ok(());
            } else if let Some(code) = s.code() {
                let msg = format!("Process {p} exited with status code: {code}");
                return Err(Error::other(msg));
            } else {
                return Err(Error::other(format!("Process {p} terminated by signal")));
            }
        }
    }
    Err(Error::other(format!("None of {programs:?} worked.")))
}

fn filter_logs(keyword: &str, max_entries: usize) -> Result<()> {
    let logs = read_to_string(LOG_FILE)?;

    let start = STYLE.render();
    let end = STYLE.render_reset();

    let mut lines = Vec::with_capacity(1024);
    for line in logs.lines().rev() {
        if let Some((time, remaining)) = line.split_once(" [ALPM] ") {
            let mut parts = remaining.splitn(3, ' ');
            if parts.next() == Some(keyword) {
                if let Some(pkg) = parts.next()
                    && let Some(version) = parts.next()
                {
                    lines.push((time, pkg, version));
                    if lines.len() >= max_entries {
                        break;
                    }
                } else {
                    return Err(Error::other(format!("invalid line {line}")));
                }
            }
        }
    }

    let lock = anstream::stdout().lock();
    let mut buf = BufWriter::new(lock);
    for (time, pkg, version) in lines.into_iter().rev() {
        writeln!(buf, "{time} {keyword} {start}{pkg}{end} {version}")?;
    }

    Ok(())
}

fn explicitly_installed(max_entries: usize) -> Result<()> {
    let mut explicit_pkgs = Command::new("pacman")
        .args(["-Qqe"])
        .output()?
        .stdout
        .lines()
        .map_while(Result::ok)
        .collect::<HashSet<_>>();

    let logs = read_to_string(LOG_FILE)?;

    let start = STYLE.render();
    let end = STYLE.render_reset();

    let mut lines = Vec::with_capacity(1024);
    for line in logs.lines().rev() {
        if let Some((time, remaining)) = line.split_once(" [ALPM] installed ") {
            if let Some((pkg, version)) = remaining.split_once(' ') {
                if explicit_pkgs.remove(pkg) {
                    lines.push((time, pkg, version));
                    if lines.len() >= max_entries {
                        break;
                    }
                }
            } else {
                return Err(Error::other(format!("invalid line {line}")));
            }
        }
    }

    let lock = anstream::stdout().lock();
    let mut buf = BufWriter::new(lock);
    for (time, pkg, version) in lines.into_iter().rev() {
        writeln!(buf, "{time} installed {start}{pkg}{end} {version}")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
