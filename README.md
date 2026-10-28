# Pautinka

[English](README.md) · [Русский](README.ru.md)

[![crates.io](https://img.shields.io/crates/v/pautinka.svg)](https://crates.io/crates/pautinka)
[![docs.rs](https://docs.rs/pautinka/badge.svg)](https://docs.rs/pautinka)
[![CI](https://github.com/hexqnt/pautinka/actions/workflows/pautinka.yml/badge.svg)](https://github.com/hexqnt/pautinka/actions/workflows/pautinka.yml)

Interactive graph diagrams for Rust, designed for technological and engineering networks. Your application owns the data, positions, routes and editing history; Pautinka provides geometry and an optional egui canvas. The name means “little spiderweb” in Russian, pronounced “pah-oo-TEEN-kah”.

## Getting started

```toml
[dependencies]
pautinka = { version = "0.1", features = ["egui"] }
```

```rust
use pautinka::{EdgePath, RoutingMode, egui::{Canvas, Graph}};
```

Geometry is available at the crate root; the canvas is under `pautinka::egui`. No features are enabled by default: use `pautinka = "0.1"` for geometry alone.

- [pautinka-core](crates/pautinka-core): geometry, routing, snapping and spatial indexing.
- [pautinka-egui](crates/pautinka-egui): interactive canvas with selection, dragging, camera controls and custom rendering. See its README for integration details.

```sh
cargo run -p pautinka-egui --example basic
cargo run -p pautinka-egui --example editing
```
