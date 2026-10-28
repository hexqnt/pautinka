use std::{env, error::Error, ffi::OsString, fs, io, path::PathBuf};

use pautinka_stress::Scenario;

pub(super) fn run() -> Result<(), Box<dyn Error>> {
    let Command::Run(config) = Command::parse(env::args_os().skip(1))? else {
        println!(
            "Usage: pautinka-stress [--config FILE]\n\nLoad a TOML scenario, or start the interactive demo with default settings."
        );
        return Ok(());
    };
    let scenario = if let Some(path) = config {
        let source = fs::read_to_string(&path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("Unable to read {}: {error}", path.display()),
            )
        })?;
        source.parse::<Scenario>().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid scenario {}: {error}", path.display()),
            )
        })?
    } else {
        Scenario::default()
    };
    eframe::run_native(
        "Pautinka · Stress laboratory",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 960.0])
                .with_min_inner_size([360.0, 480.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(pautinka_stress::Demo::with_scenario(scenario)))),
    )?;
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    Run(Option<PathBuf>),
    Help,
}

impl Command {
    fn parse(mut args: impl Iterator<Item = OsString>) -> Result<Self, String> {
        let mut config = None;
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--help" | "-h") => return Ok(Self::Help),
                Some("--config") => {
                    if config.is_some() {
                        return Err("--config can only be specified once".into());
                    }
                    let path = args
                        .next()
                        .filter(|path| !path.is_empty())
                        .ok_or("--config requires a non-empty path")?;
                    config = Some(path.into());
                }
                _ => return Err(format!("Unexpected argument: {}", arg.to_string_lossy())),
            }
        }
        Ok(Self::Run(config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Command, String> {
        Command::parse(args.iter().map(OsString::from))
    }

    #[test]
    fn launch_modes_preserve_paths_with_spaces() {
        assert_eq!(parse(&[]).unwrap(), Command::Run(None));
        assert_eq!(parse(&["--help"]).unwrap(), Command::Help);
        assert_eq!(parse(&["-h"]).unwrap(), Command::Help);
        assert_eq!(
            parse(&["--config", "my scenario.toml"]).unwrap(),
            Command::Run(Some(PathBuf::from("my scenario.toml")))
        );
    }

    #[test]
    fn invalid_arguments_do_not_start_the_application() {
        for args in [
            &["--config"][..],
            &["--config", ""],
            &["--config", "a.toml", "--config", "b.toml"],
            &["--unknown"],
            &["scenario.toml"],
            &["--config", "a.toml", "unexpected"],
        ] {
            assert!(parse(args).is_err(), "Accepted invalid arguments: {args:?}");
        }
    }
}
