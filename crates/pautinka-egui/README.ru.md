# pautinka-egui

[English](README.md) · [Русский](README.ru.md)

Интерактивный холст `egui` для сетевых схем. Реализуйте `Graph` для своих данных: виджет заимствует подписи и пути, а приложение управляет расположением элементов, маршрутами и историей изменений. Общая геометрия и маршрутизация находятся в [pautinka-core](../pautinka-core).

```toml
[dependencies]
egui = "0.37"
pautinka-egui = "0.1"
```

Храните `Canvas` между кадрами и готовьте маршруты при изменении данных:

```rust
use pautinka_egui::{Canvas, Graph, InteractionOptions};

fn network_ui(ui: &mut egui::Ui, canvas: &mut Canvas, graph: &dyn Graph) {
    canvas.options.interaction = InteractionOptions::read_only();
    if let Err(error) = canvas.try_show(ui, graph) {
        eprintln!("Cannot display network: {error}");
    }
}
```

## Возможности и примеры

- Множественный выбор, рамка выделения, клавиатурная навигация, подсказки и контекстные действия.
- Перетаскивание через события, отмена жестов, закреплённые узлы и вложенные рамки групп.
- Собственные renderer, формы узлов, подписи, штриховые линии, разрывы и мостики на пересечениях.
- Перемещение камеры, масштабирование, вписывание и фокусировка, мини-карта, адаптивная сетка и привязки к осям, диагоналям или сетке.
- Кеширование выбора и отсечения, локальное обновление пространственного индекса и подготовка в рабочем потоке.
- Опциональный экспорт SVG/PNG без окна и GPU.

Запуск примера: `cargo run -p pautinka-egui --example <name>`.

| Пример | Что показывает |
| --- | --- |
| [basic](examples/basic.rs) | Маршрутизация и управление камерой |
| [custom](examples/custom.rs) | Условные обозначения, порты и оформление соединений |
| [shapes](examples/shapes.rs) | Формы узлов и соединения по контуру |
| [editing](examples/editing.rs) | Перемещение выбранных узлов, отмена и Undo |
| [petgraph](examples/petgraph.rs) | Адаптер неплотного `StableDiGraph` |
| [groups](examples/groups.rs) | Вложенные группы, перемещение и свёрнутые представления |
| [labels](examples/labels.rs) | Автоматическое размещение подписей |

Перетаскивание фона перемещает камеру, прокрутка или pinch меняет масштаб, двойной щелчок по фону или Home вписывает граф. Ctrl/Command + щелчок переключает выбор, Shift + щелчок добавляет объект, Shift + перетаскивание выделяет область. При фокусе холста стрелки переключают объекты, Ctrl/Command + A выбирает всё, F фокусирует основной выбранный объект. Escape отменяет жест, а вне жеста снимает выбор.

## Интеграция

`Graph` использует плотные временные индексы и устойчивые ID. Повышайте версию топологии при добавлении, удалении и перестановке элементов, версию геометрии — при изменении позиций, маршрутов и границ, версию оформления — при визуальных изменениях, сохраняющих границы. Вычисляйте маршруты и `Graph::bounds` вне отрисовки. При смене документа вызывайте `Canvas::reset`, для нескольких холстов используйте разные области `ui.push_id`.

Применяйте `CanvasEvent` к модели и истории до следующего показа, который очищает буфер событий. Холст сам не меняет граф. `try_show` / `try_show_with` возвращают ошибки, `show` / `show_with` отображают их. Создание и переподключение связей реализует приложение; маршрутизация не обходит препятствия. Геометрия хранится в `f32`, поэтому выбирайте подходящие начало координат и единицы.

Собственный `Renderer` должен согласовать границы и проверку попадания с нарисованной геометрией. Используйте один renderer для подготовки, выбора и отрисовки; повышайте его версию при изменении пространственной геометрии. В `node_hit_test` используйте `HitPoint::relative_to(node.position)` для сохранения точности указателя. Для рисования за границами элементов объявляйте экранные поля.

`Graph::geometry_changes` позволяет обновлять пространственный индекс локально. `PreparedGraph::new` готовит индекс в рабочем потоке; `Canvas::install_prepared` принимает результат для того же экземпляра и версии графа. `Canvas::stats()` возвращает статистику кешей и индекса.

## Формы, рёбра и подписи

Возвращайте подготовленную `NodeShape` из `Graph::node_shape`: круг, эллипс, прямоугольник, треугольник, ромб или правильный многоугольник. Размеры задаются в мировых координатах; формы с углами поддерживают скругление. Отрисовка, выбор и точки подключения `anchor` учитывают контур.

