use std::{env, error::Error, num::NonZeroUsize, path::PathBuf, str::FromStr};

use pautinka_test_support::canvas::SceneKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Fit,
    Local,
}

impl View {
    pub(super) const ALL: [Self; 2] = [Self::Fit, Self::Local];

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Fit => "fit",
            Self::Local => "local",
        }
    }
}

impl FromStr for View {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|view| view.as_str() == value)
            .ok_or("Unknown view (expected fit or local)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Idle,
    Pan,
    Zoom,
    Pick,
    Move,
    SnapMove,
}

impl Action {
    pub(super) const ALL: [Self; 6] = [
        Self::Idle,
        Self::Pan,
        Self::Zoom,
        Self::Pick,
        Self::Move,
        Self::SnapMove,
    ];

    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Pan => "pan",
            Self::Zoom => "zoom",
            Self::Pick => "pick",
            Self::Move => "move",
            Self::SnapMove => "snap-move",
        }
    }
}

impl FromStr for Action {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|action| action.as_str() == value)
            .ok_or("Unknown action (expected idle, pan, zoom, pick, move or snap-move)")
    }
}

pub struct Options {
    pub(super) frames: NonZeroUsize,
    pub(super) sizes: Vec<NonZeroUsize>,
    pub(super) scene: Option<SceneKind>,
    pub(super) view: Option<View>,
    pub(super) action: Option<Action>,
    pub(super) no_lod: bool,
    pub(super) smart_labels: bool,
    pub(super) bridges: bool,
    pub(super) output: Option<PathBuf>,
}

impl Options {
    pub(super) fn parse() -> Result<Self, Box<dyn Error>> {
        Self::parse_args(env::args().skip(1))
    }

    fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            frames: NonZeroUsize::new(60).unwrap(),
            sizes: [1000, 10_000].map(|n| NonZeroUsize::new(n).unwrap()).into(),
            scene: None,
            view: None,
            action: None,
            no_lod: false,
            smart_labels: false,
            bridges: false,
            output: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--frames" => {
                    options.frames = args.next().ok_or("Missing --frames value")?.parse()?;
                }
                "--sizes" => {
                    options.sizes = args
                        .next()
                        .ok_or("Missing --sizes value")?
                        .split(',')
                        .map(str::parse)
                        .collect::<Result<_, _>>()?;
                }
                "--scene" => {
                    options.scene = Some(args.next().ok_or("Missing --scene value")?.parse()?);
                }
                "--view" => {
                    options.view = Some(args.next().ok_or("Missing --view value")?.parse()?);
                }
                "--action" => {
                    options.action = Some(args.next().ok_or("Missing --action value")?.parse()?);
                }
                // Совместимость с командами исторических сравнений.
                "--canvas-only" => {}
                "--no-lod" => options.no_lod = true,
                "--smart-labels" => options.smart_labels = true,
                "--bridges" => options.bridges = true,
                "--stress" => options.sizes.push(NonZeroUsize::new(100_000).unwrap()),
                "--output" => {
                    options.output = Some(args.next().ok_or("Missing --output value")?.into());
                }
                _ => return Err(format!("Unknown argument: {arg}").into()),
            }
        }
        options.sizes.sort_unstable();
        options.sizes.dedup();
        if options.frames.get() < 3
            && options
                .action
                .is_none_or(|action| matches!(action, Action::Move | Action::SnapMove))
        {
            return Err("Moving scenario requires at least three frames".into());
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_filters_are_optional_and_typed() {
        let defaults = Options::parse_args([]).unwrap();
        assert_eq!(defaults.view, None);
        assert_eq!(defaults.action, None);

        let selected = Options::parse_args(
            [
                "--view",
                "local",
                "--action",
                "pan",
                "--scene",
                "orthogonal",
                "--canvas-only",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(selected.view, Some(View::Local));
        assert_eq!(selected.action, Some(Action::Pan));
        assert_eq!(selected.scene, Some(SceneKind::Orthogonal));

        for args in [
            vec!["--view"],
            vec!["--view", "overview"],
            vec!["--action"],
            vec!["--action", "drag"],
            vec!["--scene"],
            vec!["--scene", "unknown"],
            vec!["--action", "move", "--frames", "1"],
        ] {
            assert!(Options::parse_args(args.into_iter().map(str::to_owned)).is_err());
        }
    }
}
