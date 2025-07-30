use std::fs::read_to_string;
use std::io::{self, BufRead, BufWriter, Write};
use std::process::Command;

use ahash::AHashSet as HashSet;
use clap::{Parser, ValueEnum};
use colored::Colorize;

const LOG_FILE: &str = "/var/log/pacman.log";

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

fn main() {
    let args = Args::parse();

    match args.filter {
        Filter::A | Filter::All => show_all_logs(),
        Filter::I | Filter::Installed => filter_logs("installed", args.n),
        Filter::E | Filter::Explicitly => explicitly_installed(args.n),
        Filter::U | Filter::Upgraded => filter_logs("upgraded", args.n),
        Filter::R | Filter::Removed | Filter::Uninstalled => filter_logs("removed", args.n),
    }
}

fn show_all_logs() {
    let programs = ["nvim", "vim", "bat", "cat"];
    for p in &programs {
        if let Ok(mut child) = Command::new(p).args([LOG_FILE]).spawn() {
            child
                .wait()
                .unwrap_or_else(|e| panic!("Command {p} failed with error: {e}"));
            return;
        }
    }
    eprintln!("None of {:?} worked.", programs);
}

fn filter_logs(keyword: &str, max_entries: usize) {
    let logs = read_to_string(LOG_FILE).unwrap();
    let lock = io::stdout().lock();
    let mut buf = BufWriter::new(lock);
    logs.lines()
        .rev()
        .filter_map(|line| {
            line.split_once(" [ALPM] ").and_then(|(time, remaining)| {
                let mut parts = remaining.splitn(3, ' ');
                parts.next().and_then(|k| {
                    if k == keyword {
                        Some((time, parts.next().unwrap(), parts.next().unwrap()))
                    } else {
                        None
                    }
                })
            })
        })
        .take(max_entries)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .for_each(|(time, pkg, version)| {
            let _ = writeln!(buf, "{} {keyword} {} {}", time, pkg.bright_green(), version);
        });
}

fn explicitly_installed(max_entries: usize) {
    let mut explicit_pkgs = Command::new("pacman")
        .args(["-Qqe"])
        .output()
        .unwrap()
        .stdout
        .lines()
        .map_while(Result::ok)
        .collect::<HashSet<_>>();

    let logs = read_to_string(LOG_FILE).unwrap();

    let lock = io::stdout().lock();
    let mut buf = BufWriter::new(lock);
    logs.lines()
        .rev()
        .filter_map(|line| {
            line.split_once(" [ALPM] installed ")
                .and_then(|(time, remaining)| {
                    remaining.split_once(' ').and_then(|(pkg, version)| {
                        if explicit_pkgs.remove(pkg) {
                            Some((time, pkg, version))
                        } else {
                            None
                        }
                    })
                })
        })
        .take(max_entries)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .for_each(|(time, pkg, version)| {
            let _ = writeln!(buf, "{} installed {} {}", time, pkg.bright_green(), version);
        });
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
