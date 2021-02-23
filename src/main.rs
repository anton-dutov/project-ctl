use std::{
    fs::File
};
use std::io::Write;

use anyhow::{
    Result as Fallible,
    format_err
};

use walkdir::WalkDir;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use clap::Clap;
use serde::Deserialize;

static CFG_FILE: &str = "project-ctl.yaml";

#[derive(Debug, Deserialize)]
struct ProjectPath {
    pub path: String,
}

#[derive(Debug, Deserialize)]
struct Target {
    path: String,
    name: String,
}


/// Project control helper
#[derive(Clap)]
#[clap(name = "Project control", version = env!("CARGO_PKG_VERSION"), author = "Anton Dutov <anton.dutov@gmail.com>")]
struct Opts {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Clap)]
enum Command {
    #[clap(name = "cargo-update")]
    CargoUpdate(CargoUpdate),

    #[clap(name = "cargo-clean")]
    CargoClean(CargoClean),

    #[clap(name = "git-state")]
    GitState(GitState),
}

/// Run cargo update for all cargo repos
#[derive(Clap)]
struct CargoUpdate {
    /// print debug information verbosely
    #[clap(short = 'D', long)]
    debug: bool
}

/// Run cargo clean for all cargo repos
#[derive(Clap)]
struct CargoClean {
    /// print debug information verbosely
    #[clap(short = 'D', long)]
    debug: bool
}

/// Show git state of all repos
#[derive(Clap)]
struct GitState {
    /// Show all checked repos
    #[clap(short, long)]
    all: bool,
    /// print debug information verbosely
    #[clap(short = 'D', long)]
    debug: bool
}


fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }

}

fn run() -> Fallible<()> {

    let mut cfg = dirs::config_dir().ok_or_else(|| {
        format_err!("CONFIG DIR NOT FOUND")
    })?;

    cfg.push(CFG_FILE);


    let config = File::open(cfg)?;

    let paths: Vec<ProjectPath> = serde_yaml::from_reader(config)?;

    let repos: Vec<String> = paths.into_iter().map(|i| i.path).collect();


    if repos.is_empty() {
        return Err(format_err!("{}: Path not defined", CFG_FILE))
    }

    let opts: Opts = Opts::parse();

    match opts.cmd {
        Command::CargoClean(o) => {
            if let Err(e) = cmd_cargo_clean(&repos, o.debug) {
                println!("FAIL: {}", e);
            }
        }
        Command::CargoUpdate(o) => {
            if let Err(e) = cmd_cargo_update(&repos, o.debug) {
                println!("FAIL: {}", e);
            }
        }
        Command::GitState(o) => {
            if let Err(e) = cmd_git_state(&repos, o.all, o.debug) {
                println!("FAIL: {}", e);
            }
        }
    }

    Ok(())
}

fn cmd_git_state(paths: &[String], all: bool, _with_debug: bool) -> Fallible<()> {

    let term = Terminal {};


    for path in paths {

        println!("===[ {} ]=", path);

        let walker = WalkDir::new(&path).into_iter();

        for entry in  walker {

            let entry = entry?;

            if entry.file_name().to_str() != Some(".git") {
                continue;
            }

            if let Some(parent) = entry.path().parent() {

                let repo = parent
                    .to_str()
                    .unwrap_or_default()
                    .to_owned();


                let name = parent
                    .strip_prefix(path)?
                    .to_str()
                    .unwrap_or_default()
                    .to_owned();

                let (clr, st) = git_state(&repo)?;

                if !all && st.trim().is_empty() {
                    continue;
                }

                term.write(&[
                        &Output::FontColor(clr),
                        &Output::Text(st),
                        &Output::Reset,
                        &Output::Text(String::from(" ")),
                        &Output::Text(name),
                    ]
                ).ok();
            }
        }
    }

    Ok(())
}


