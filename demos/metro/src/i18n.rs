use crate::metro::{City, Line};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    English,
    Russian,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::English, Self::Russian];

    /// Выбирает русский только для основного языкового тега `ru`; остальные локали используют английский.
    pub fn from_locale(locale: Option<&str>) -> Self {
        if locale
            .and_then(|locale| locale.split(['-', '_', '.', '@']).next())
            .is_some_and(|primary| primary.eq_ignore_ascii_case("ru"))
        {
            Self::Russian
        } else {
            Self::English
        }
    }

    pub fn detect() -> Self {
        Self::from_locale(sys_locale::get_locale().as_deref())
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Русский",
        }
    }

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::English => 0,
            Self::Russian => 1,
        }
    }

    pub fn app_title(self) -> &'static str {
        match self {
            Self::English => "Pautinka · City connections",
            Self::Russian => "Pautinka · Городские связи",
        }
    }

    pub(crate) const fn city_name(self, city: City) -> &'static str {
        match (self, city) {
            (Self::Russian, _) => city.name(),
            (Self::English, City::SaintPetersburg) => "Saint Petersburg",
            (Self::English, City::Moscow) => "Moscow",
        }
    }

    pub(crate) const fn city_title(self, city: City) -> &'static str {
        match (self, city) {
            (Self::Russian, _) => city.title(),
            (Self::English, City::SaintPetersburg) => "Petersburg connected",
            (Self::English, City::Moscow) => "Moscow connected",
        }
    }

    pub(crate) const fn map_label(self, city: City) -> &'static str {
        match (self, city) {
            (Self::Russian, _) => city.map_label(),
            (Self::English, City::SaintPetersburg) => "Saint Petersburg metro map",
            (Self::English, City::Moscow) => "Moscow metro map",
        }
    }

    pub(crate) const fn city_description(self, city: City) -> &'static str {
        match (self, city) {
            (Self::Russian, _) => city.description(),
            (Self::English, City::SaintPetersburg) => {
                "Stations as of 18 October 2026. Closed stations allow through trains only. Geometry and travel times are illustrative and exclude waiting."
            }
            (Self::English, City::Moscow) => {
                "Historical snapshot as of 1 April 2021. Closed stations allow through trains only. Geographic map; illustrative times of 3 min per segment and 4 min per transfer, excluding waiting."
            }
        }
    }

    pub(crate) const fn line_name(self, line: Line) -> &'static str {
        if matches!(self, Self::Russian) {
            return line.name();
        }
        match line {
            Line::Red => "Kirovsko-Vyborgskaya",
            Line::Blue => "Moskovsko-Petrogradskaya",
            Line::Green => "Nevsko-Vasileostrovskaya",
            Line::Orange => "Lakhtinsko-Pravoberezhnaya",
            Line::Purple => "Frunzensko-Primorskaya",
            Line::Brown => "Krasnoselsko-Kalininskaya",
            Line::Moscow1 => "Sokolnicheskaya",
            Line::Moscow2 => "Zamoskvoretskaya",
            Line::Moscow3 => "Arbatsko-Pokrovskaya",
            Line::Moscow4 => "Filyovskaya",
            Line::Moscow5 => "Koltsevaya",
            Line::Moscow6 => "Kaluzhsko-Rizhskaya",
            Line::Moscow7 => "Tagansko-Krasnopresnenskaya",
            Line::Moscow8 => "Kalininskaya",
            Line::Moscow8A => "Solntsevskaya",
            Line::Moscow9 => "Serpukhovsko-Timiryazevskaya",
            Line::Moscow10 => "Lyublinsko-Dmitrovskaya",
            Line::Moscow11 => "Bolshaya Koltsevaya",
            Line::Moscow12 => "Butovskaya",
            Line::Moscow15 => "Nekrasovskaya",
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn apply_document(self, city: City) {
        if let Some(document) = web_sys::window().and_then(|window| window.document()) {
            document.set_title(self.app_title());
            if let Some(root) = document.document_element() {
                let _ = root.set_attribute("lang", if self == Self::Russian { "ru" } else { "en" });
            }
            if let Some(canvas) = document.get_element_by_id("pautinka") {
                let _ = canvas.set_attribute("aria-label", self.map_label(city));
            }
        }
    }
}