```rust
use egui::Vec2;
use pautinka_egui::{CornerRadius, NodeShape};

let rectangle = NodeShape::rectangle(Vec2::new(80.0, 40.0))?
    .with_corner_radius(CornerRadius::try_from(12.0)?);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Задавайте `EdgeStyle::pattern` в `Renderer::edge_style`: `LinePattern::{Solid, Dashed, Dotted, DashDot, DashDotDot}`. Длины штрихов и промежутков измеряются в экранных точках. `Style::crossings` настраивает `CrossingMode::{None, Gap, Bridge}`, размеры и порядок пересечений ломаных и кривых Безье. `Canvas::set_edge_crossings` переопределяет приоритет и оформление по `EdgeId`. Петли, самопересечения и наложения не оформляются.

Автоматические подписи включаются через `canvas.options.labels = Some(labels::LabelLayoutOptions::default())`. Размещение учитывает другие подписи, узлы и границы viewport; рёбра пока не считаются препятствиями. `Renderer::node_label_options` задаёт приоритет, позиции и видимость. Собственная реализация `paint_node` должна учитывать `show_label = false`; автоматические подписи рисуются через `Renderer::paint_node_label`. `Canvas::node_labels()` возвращает полученные прямоугольники.

## Группы

Реализуйте `Graph::group_count` и `group`, возвращая заимствованные представления `Group`: внешние рамки перед внутренними. Создавайте рамки через `GroupFrame::new` или `GroupFrame::enclosing`. Группы поддерживают выбор, фокусировку, мини-карту и экспорт; составом и иерархией управляет приложение.

Перетаскивание заголовка выдаёт `GroupDragStarted`, `GroupDragged`, затем `GroupDragFinished` или `GroupDragCancelled`. Применяйте полное мировое смещение `delta` к снимку на начало жеста и восстанавливайте его при отмене. Привязки при перемещении групп не действуют. `Group::state` включает кнопку сворачивания с событием `GroupCollapseRequested`; приложение перестраивает видимый граф и маршруты. Холст сам не перемещает и не скрывает элементы.

`Graph::group_interaction` и общие настройки `interaction.group_drag`, `group_drag_modifier`, `group_toggle` управляют жестами. `read_only()` отключает запросы перемещения и сворачивания. Изменение рамок требует новой версии геометрии и `Selection::Group(index)` в полном журнале геометрии. Включайте рамки в `Graph::bounds`.

## Камера, сетка и привязки

Мини-карта включается через `canvas.options.navigator.enabled = true`; `navigator.corner` выбирает угол. Щелчок и перетаскивание перемещают камеру. При `interaction.pan = false` мини-карта доступна только для просмотра.

`canvas.options.grid = Some(GridOptions::default())` включает адаптивную мировую сетку. Вид задаётся через `GridKind::{Lines, Dots}`, цвета и толщины. `GridOptions::new(base_step, min_spacing)` принимает положительный конечный мировой шаг и экранный интервал не меньше 8 точек. При масштабировании шаг выбирается из ряда `base_step × 5ⁿ`. Для собственной подложки переопределите `Renderer::paint_background`; вызовите `paint_grid`, чтобы сохранить сетку. Экспорт этот метод не использует.

```rust
use pautinka_egui::{Canvas, GridOptions, GridSnapMode, SnapKinds};

let mut canvas = Canvas::default();
canvas.options.grid = Some(GridOptions::default());
canvas.options.interaction.snapping = true;
canvas.options.snapping.kinds = SnapKinds::AXES | SnapKinds::DIAGONALS;
canvas.options.grid_snap = GridSnapMode::Fixed;
```

По умолчанию привязки выключены. `GridSnapMode::Fixed` использует базовый шаг, `Adaptive` — текущий шаг отрисовки, `Disabled` оставляет только направляющие от узлов. `SnapKinds::NONE` позволяет привязываться только к сетке. Видимость сетки независима: `grid.visible = false` сохраняет настройки, а при `grid = None` привязка использует стандартные параметры. Допуски захвата и отпускания по умолчанию равны 6 и 9 экранным точкам. События перемещения содержат привязанные позиции; приложение применяет смещение ведущего узла к остальным выбранным узлам.

## Экспорт SVG и PNG

Опция `svg` включает модуль `export`, `png` — оба формата. Верхний крейт `pautinka` передаёт одноимённые опции. Экспорт возвращает SVG как `String` или закодированный PNG как `Vec<u8>`; сохранение выполняет приложение.

```rust
# #[cfg(feature = "png")]
# fn save(graph: &impl pautinka_egui::Graph) -> Result<(), Box<dyn std::error::Error>> {
use pautinka_egui::{DefaultRenderer, Style, export::{ExportOptions, Exporter}};

let exporter = Exporter::default();
let options = ExportOptions::new(2.0, 24.0)?;
let style = Style::default();
std::fs::write("diagram.svg", exporter.svg(graph, &DefaultRenderer, &style, &options)?)?;
std::fs::write("diagram.png", exporter.png(graph, &DefaultRenderer, &style, &options, 2.0)?)?;
# Ok(())
# }
```

`ExportOptions` управляет масштабом, полями, обрезкой, фоном, подписями и петлями. Автоматическое размещение подписей включается отдельно через `with_label_layout(Some(options))?`. Экспорт не включает элементы управления и выделение. Подписи сохраняются векторными контурами; для собственных шрифтов передайте `egui::FontDefinitions` в `Exporter::new`. Неподдерживаемая отрисовка, например текстуры или GPU callbacks, возвращает `ExportError::UnsupportedShape`.

```sh
cargo run -p pautinka-egui --features png --example export -- /tmp/pautinka-export
```
