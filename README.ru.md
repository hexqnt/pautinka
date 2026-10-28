# Pautinka

[English](README.md) · [Русский](README.ru.md)

[![crates.io](https://img.shields.io/crates/v/pautinka.svg)](https://crates.io/crates/pautinka)
[![docs.rs](https://docs.rs/pautinka/badge.svg)](https://docs.rs/pautinka)
[![CI](https://github.com/hexqnt/pautinka/actions/workflows/pautinka.yml/badge.svg)](https://github.com/hexqnt/pautinka/actions/workflows/pautinka.yml)

Интерактивные графовые схемы на Rust для технологических и инженерных сетей. Приложение владеет данными, координатами, маршрутами рёбер и историей изменений; Pautinka предоставляет геометрию и опциональный холст egui. Название — транслитерация слова «паутинка».

## Начало работы

```toml
[dependencies]
pautinka = { version = "0.1", features = ["egui"] }
```

```rust
use pautinka::{EdgePath, RoutingMode, egui::{Canvas, Graph}};
```

Геометрия доступна в корне крейта, холст — в `pautinka::egui`. По умолчанию все опции отключены: для одной геометрии достаточно `pautinka = "0.1"`.

- [pautinka-core](crates/pautinka-core): геометрия, трассировка рёбер, привязка и пространственный индекс.
- [pautinka-egui](crates/pautinka-egui): интерактивный холст с выделением, перетаскиванием, управлением камерой и настраиваемой отрисовкой. Подробности интеграции — в [русском руководстве](crates/pautinka-egui/README.ru.md).

```sh
cargo run -p pautinka-egui --example basic
cargo run -p pautinka-egui --example editing
```