macro_rules! messages {
    ($($key:ident: $ru:literal => $en:literal),+ $(,)?) => {
        #[derive(Clone, Copy)]
        pub(crate) enum Text { $($key),+ }

        impl Language {
            pub(crate) const fn text(self, message: Text) -> &'static str {
                match (self, message) {
                    $((Self::Russian, Text::$key) => $ru,
                      (Self::English, Text::$key) => $en),+
                }
            }
        }
    };
}

messages! {
    LinesLabels: "Линии и подписи" => "Lines and labels",
    LinesDescription: "Геометрия связей и названия станций" => "Connection geometry and station names",
    StationLabels: "Подписи станций" => "Station labels",
    LabelZoom: "Показывать от масштаба" => "Show labels at zoom",
    SelectedLabelHint: "Выбранная станция подписана при любом масштабе." => "Selected stations are labeled at any zoom level.",
    Crossings: "Пересечения" => "Crossings",
    CrossingsDescription: "Различайте пересечение линий и пересадку" => "Distinguish line crossings from transfers",
    Navigation: "Навигация" => "Navigation",
    NavigationDescription: "Обзор большой схемы и перемещение по ней" => "Explore and navigate a large map",
    Grid: "Сетка" => "Grid",
    GridDescription: "Адаптивная сетка меняет детализацию при масштабировании" => "The adaptive grid adjusts its detail as you zoom",
    Minimap: "Мини-карта" => "Mini-map",
    MinimapPosition: "Положение мини-карты" => "Mini-map position",
    MinimapHint: "Нажмите или потяните на мини-карте, чтобы переместить обзор." => "Click or drag on the mini-map to move the view.",
    FitMap: "Вписать схему" => "Fit map",
    NavigationHint: "Фон — перемещение · колесо и щипок — масштаб · Home — вся схема" => "Drag the background to pan · scroll or pinch to zoom · Home to fit the map",
    Geometry: "Геометрия линий" => "Line geometry",
    Snapping: "Привязка" => "Snapping",
    SnapModes: "Режимы привязки" => "Snap modes",
    Horizontal: "Горизонталь" => "Horizontal",
    Vertical: "Вертикаль" => "Vertical",
    DiagonalUp: "Диагональ / (45°)" => "Diagonal / (45°)",
    DiagonalDown: "Диагональ \\ (45°)" => "Diagonal \\ (45°)",
    NoGridSnap: "Без привязки к сетке" => "No grid snapping",
    AdaptiveGridSnap: "К адаптивной сетке" => "Snap to adaptive grid",
    FixedGridSnap: "К базовой сетке" => "Snap to base grid",
    Straight: "Прямые" => "Straight",
    Curved: "Кривые" => "Curved",
    Orthogonal: "Углы 90°" => "90° angles",
    Octilinear: "Углы 45°" => "45° angles",
    CornerRadius: "Радиус скругления" => "Corner radius",
    Angular15: "Углы 15°" => "15° angles",
    TopLeft: "Слева сверху" => "Top left",
    TopRight: "Справа сверху" => "Top right",
    BottomLeft: "Слева снизу" => "Bottom left",
    BottomRight: "Справа снизу" => "Bottom right",
    SelectedEdge: "Выбранное ребро" => "Selected edge",
    SelectEdgeHint: "Выберите ребро для индивидуальной настройки" => "Select an edge to customize it",
    EdgePriority: "Приоритет ребра" => "Edge priority",
    EdgePriorityHint: "Ребро с большим приоритетом проходит сверху" => "Edges with higher priority are drawn on top",
    CustomEdgeStyle: "Особое оформление ребра" => "Custom edge style",
    ResetEdge: "Сбросить настройки ребра" => "Reset edge settings",
    CrossingStyle: "Оформление пересечений" => "Crossing style",
    NoCrossingStyle: "Без оформления" => "None",
    Gap: "Разрыв" => "Gap",
    Bridge: "Дуга-мостик" => "Bridge",
    TopEdge: "Верхнее ребро" => "Edge on top",
    ById: "По ID" => "By ID",
    HorizontalFirst: "Горизонтальные сверху" => "Horizontal first",
    VerticalFirst: "Вертикальные сверху" => "Vertical first",
    AutoSize: "Размер по толщине линий" => "Size from line width",
    Width: "Ширина" => "Width",
    LinePadding: "Отступ от линий" => "Line padding",
    BridgeHeight: "Высота дуги" => "Bridge height",
    BridgeSide: "Сторона дуги" => "Bridge side",
    Automatic: "Автоматически" => "Automatic",
    LeftOfEdge: "Слева по ходу ребра" => "Left of edge direction",
    RightOfEdge: "Справа по ходу ребра" => "Right of edge direction",
    Advanced: "Дополнительно" => "Advanced",
    MinAngle: "Минимальный угол" => "Minimum angle",
    BendClearance: "Отступ от изгибов" => "Bend clearance",
    ShowGrid: "Показывать сетку" => "Show grid",
    GridSnappingHint: "Видимость сетки и привязка к ней независимы. Привязка настраивается в нижней панели." => "Grid visibility and snapping are independent. Configure snapping in the bottom bar.",
    Pattern: "Рисунок" => "Pattern",
    GridLines: "Линии сетки" => "Grid lines",
    GridDots: "Точки сетки" => "Grid dots",
    GridSpacing: "Шаг сетки" => "Grid spacing",
    BaseStep: "Базовый шаг" => "Base step",
    ScreenSpacing: "Интервал на экране" => "Screen spacing",
    Appearance: "Оформление" => "Appearance",
    MinorSize: "Размер мелких делений" => "Minor mark size",
    MajorSize: "Размер основных делений" => "Major mark size",
    MinorMarks: "Мелкие деления" => "Minor marks",
    MajorMarks: "Основные деления" => "Major marks",
    Stations: "Станции" => "Stations",
    ShowSidebar: "Показать боковую панель" => "Show sidebar",
    HideSidebar: "Скрыть боковую панель" => "Hide sidebar",
    City: "Город" => "City",
    LinesHeading: "ЛИНИИ" => "LINES",
    AllLines: "Все линии" => "All lines",
    MetroHeading: "МЕТРО / 01" => "METRO / 01",
    ExploreHint: "Исследуйте линии и находите свой путь через город." => "Explore the lines and find your way across the city.",
    StationsUnit: "станции" => "stations",
    LinesUnit: "линий" => "lines",
    RouteHeading: "МАРШРУТ" => "ROUTE",
    SelectStationHint: "Выберите станцию на карте или через поиск." => "Select a station on the map or use search.",
    StationClosed: "Закрыта для входа и выхода; доступен проезд" => "Closed for boarding and exiting; through trains only",
    FromHere: "Отсюда" => "From here",
    ToHere: "Сюда" => "To here",
    NoDeparture: "Отправление не выбрано" => "No departure selected",
    NoDestination: "Назначение не выбрано" => "No destination selected",
    Minutes: "мин" => "min",
    StationsCount: "Станций" => "Stations",
    TransfersCount: "Пересадок" => "Transfers",
    TravelTimeHint: "Время условное, без ожидания поезда" => "Illustrative travel time, excluding waiting",
    RouteStations: "Станции маршрута" => "Route stations",
    ResetRoute: "Сбросить маршрут" => "Reset route",
    AboutPautinka: "Как устроена Pautinka ↗" => "How Pautinka works ↗",
    Search: "Поиск" => "Search",
    SearchHint: "Название или сокращение…" => "Station name or abbreviation…",
    StationSearch: "Поиск станции" => "Station search",
    NoStation: "Станция не найдена" => "No stations found",
    LineWord: "линия" => "line",
    StationsRoute: "Станции и маршрут" => "Stations and route",
    ZoomLabelsHint: "Приблизьте схему, чтобы увидеть названия станций" => "Zoom in to see station names",
    SelectStation: "Нажмите, чтобы выбрать станцию" => "Click to select a station",
    ExampleData: "по данным примера" => "sample data",
}

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn only_russian_primary_language_selects_russian() {
        for locale in [
            "ru",
            "ru-RU",
            "ru-BY",
            "RU-ru",
            "ru-Cyrl-RU",
            "ru_RU.UTF-8",
            "ru_RU@custom",
        ] {
            assert_eq!(
                Language::from_locale(Some(locale)),
                Language::Russian,
                "{locale}"
            );
        }
        for locale in [
            None,
            Some(""),
            Some("en"),
            Some("en-RU"),
            Some("fr-FR"),
            Some("de-DE"),
            Some("uk-UA"),
            Some("be-BY"),
            Some("zh-Hans-CN"),
            Some("C"),
            Some("POSIX"),
            Some("rus"),
            Some("russian"),
        ] {
            assert_eq!(
                Language::from_locale(locale),
                Language::English,
                "{locale:?}"
            );
        }
    }
}
