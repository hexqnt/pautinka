use std::cmp::Reverse;

use egui::{Key, Modifiers, RichText};
use nucleo_matcher::{
    Config, Matcher, Utf32String,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

use crate::i18n::{Language, Text};
use crate::metro::{Metro, StationId};

const RESULT_LIMIT: usize = 8;

struct Entry {
    text: Utf32String,
    // Обе локализации готовятся при загрузке, без форматирования строк в кадре.
    labels: [String; 2],
}

pub(super) struct StationSearch {
    query: String,
    matcher: Matcher,
    entries: Vec<Entry>,
    matches: Vec<(StationId, u32)>,
    active: usize,
    open: bool,
}

impl StationSearch {
    pub(super) fn new(metro: &Metro) -> Self {
        Self {
            query: String::new(),
            matcher: Matcher::new(Config::DEFAULT),
            entries: metro
                .stations
                .iter()
                .map(|station| {
                    let duplicate = metro
                        .stations
                        .iter()
                        .any(|other| other.name == station.name && other.line != station.line);
                    Entry {
                        text: station.name.as_str().into(),
                        labels: Language::ALL.map(|language| {
                            if duplicate {
                                format!(
                                    "→ {} · {} {}",
                                    station.name,
                                    language.text(Text::LineWord),
                                    station.line.number()
                                )
                            } else {
                                format!("→ {}", station.name)
                            }
                        }),
                    }
                })
                .collect(),
            matches: Vec::with_capacity(metro.stations.len()),
            active: 0,
            open: false,
        }
    }

    fn update(&mut self) {
        self.matches.clear();
        self.active = 0;
        let query = self.query.trim();
        if query.is_empty() {
            return;
        }
        // Строки UTF-32 и память matcher переиспользуются; пересчёт выполняется только при изменении запроса.
        let pattern = Pattern::new(
            query,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        self.matches.extend(
            self.entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    pattern
                        .score(entry.text.slice(..), &mut self.matcher)
                        .map(|score| (StationId(index), score))
                }),
        );
        let rank = |&(id, score): &(StationId, u32)| (Reverse(score), id);
        if self.matches.len() > RESULT_LIMIT {
            self.matches.select_nth_unstable_by_key(RESULT_LIMIT, rank);
            self.matches.truncate(RESULT_LIMIT);
        }
        self.matches.sort_unstable_by_key(rank);
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui, metro: &Metro) -> Option<StationId> {
        let language = metro.language;
        let id = ui.make_persistent_id("station-search");
        let focused = ui.memory(|memory| memory.has_focus(id));
        let key = focused
            .then(|| {
                ui.input_mut(|input| {
                    [Key::ArrowDown, Key::ArrowUp, Key::Enter, Key::Escape]
                        .into_iter()
                        .find(|&key| input.consume_key(Modifiers::NONE, key))
                })
            })
            .flatten();
        ui.add(
            egui::Image::new(egui::include_image!("../../assets/search.svg"))
                .fit_to_exact_size(egui::Vec2::splat(20.0))
                .tint(ui.visuals().text_color())
                .alt_text(language.text(Text::Search)),
        );
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .id(id)
                .margin(egui::Margin::symmetric(4, 2))
                .event_filter(egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                    ..Default::default()
                })
                .hint_text(language.text(Text::SearchHint))
                .desired_width(f32::INFINITY),
        );
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_label(language.text(Text::StationSearch));
        });
        if response.changed() {
            self.update();
        }
        if response.changed() || response.clicked() || response.gained_focus() {
            self.open = !self.query.trim().is_empty();
        }
        let mut chosen = self.navigate(key);
        let mut open = self.open;
        egui::Popup::from_response(&response)
            .id(id.with("results"))
            .open_bool(&mut open)
            .close_behavior(if response.clicked() {
                egui::PopupCloseBehavior::IgnoreClicks
            } else {
                egui::PopupCloseBehavior::CloseOnClickOutside
            })
            .width(response.rect.width())
            .show(|ui| {
                ui.set_min_width(response.rect.width() - 16.0);
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        if self.matches.is_empty() {
                            ui.weak(language.text(Text::NoStation));
                        }
                        for (index, &(station, _)) in self.matches.iter().enumerate() {
                            let row = ui.selectable_label(
                                index == self.active,
                                RichText::new(&self.entries[station.0].labels[language.index()])
                                    .color(metro.stations[station.0].line.color()),
                            );
                            if key.is_some() && index == self.active {
                                row.scroll_to_me(None);
                            }
                            if row.clicked() {
                                chosen = Some(station);
                            }
                        }
                    });
            });
        self.open = open && !response.lost_focus();
        if let Some(station) = chosen {
            self.query.clear();
            self.query.push_str(&metro.stations[station.0].name);
            self.update();
            self.open = false;
            response.surrender_focus();
        }
        chosen
    }

    fn navigate(&mut self, key: Option<Key>) -> Option<StationId> {
        match key {
            Some(Key::Escape) => self.open = false,
            Some(Key::ArrowDown | Key::ArrowUp) if !self.matches.is_empty() => {
                if self.open {
                    self.active = if key == Some(Key::ArrowDown) {
                        (self.active + 1) % self.matches.len()
                    } else {
                        (self.active + self.matches.len() - 1) % self.matches.len()
                    };
                }
                self.open = true;
            }
            Some(Key::Enter) if self.open => {
                return self.matches.get(self.active).map(|&(id, _)| id);
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_search_matches_cyrillic_abbreviations_words_and_case() {
        let metro = Metro::embedded();
        let mut search = StationSearch::new(&metro);
        for (query, expected) in [
            ("бгв", "Беговая"),
            ("ЗЕНИТ", "Зенит"),
            ("нв пр", "Невский проспект"),
        ] {
            search.query = query.into();
            search.update();
            assert!(
                search
                    .matches
                    .iter()
                    .any(|&(id, _)| metro.stations[id.0].name == expected),
                "{query}"
            );
            assert!(search.matches.windows(2).all(|pair| pair[0].1 >= pair[1].1));
        }
    }

    #[test]
    fn exact_match_ranks_first_and_empty_queries_have_no_results() {
        let metro = Metro::embedded();
        let mut search = StationSearch::new(&metro);
        search.query = "  Зенит  ".into();
        search.update();
        let (station, _) = search.matches[0];
        assert_eq!(metro.stations[station.0].name, "Зенит");
        for query in ["", "   ", "неттакойстанции"] {
            search.query = query.into();
            search.update();
            assert_eq!(search.matches, []);
        }
    }

    #[test]
    fn limited_results_match_full_ranking_including_tied_scores() {
        let metro = Metro::for_city(crate::metro::City::Moscow);
        let mut search = StationSearch::new(&metro);
        let mut matcher = Matcher::new(Config::DEFAULT);
        for query in ["а", "о", "п"] {
            let pattern = Pattern::new(
                query,
                CaseMatching::Ignore,
                Normalization::Smart,
                AtomKind::Fuzzy,
            );
            let mut expected: Vec<_> = search
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    pattern
                        .score(entry.text.slice(..), &mut matcher)
                        .map(|score| (StationId(index), score))
                })
                .collect();
            assert!(expected.len() > RESULT_LIMIT);
            expected.sort_unstable_by_key(|&(id, score)| (Reverse(score), id));
            expected.truncate(RESULT_LIMIT);

            search.query = query.into();
            search.update();
            assert_eq!(search.matches, expected, "{query}");
        }

        for entry in &mut search.entries {
            entry.text = "Парк".into();
        }
        search.query = "п".into();
        search.update();
        assert_eq!(
            search.matches.iter().map(|&(id, _)| id).collect::<Vec<_>>(),
            (0..RESULT_LIMIT).map(StationId).collect::<Vec<_>>()
        );
    }
}