fn cmd_cargo_update(paths: &[String], with_debug: bool) -> Fallible<()> {

    let term = Terminal {};


    for path in paths {

        let repos = cargo_projects(&path, with_debug)?;

        let total = repos.len();

        println!("===[ {} ({}) ]=", path, total);

        for (i, target) in repos.iter().enumerate() {

            if let Err(_e) = nix::unistd::chdir(target.path.as_str()) {
                continue;
            }

            let clr = if std::process::Command::new("cargo")
                .args(&["update"])
                .output()
                .is_ok() {
                Color::Rgb(0, 0xFF, 0)
            } else {
                Color::Rgb(0xFF, 0, 0)
            };
            term.write(&[
                    &Output::FontColor(clr),
                    &Output::Text(format!("{:03}/{:03} {}", i, total, target.name)),
                ]
            ).ok();
        }
    }

    Ok(())
}


fn cmd_cargo_clean(paths: &[String], with_debug: bool) -> Fallible<()> {

    let term = Terminal {};


    for path in paths {

        let repos = cargo_projects(&path, with_debug)?;

        let total = repos.len();

        println!("===[ {} ({}) ]=", path, total);

        for (i, target) in repos.iter().enumerate() {

            if let Err(_e) = nix::unistd::chdir(target.path.as_str()) {
                continue;
            }

            let clr = if std::process::Command::new("cargo")
                .args(&["clean"])
                .output()
                .is_ok() {
                Color::Rgb(0, 0xFF, 0)
            } else {
                Color::Rgb(0xFF, 0, 0)
            };
            term.write(&[
                    &Output::FontColor(clr),
                    &Output::Text(format!("{:03}/{:03} {}", i, total, target.name)),
                ]
            ).ok();
        }
    }

    Ok(())
}

fn git_state(path: &str) -> Fallible<(Color, String)> {
    nix::unistd::chdir(path)?;

    let st1 = std::process::Command::new("git")
            .args(&["diff", "--numstat", "--cached", "origin/master"])
            .output()?;

    let st2 = std::process::Command::new("git")
            .args(&["status", "-s"])
            .output()?;

    if ! st1.stderr.is_empty() {
       return Ok((Color::Rgb(0xFF, 0x00, 0x0), String::from("FAIL")))
    }

    let mut lines = st1.stdout.iter().filter(|&n| *n == 0x0A).count();

    lines += st2.stdout.iter().filter(|&n| *n == 0x0A).count();

    if lines == 0 {
        return Ok((Color::Rgb(0, 0xFF, 0), String::from("    ")))
    }

    Ok((Color::Rgb(0xFF, 0xFF, 0x0), format!("{:04}", lines)))
}


enum Output {
    Reset,
    FontColor(Color),
    Text(String)
}

#[derive(Debug)]
struct Terminal {}


impl Terminal {
    pub fn write(&self, data: &[&Output]) -> Fallible<()> {

        let mut stdout = StandardStream::stdout(ColorChoice::Always);

        for i in data {
            match i {
                Output::Reset => {
                    stdout.set_color(ColorSpec::new().set_fg(None))?;
                }

                Output::Text(t) => {
                    write!(&mut stdout, "{}", t)?;
                }

                Output::FontColor(c) => {
                    stdout.set_color(ColorSpec::new().set_fg(Some(*c)))?;
                }
            }
        }

        stdout.set_color(ColorSpec::new().set_fg(None))?;
        writeln!(&mut stdout)?;


        Ok(())
    }
}



fn cargo_projects(path: &str, _with_debug: bool) -> Fallible<Vec<Target>>{

    let mut result = Vec::new();
    let iter = WalkDir::new(path)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_name().to_str() == Some("Cargo.toml"));

    for repo in iter {
        let parent = if let Some(parent) = repo.path().parent() {
            parent
        } else {
            continue;
        };

        let repo = parent
            .to_str()
            .unwrap_or_default()
            .to_owned();

        let name = parent
            .strip_prefix(path)?
            .to_str()
            .unwrap_or_default()
            .to_owned();

        result.push(Target {path: repo, name});
    }

    Ok(result)
}