

use std::{
    fs::File
};
use std::io::Write;

use failure::{
    Error,
    format_err
};

use walkdir::WalkDir;

use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use clap::{Arg, App, SubCommand};
use serde::Deserialize;

static CFG_FILE: &str = "project-ctl.yaml";

#[derive(Debug, Deserialize)]
struct ProjectPath {
    pub path: String
}


fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }

}

fn run() -> Result<(), Error> {

    let mut cfg = dirs::config_dir().ok_or_else(|| {
        format_err!("CONFIG DIR NOT FOUND")
    })?;

    cfg.push(CFG_FILE);


    let config = File::open(cfg)?;

    let paths: Vec<ProjectPath> = serde_yaml::from_reader(config)?;

    let repos: Vec<String> = paths.into_iter().map(|i| i.path).collect();


    if repos.is_empty() {
        Err(format_err!("{}: Path not defined", CFG_FILE))?
    }


    let git_state = SubCommand::with_name("git-state")
        .about("Show git state of all repos")
        .arg(Arg::with_name("debug")
                .short("D")
                .help("print debug information verbosely"));

    let cargo_upd = SubCommand::with_name("cargo-update")
        .about("Run cargo update for all cargo repos")
        .arg(Arg::with_name("debug")
                .short("D")
                .help("print debug information verbosely"));

    let matches = App::new("Project control")
      .version(env!("CARGO_PKG_VERSION"))
      .author("Anton Dutov <anton.dutov@gmail.com>")
      .about("Project control helper")
      .subcommand(git_state)
      .subcommand(cargo_upd)
      .get_matches();


    if let Some(matches) = matches.subcommand_matches("git-state") {
        if let Err(e) = cmd_git_state(&repos, matches.is_present("debug")) {
            println!("FAIL: {}", e);
        }
    } else if let Some(_matches) = matches.subcommand_matches("cargo-update") {
        if let Err(e) = cmd_cargo_update(&repos, matches.is_present("debug")) {
            println!("FAIL: {}", e);
        }
    } else {
        println!("Command required, try help");
    }

    Ok(())
}

fn cmd_git_state(paths: &[String], _with_debug: bool) -> Result<(), Error> {

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


fn cmd_cargo_update(paths: &[String], _with_debug: bool) -> Result<(), Error> {

    let term = Terminal {};


    for path in paths {

        println!("===[ {} ]=", path);

        let walker = WalkDir::new(&path).into_iter();

        for entry in  walker {

            let entry = entry?;

            if entry.file_name().to_str() != Some("Cargo.toml") {
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

                nix::unistd::chdir(repo.as_str())?;

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
                        &Output::Text(name),
                    ]
                ).ok();
            }
        }
    }

    Ok(())
}

fn git_state(path: &str) -> Result<(Color, String), Error> {
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
struct Terminal {


}


impl Terminal {
    pub fn write(&self, data: &[&Output]) -> Result<(), Error> {

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