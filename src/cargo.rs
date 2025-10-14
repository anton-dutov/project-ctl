use super::*;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use owo_colors::OwoColorize;

const LAST_EDITION: &str = "2024";
const PROJECTS: &str = "Projects";

#[derive(Debug, serde::Deserialize)]
struct Project {
    #[serde(default)]
    has_bin: bool,

    #[serde(default)]
    path: String,

    package: ProjectMain,

    #[serde(default)]
    profile: Profiles,
}

#[derive(Debug, serde::Deserialize)]
struct ProjectMain {
    name: String,
    edition: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct Profiles {
    release: Option<Profile>,
    // debug: Option<Profile>,
}

#[derive(Debug, serde::Deserialize)]
struct Profile {
    lto: Option<bool>,
    strip: Option<bool>,
    debug: Option<bool>,
    rpath: Option<bool>,
    incremental: Option<bool>,

    #[serde(rename = "debug-assertions")]
    debug_assertions: Option<bool>,

    #[serde(rename = "opt-level")]
    opt_level: Option<toml::Value>,

    #[serde(rename = "codegen-units")]
    codegen_units: Option<usize>,
}

// [profile.release]
// rpath = false
// incremental   = false

impl Profile {
    fn is_ok(&self) -> bool {
        self.lto == Some(true)
            && self.strip == Some(true)
            && self.debug == Some(false)
            && self.debug_assertions == Some(false)
            && self.rpath == Some(false)
            && self.incremental == Some(false)
            && self.opt_level == Some(toml::Value::Integer(2))
            && self.codegen_units == Some(1)
    }
}

#[derive(Clap)]
pub enum Command {
    #[clap(name = "audit")]
    Audit,

    #[clap(name = "clean")]
    Clean,

    #[clap(name = "info")]
    Info,

    #[clap(name = "update")]
    Update,

    #[clap(name = "build-win")]
    BuildWin(BuildParams),
}

#[derive(Clap)]
pub struct BuildParams {
    #[clap(long)]
    use_upx: bool,

