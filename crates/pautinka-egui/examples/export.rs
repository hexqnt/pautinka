mod support;

use std::{error::Error, fs, path::PathBuf};

use pautinka_egui::{
    DefaultRenderer, Style,
    export::{ExportOptions, Exporter},
};

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let graph = support::Network::default();
    let exporter = Exporter::default();
    let style = Style::default();
    let options = ExportOptions::new(2.0, 24.0)?;
    fs::create_dir_all(&directory)?;
    fs::write(
        directory.join("diagram.svg"),
        exporter.svg(&graph, &DefaultRenderer, &style, &options)?,
    )?;
    fs::write(
        directory.join("diagram.png"),
        exporter.png(&graph, &DefaultRenderer, &style, &options, 2.0)?,
    )?;
    println!(
        "Exported diagram.svg and diagram.png to {}",
        directory.display()
    );
    Ok(())
}
