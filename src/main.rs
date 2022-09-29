use std::fs::read_to_string;
use std::io::{self, BufRead, BufWriter, Write};
use std::process::Command;

use ahash::AHashSet as HashSet;
use clap::{Parser, ValueEnum};
use colored::Colorize;
use regex::Regex;

const LOG_FILE: &str = "/var/log/pacman.log";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
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
        if Command::new(p).args([LOG_FILE]).status().is_ok() {
            return;
        }
    }
    eprintln!("None of {:?} worked.", programs);
}

fn filter_logs(keyword: &str, max_entries: usize) {
    let logs = read_to_string(LOG_FILE).unwrap();
    let re = Regex::new(&format!(r"^\[(.*)\] \[ALPM\] {keyword} (\S+) (.*)$")).unwrap();
    let lock = io::stdout().lock();
    let mut buf = BufWriter::new(lock);
    logs.lines()
        .rev()
        .filter_map(|line| re.captures(line))
        .take(max_entries)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .for_each(|caps| {
            let _ = writeln!(
                buf,
                "{} {keyword} {} {}",
                &caps[1],
                &caps[2].bright_green(),
                &caps[3]
            );
        });
}

fn explicitly_installed(max_entries: usize) {
    let mut explicit_pkgs = Command::new("pacman")
        .args(["-Qqe"])
        .output()
        .unwrap()
        .stdout
        .lines()
        .filter_map(|l| match l {
            Ok(l) => Some(l),
            Err(_) => None,
        })
        .collect::<HashSet<_>>();

    let logs = read_to_string(LOG_FILE).unwrap();
    let re = Regex::new(r"\[(.*)\] \[ALPM\] installed (\S+) (.*)").unwrap();

    let lock = io::stdout().lock();
    let mut buf = BufWriter::new(lock);
    logs.lines()
        .rev()
        .filter_map(|line| {
            re.captures(line).map(|caps| {
                if explicit_pkgs.contains(&caps[2]) {
                    explicit_pkgs.remove(&caps[2]);
                    Some(caps)
                } else {
                    None
                }
            })
        })
        .flatten()
        .take(max_entries)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .for_each(|caps| {
            let _ = writeln!(
                buf,
                "{} installed {} {}",
                &caps[1],
                &caps[2].bright_green(),
                &caps[3]
            );
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