    #[clap(long)]
    release: bool,
}

pub fn cmd(paths: &[String], cmd: Option<&str>, with_debug: bool) -> Fallible<()> {
    for path in paths {
        let repos = projects(path, with_debug)?;

        let total = repos.len();

        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(80)
            .set_header(vec![
                Cell::new("Path").add_attribute(Attribute::Bold),
                Cell::new("Projects").fg(Color::Green),
            ])
            .add_row(vec![
                Cell::new(path)
                    .add_attribute(Attribute::Bold)
                    .fg(Color::White),
                Cell::new(total.to_string()).fg(Color::Yellow),
            ]);

        println!("{table}");

        for (i, target) in repos.iter().enumerate() {
            let i = i + 1;

            if let Err(_e) = nix::unistd::chdir(target.path.as_str()) {
                continue;
            }

            // Determine colors based on conditions
            let index_color = if let Some(cmd) = cmd {
                if std::process::Command::new("cargo")
                    .args([cmd])
                    .output()
                    .is_ok()
                {
                    "+"
                } else {
                    "-"
                }
            } else {
                "+"
            };

            let edition_color = if target.package.edition.is_none() {
                "-"
            } else if target.package.edition.as_deref() == Some(LAST_EDITION) {
                "+"
            } else {
                "-"
            };

            let edition = target.package.edition.as_deref().unwrap_or("----");

            let (release_text, release_color) = if !target.has_bin {
                ("", "=")
            } else if let Some(profile) = &target.profile.release {
                if profile.is_ok() {
                    ("R", "+")
                } else {
                    ("R-", "-")
                }
            } else {
                ("R!", "!")
            };

            let path = if let Some(path) = target.path.strip_prefix(path) {
                path.strip_prefix("/")
                    .map(|i| i.to_string())
                    .unwrap_or_default()
            } else {
                target.path.clone()
            };

            // Print formatted line using owo_colors
            let index_str = format!("{:04} ", i);
            let edition_str = format!("{:<5}", edition);
            let release_str = format!("{:<4}", release_text);
            let name_str = format!("{:24}", target.package.name);

            // Print formatted line using owo_colors
            let colored_index = match index_color {
                "+" => index_str.bright_black().to_string(),
                _ => index_str.red().to_string(),
            };

            let colored_edition = match edition_color {
                "+" => edition_str.white().to_string(),
                _ => edition_str.yellow().to_string(),
            };

            let colored_release = match release_color {
                "+" => release_str.bright_green().to_string(),
                "!" => release_str.bright_yellow().to_string(),
                "-" => release_str.bright_red().to_string(),
                _ => release_str.white().to_string(),
            };

            println!(
                "{}{}{}{}{}",
                colored_index,
                colored_edition,
                colored_release,
                name_str.bright_white(),
                path.white()
            );
        }
    }

    Ok(())
}

pub fn build_windows(_with_debug: bool, params: BuildParams) -> Fallible<()> {
    let current_dir = std::env::current_dir()?;

    println!("WORKDIR: {current_dir:?}");

    let cargo_toml = current_dir.join("Cargo.toml");
    let cargo_toml = std::fs::read_to_string(cargo_toml)?;
    let cargo_toml: CargoToml = toml::from_str(&cargo_toml)?;

    println!(
        "PROJECT: {} v{}",
        cargo_toml.pkg.name, cargo_toml.pkg.version
    );

    let targets = vec![
        ("x86_64-pc-windows-gnu", "x86_64"),
        ("i686-pc-windows-gnu", "x86_32"),
    ];

    let kind = if params.release { "release" } else { "debug" };
    let raw_name = format!("{}.exe", cargo_toml.pkg.name);

    for (target, suffix) in targets {
        let _zip_name = format!(
            "{}-v{}-windows-{}.exe",
            cargo_toml.pkg.name, cargo_toml.pkg.version, suffix
        );
        let dst_name = format!(
            "{}-v{}-windows-{}.exe",
            cargo_toml.pkg.name, cargo_toml.pkg.version, suffix
        );

        println!("TARGET: {target}");

        let mut args = vec!["build", "--target", target];

        if params.release {
            args.push("--release");
        }

        system_cmd("cargo", &args);
        system_cmd(
            "cp",
            &[
                &format!("./target/{target}/{kind}/{raw_name}"),
                &format!("/home/anton/share/{dst_name}"),
            ],
        );

        if params.use_upx {
            system_cmd("upx", &[&format!("/home/anton/share/{dst_name}")]);
        }

        // if use_zip:
        //     system("rm  -f ~/share/{zip_name}".format(zip_name=zip_name))
        //     system("cp     ~/share/{dst_name} ~/share/{raw_name}".format(dst_name=dst_name, raw_name=raw_name))
        //     system("cd ~/share/ && zip -m -9 {zip_name} ./{raw_name}".format(zip_name=zip_name, raw_name=raw_name))
    }
    //     is_clean   = "clean" in sys.argv
    // is_release = "debug" not in sys.argv
    // use_upx    = "noupx" not in sys.argv
    // use_zip    = False #"nozip" not in sys.argv

    // target  = "release" if is_release else "debug"
    // name    = pkg['name']
    // version = pkg['version']
    // suffix  = "" if is_release else "-dbg"
    // flag    = "--release" if is_release else ""

    // if not is_clean:
    //   suffix += '-p{}'.format(int(time.time() / 100))

    // system("PKG_CONFIG_ALLOW_CROSS=1 cargo build {} --target i686-pc-windows-gnu".format(flag))

    // print("{} BUILD: {} {} ".format(target.upper(), name, version))

    Ok(())
}

fn system_cmd(cmd: &str, args: &[&str]) {
    let output = std::process::Command::new(cmd).args(args).output().unwrap();

    if !output.stderr.is_empty() {
        println!("{}", String::from_utf8_lossy(&output.stderr));
    }
}

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CargoToml {
    #[serde(rename = "package")]
    pkg: CargoPackage,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    version: String,
}

fn projects(path: &str, _with_debug: bool) -> Fallible<Vec<Project>> {
    let mut result = Vec::new();
    let iter = WalkDir::new(path)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_name().to_str() == Some("Cargo.toml"));

    for repo in iter {
        let path_toml = repo.path();

        // println!("{:?}", path_toml);

        let parent = if let Some(parent) = path_toml.parent() {
            parent
        } else {
            continue;
        };

        // println!("{path_toml:?}");

        let mut project = if let Ok(content) = std::fs::read_to_string(path_toml) {
            match toml::from_str::<Project>(&content) {
                Ok(info) => info,
                Err(_err) => {
                    // println!("{path_toml:?}: {err}");
                    continue;
                }
            }
        } else {
            continue;
        };

        // println!("{:?}", project.package.name);

        project.has_bin = parent.join("src").join("main.rs").exists();

        project.path = parent.to_str().unwrap_or_default().to_owned();

        result.push(project);
    }

    Ok(result)
}
