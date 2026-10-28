use egui::{Modifiers, PointerButton};

/// Модификаторы жестов проверяются одинаково на узлах и фоне; Command включает Ctrl.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModifierKey {
    #[default]
    None,
    Shift,
    Control,
    Command,
    Alt,
}

impl ModifierKey {
    pub fn matches(self, modifiers: Modifiers) -> bool {
        match self {
            Self::None => modifiers.is_none(),
            Self::Shift => modifiers.shift,
            Self::Control => modifiers.ctrl,
            Self::Command => modifiers.command || modifiers.ctrl,
            Self::Alt => modifiers.alt,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DetailLevel {
    #[default]
    Full,
    Overview,
}

#[derive(Clone, Copy, Debug)]
pub struct InteractionOptions {
    /// Включает выбранные в `CanvasOptions` режимы привязки при перемещении.
    pub snapping: bool,
    pub selection: bool,
    pub node_drag: bool,
    pub group_drag: bool,
    pub group_toggle: bool,
    pub pan: bool,
    pub zoom: bool,
    pub keyboard: bool,
    pub activation: bool,
    pub context_menu: bool,
    pub double_click_fit: bool,
    pub pan_button: PointerButton,
    pub pan_modifier: ModifierKey,
    pub node_drag_modifier: ModifierKey,
    pub group_drag_modifier: ModifierKey,
    pub marquee_modifier: ModifierKey,
    pub toggle_modifier: ModifierKey,
    pub add_modifier: ModifierKey,
}

impl InteractionOptions {
    /// Просмотр сохраняет навигацию и выбор, но не создаёт намерений изменения модели.
    pub fn read_only() -> Self {
        Self {
            node_drag: false,
            group_drag: false,
            group_toggle: false,
            ..Self::default()
        }
    }
    pub fn disabled() -> Self {
        Self {
            selection: false,
            node_drag: false,
            group_drag: false,
            group_toggle: false,
            pan: false,
            zoom: false,
            keyboard: false,
            activation: false,
            context_menu: false,
            double_click_fit: false,
            ..Self::default()
        }
    }
}

impl Default for InteractionOptions {
    fn default() -> Self {
        Self {
            snapping: false,
            selection: true,
            node_drag: true,
            group_drag: true,
            group_toggle: true,
            pan: true,
            zoom: true,
            keyboard: true,
            activation: true,
            context_menu: true,
            double_click_fit: true,
            pan_button: PointerButton::Primary,
            pan_modifier: ModifierKey::None,
            node_drag_modifier: ModifierKey::None,
            group_drag_modifier: ModifierKey::None,
            marquee_modifier: ModifierKey::Shift,
            toggle_modifier: ModifierKey::Command,
            add_modifier: ModifierKey::Shift,
        }
    }
}

/// Возможности выбора и переноса элемента дополняют глобальную политику; закрепление не мешает выбору.
#[derive(Clone, Copy, Debug)]
pub struct NodeInteraction {
    pub selectable: bool,
    pub draggable: bool,
}

impl Default for NodeInteraction {
    fn default() -> Self {
        Self {
            selectable: true,
            draggable: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomRange {
    min: f32,
    max: f32,
}

impl ZoomRange {
    pub fn new(min: f32, max: f32) -> Option<Self> {
        (min.is_finite() && max.is_finite() && min > 0.0 && min <= max).then_some(Self { min, max })
    }
    pub fn min(self) -> f32 {
        self.min
    }
    pub fn max(self) -> f32 {
        self.max
    }
    pub fn clamp(self, zoom: f32) -> f32 {
        zoom.clamp(self.min, self.max)
    }
}

impl Default for ZoomRange {
    fn default() -> Self {
        Self {
            min: f32::MIN_POSITIVE,
            max: f32::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CameraOptions {
    pub zoom: ZoomRange,
    pub fit_margin: f32,
    pub focus_margin: f32,
    /// Увеличение при фокусировке точки относительно полного графа, независимо от мировых единиц.
    pub focus_scale: f32,
    pub limit_zoom_out_to_fit: bool,
    pub wheel_sensitivity: f32,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            zoom: ZoomRange::default(),
            fit_margin: 16.0,
            focus_margin: 48.0,
            focus_scale: 4.0,
            limit_zoom_out_to_fit: true,
            wheel_sensitivity: 0.003,
        }
    }
}

/// В обзорном режиме сохраняются пути и доступность; убираются стрелки, обводки и обычные подписи.
#[derive(Clone, Copy, Debug)]
pub struct LodOptions {
    pub enabled: bool,
    pub overview_below_zoom: f32,
    pub overview_above_elements: usize,
}

impl Default for LodOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            overview_below_zoom: 0.2,
            overview_above_elements: 20_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasOptions {
    /// Общие параметры рисунка и привязки к сетке; `None` скрывает рисунок и оставляет стандартные шаги привязки.
    pub grid: Option<crate::GridOptions>,
    /// Режим привязки к пересечениям сетки; использует общие допуски `snapping` и переключатель `interaction.snapping`.
    pub grid_snap: crate::GridSnapMode,
    /// `None` сохраняет обычные подписи без подготовки раскладки и выделения её буферов.
    pub labels: Option<crate::labels::LabelLayoutOptions>,
    pub navigator: crate::NavigatorOptions,
    /// Виды привязки и экранные допуски захвата и отпускания.
    pub snapping: crate::SnapOptions,
    pub interaction: InteractionOptions,
    pub camera: CameraOptions,
    pub lod: LodOptions,
}
