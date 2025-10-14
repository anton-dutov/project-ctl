use super::*;
use owo_colors::OwoColorize;

#[derive(Clap)]
pub enum Command {
    #[clap(name = "state")]
    State(State),
}

/// Show git state of all repos
#[derive(Clap)]
pub struct State {
    /// Show all checked repos
    #[clap(short, long)]
    pub all: bool,
}

pub fn cmd_state(paths: &[String], all: bool, _with_debug: bool) -> Fallible<()> {
    for path in paths {
        println!("{}", format!("===[ {} ]=", path).red());

        let walker = WalkDir::new(path).into_iter();

        for entry in walker {
            let entry = entry?;

            if entry.file_name().to_str() != Some(".git") {
                continue;
            }

            if let Some(parent) = entry.path().parent() {
                let repo = parent.to_str().unwrap_or_default().to_owned();

                let name = parent
                    .strip_prefix(path)?
                    .to_str()
                    .unwrap_or_default()
                    .to_owned();

                let (color, st) = git_state(&repo)?;

                if !all && st.trim().is_empty() {
                    continue;
                }

                // Print colored output using owo_colors
                let colored_status = match color {
                    "red" => st.red().to_string(),
                    "green" => st.green().to_string(),
                    "yellow" => st.yellow().to_string(),
                    _ => st.white().to_string(),
                };

                println!("{} {}", colored_status, name);
            }
        }
    }

    Ok(())
}

fn git_state(path: &str) -> Fallible<(&'static str, String)> {
    nix::unistd::chdir(path)?;

    let mut st1 = std::process::Command::new("git")
        .args(["diff", "--numstat", "--cached", "origin/main"])
        .output()?;

    if !st1.stderr.is_empty() {
        st1 = std::process::Command::new("git")
            .args(["diff", "--numstat", "--cached", "origin/master"])
            .output()?
    };

    let st2 = std::process::Command::new("git")
        .args(["status", "-s"])
        .output()?;

    if !st1.stderr.is_empty() {
        println!("{:?}", String::from_utf8_lossy(&st1.stderr));
        return Ok(("red", String::from("FAIL")));
    }

    let mut lines = st1.stdout.iter().filter(|&n| *n == 0x0A).count();

    lines += st2.stdout.iter().filter(|&n| *n == 0x0A).count();

    if lines == 0 {
        return Ok(("green", String::from("    ")));
    }

    Ok(("yellow", format!("{:04}", lines)))
}
