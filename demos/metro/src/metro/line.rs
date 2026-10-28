use egui::Color32;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum Line {
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "#8e479b")]
    Purple,
    #[serde(rename = "brown")]
    Brown,
    #[serde(rename = "1SL")]
    Moscow1,
    #[serde(rename = "2ZL")]
    Moscow2,
    #[serde(rename = "3APL")]
    Moscow3,
    #[serde(rename = "4FL")]
    Moscow4,
    #[serde(rename = "5KL")]
    Moscow5,
    #[serde(rename = "6KRL")]
    Moscow6,
    #[serde(rename = "7TKL")]
    Moscow7,
    #[serde(rename = "8KL")]
    Moscow8,
    #[serde(rename = "8SL")]
    Moscow8A,
    #[serde(rename = "9STL")]
    Moscow9,
    #[serde(rename = "10LDL")]
    Moscow10,
    #[serde(rename = "11BKL")]
    Moscow11,
    #[serde(rename = "12BL")]
    Moscow12,
    #[serde(rename = "15NL")]
    Moscow15,
}

impl Line {
    pub const SAINT_PETERSBURG: [Self; 6] = [
        Self::Red,
        Self::Blue,
        Self::Green,
        Self::Orange,
        Self::Purple,
        Self::Brown,
    ];

    pub const MOSCOW: [Self; 14] = [
        Self::Moscow1,
        Self::Moscow2,
        Self::Moscow3,
        Self::Moscow4,
        Self::Moscow5,
        Self::Moscow6,
        Self::Moscow7,
        Self::Moscow8,
        Self::Moscow8A,
        Self::Moscow9,
        Self::Moscow10,
        Self::Moscow11,
        Self::Moscow12,
        Self::Moscow15,
    ];

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.info().name
    }

    #[must_use]
    pub const fn number(self) -> &'static str {
        self.info().number
    }

    #[must_use]
    pub const fn color(self) -> Color32 {
        let [red, green, blue] = self.info().color;
        Color32::from_rgb(red, green, blue)
    }

    const fn info(self) -> LineInfo {
        let (number, name, color) = match self {
            Self::Red => ("1", "Кировско-Выборгская", [226, 77, 88]),
            Self::Blue => ("2", "Московско-Петроградская", [65, 132, 222]),
            Self::Green => ("3", "Невско-Василеостровская", [31, 161, 124]),
            Self::Orange => ("4", "Лахтинско-Правобережная", [234, 149, 51]),
            Self::Purple => ("5", "Фрунзенско-Приморская", [158, 103, 192]),
            Self::Brown => ("6", "Красносельско-Калининская", [155, 100, 60]),
            Self::Moscow1 => ("1", "Сокольническая", [226, 50, 50]),
            Self::Moscow2 => ("2", "Замоскворецкая", [45, 160, 70]),
            Self::Moscow3 => ("3", "Арбатско-Покровская", [40, 80, 180]),
            Self::Moscow4 => ("4", "Филёвская", [60, 190, 225]),
            Self::Moscow5 => ("5", "Кольцевая", [145, 90, 55]),
            Self::Moscow6 => ("6", "Калужско-Рижская", [240, 135, 35]),
            Self::Moscow7 => ("7", "Таганско-Краснопресненская", [150, 55, 155]),
            Self::Moscow8 => ("8", "Калининская", [230, 190, 30]),
            Self::Moscow8A => ("8А", "Солнцевская", [230, 190, 30]),
            Self::Moscow9 => ("9", "Серпуховско-Тимирязевская", [145, 150, 155]),
            Self::Moscow10 => ("10", "Люблинско-Дмитровская", [145, 190, 45]),
            Self::Moscow11 => ("11", "Большая кольцевая", [65, 180, 175]),
            Self::Moscow12 => ("12", "Бутовская", [150, 190, 220]),
            Self::Moscow15 => ("15", "Некрасовская", [220, 105, 160]),
        };
        LineInfo {
            number,
            name,
            color,
        }
    }
}

struct LineInfo {
    number: &'static str,
    name: &'static str,
    color: [u8; 3],
}
