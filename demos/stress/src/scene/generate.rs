use std::f64::consts::TAU;

use egui::{Pos2, Vec2};

use super::{Link, Load, Particle, Scene, Topology, route};

// Единица 8√3 задаёт шаг в пропорции 4:3 с прежней площадью ячейки 48².
const CELL_UNIT: f32 = 13.856_406;
const CELL_SPACING: Vec2 = Vec2::new(4.0 * CELL_UNIT, 3.0 * CELL_UNIT);

impl Scene {
    pub(super) fn generate(&mut self, load: Load) {
        self.nodes.truncate(load.nodes());
        self.nodes
            .reserve(load.nodes().saturating_sub(self.nodes.len()));
        for index in self.nodes.len()..load.nodes() {
            let random = hash(index as u32 ^ self.seed);
            let [x, y] = cell(index);
            let mut node = Particle {
                center: egui::pos2(x as f32 * CELL_SPACING.x, y as f32 * CELL_SPACING.y),
                position: Pos2::ZERO,
                phase: f64::from(random) / f64::from(u32::MAX) * TAU,
                frequency: 0.3 + f64::from(random % 100) * 0.006,
                amplitude: Vec2::new(7.0 + (random % 13) as f32, 7.0 + (random % 11) as f32),
                moving_rank: (hash(random) % 100) as u8,
                label: format!("Node {index}"),
            };
            node.position = node.center + node.orbit();
            self.nodes.push(node);
        }
        self.edges.clear();
        self.edges.reserve(load.edge_count());
        let chooser = Targets::new(self.seed, load, self.topology);
        for a in 0..load.nodes() {
            let grid = cell(a);
            let mut targets = [usize::MAX; Load::MAX_EDGES_PER_NODE as usize];
            for lane in 0..load.edge_range(a).len() {
                let b = chooser.choose(a, lane as u8, &targets[..lane], grid);
                targets[lane] = b;
                self.edges.push(Link {
                    ends: [a, b],
                    lane: lane as u8,
                    path: route(self.style, &self.nodes, [a, b], lane as u8),
                });
            }
        }
    }
}

struct Targets {
    seed: u32,
    nodes: usize,
    topology: Topology,
    hub_offset: usize,
    backbone: bool,
}

impl Targets {
    fn new(seed: u32, load: Load, topology: Topology) -> Self {
        Self {
            seed,
            nodes: load.nodes(),
            topology,
            hub_offset: hash(seed ^ 0x6a09_e667) as usize % load.nodes(),
            backbone: topology.backbone && load.supports_backbone(),
        }
    }

    fn hub(&self, index: usize) -> usize {
        (self.hub_offset + index * self.nodes / usize::from(self.topology.hubs)) % self.nodes
    }

    fn choose(&self, source: usize, lane: u8, previous: &[usize], [x, y]: [i32; 2]) -> usize {
        if self.backbone && lane == 0 && source > 0 {
            return source - 1;
        }
        let random = hash(self.seed ^ (source as u32 * 257) ^ (u32::from(lane) * 67));
        let prefer_hub = random % 100 < u32::from(self.topology.hub_share);
        let prefer_local = hash(random) % 100 < u32::from(self.topology.locality);
        let radius = u32::from(self.topology.local_radius);
        let span = radius * 2 + 1;
        let available =
            |target| target < self.nodes && target != source && !previous.contains(&target);
        for attempt in 0..32_u32 {
            let choice = hash(random ^ attempt.wrapping_mul(0x9e37_79b9));
            let target = if prefer_hub {
                self.hub(choice as usize % usize::from(self.topology.hubs))
            } else if prefer_local {
                cell_index([
                    x + (choice % span).cast_signed() - radius.cast_signed(),
                    y + ((choice / span) % span).cast_signed() - radius.cast_signed(),
                ])
            } else {
                choice as usize % self.nodes
            };
            if available(target) {
                return target;
            }
        }
        // Если выбранная окрестность исчерпана, добираем дальнюю связь без петель и повторов.
        let mut target = hash(random ^ 0xbb67_ae85) as usize % self.nodes;
        while !available(target) {
            target = (target + 1) % self.nodes;
        }
        target
    }
}

fn hash(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

// Спираль обходит целочисленные ячейки; CELL_SPACING переводит её в прямоугольную область. Индексы не зависят от размера графа, обратное преобразование для поиска соседей занимает O(1).
fn cell(index: usize) -> [i32; 2] {
    if index == 0 {
        return [0, 0];
    }
    let ring = i32::try_from(index.isqrt().div_ceil(2)).unwrap();
    let side = ring * 2;
    let offset = i32::try_from(index).unwrap() - (side - 1).pow(2);
    match offset / side {
        0 => [ring, -ring + 1 + offset],
        1 => [ring - 1 - offset % side, ring],
        2 => [-ring, ring - 1 - offset % side],
        _ => [-ring + 1 + offset % side, -ring],
    }
}
fn cell_index([x, y]: [i32; 2]) -> usize {
    let ring = x.abs().max(y.abs());
    if ring == 0 {
        return 0;
    }
    let side = ring * 2;
    let offset = if x == ring && y > -ring {
        y + ring - 1
    } else if y == ring {
        side + ring - 1 - x
    } else if x == -ring {
        side * 2 + ring - 1 - y
    } else {
        side * 3 + x + ring - 1
    };
    usize::try_from((side - 1).pow(2) + offset).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spiral_positions_have_an_exact_inverse_for_all_supported_nodes() {
        for index in 0..=*Load::NODES.end() {
            assert_eq!(cell_index(cell(index)), index);
        }
    }

    #[test]
    fn locality_and_hub_preferences_change_the_actual_distribution() {
        let load = Load::new(1_000, 2).unwrap();
        let measure = |topology| {
            let chooser = Targets::new(42, load, topology);
            let mut local = 0;
            let mut hub = 0;
            for source in 0..load.nodes() {
                let origin = cell(source);
                let mut previous = [usize::MAX; 2];
                for lane in 0..2 {
                    let target = chooser.choose(source, lane as u8, &previous[..lane], origin);
                    previous[lane] = target;
                    let position = cell(target);
                    if (origin[0] - position[0])
                        .abs()
                        .max((origin[1] - position[1]).abs())
                        <= 2
                    {
                        local += 1;
                    }
                    if (0..usize::from(topology.hubs)).any(|index| chooser.hub(index) == target) {
                        hub += 1;
                    }
                }
            }
            (local, hub)
        };
        let local = measure(Topology::new(100, 2, 0, 4, false).unwrap());
        let random = measure(Topology::RANDOM);
        let hubs = measure(Topology::new(0, 2, 100, 4, false).unwrap());
        assert!(local.0 > 1_900);
        assert!(random.0 < 150);
        assert!(hubs.1 > 1_900);
        assert!(random.1 < 50);
    }
}
