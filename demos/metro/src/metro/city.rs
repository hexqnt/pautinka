use super::Line;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum City {
    #[default]
    SaintPetersburg,
    Moscow,
}

impl City {
    pub const ALL: [Self; 2] = [Self::SaintPetersburg, Self::Moscow];

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SaintPetersburg => "Санкт-Петербург",
            Self::Moscow => "Москва",
        }
    }

    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::SaintPetersburg => "Петербург\nна связи",
            Self::Moscow => "Москва\nна связи",
        }
    }

    #[must_use]
    pub const fn map_label(self) -> &'static str {
        match self {
            Self::SaintPetersburg => "Схема метро Санкт-Петербурга",
            Self::Moscow => "Схема метро Москвы",
        }
    }

    #[must_use]
    pub const fn lines(self) -> &'static [Line] {
        match self {
            Self::SaintPetersburg => &Line::SAINT_PETERSBURG,
            Self::Moscow => &Line::MOSCOW,
        }
    }

    pub(super) const fn data(self) -> &'static str {
        match self {
            Self::SaintPetersburg => include_str!("../../data/spb.json"),
            Self::Moscow => include_str!("../../data/msk.json"),
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::SaintPetersburg => {
                "Состав станций на 18.10.2026. Закрытые станции доступны только для проезда. Геометрия и время условные, без ожидания поезда."
            }
            Self::Moscow => {
                "Исторический срез на 01.04.2021. Закрытые станции доступны только для проезда. Географическая схема; условно 3 мин на перегон и 4 мин на переход, без ожидания поезда."
            }
        }
    }
}
