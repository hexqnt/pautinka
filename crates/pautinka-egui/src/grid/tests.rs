use super::*;

#[test]
fn options_encode_finite_positive_steps_and_sizes() {
    for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0] {
        assert!(GridOptions::new(invalid, 16.0).is_none());
        assert!(GridOptions::new(10.0, invalid).is_none());
        assert!(GridOptions::default().with_widths(invalid, 1.0).is_none());
        assert!(GridOptions::default().with_widths(1.0, invalid).is_none());
    }
    assert!(GridOptions::new(10.0, 7.99).is_none());
    assert!(GridOptions::new(f32::from_bits(1), 8.0).is_some());
    assert!(GridOptions::new(f32::MAX, f32::MAX).is_some());
}

#[test]
fn zoom_selects_nested_world_steps_with_bounded_density_at_extreme_scales() {
    for kind in [GridKind::Lines, GridKind::Dots] {
        let mut options = GridOptions {
            kind,
            ..GridOptions::default()
        };
        for size in [800.0, 100_000.0] {
            let clip = Rect::from_min_max(pos2(70.0, 30.0), pos2(size, size));
            for base in [f32::from_bits(1), 1e-25, 10.0, 1e25, f32::MAX] {
                options.base_step = base;
                for zoom in [f32::from_bits(1), 1e-25, 0.01, 1.0, 1e25, f32::MAX] {
                    let level = GridLevel::new(&options, zoom, clip);
                    assert!(level.world_step.is_finite() && level.world_step > 0.0);
                    assert!(level.screen_step >= f64::from(options.min_spacing));
                    assert!((0.0..=1.0).contains(&level.fade));
                    let exponent = (level.world_step / f64::from(base)).log(5.0);
                    assert!((exponent - exponent.round()).abs() < 1e-10);
                    for center in [0.0, -123.5, 1e30, -1e30] {
                        let axis = GridAxis::new(
                            clip.left(),
                            clip.right(),
                            clip.center().x,
                            center,
                            level,
                        );
                        assert!(axis.count <= 513);
                        if kind == GridKind::Dots {
                            assert!(axis.count * axis.count <= 17_000);
                        }
                        for (screen, _) in axis.marks() {
                            assert!(screen.is_finite());
                            assert!(screen >= clip.left() - 0.01 && screen <= clip.right() + 0.01);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn grid_is_anchored_at_world_zero_across_pans_and_negative_coordinates() {
    let options = GridOptions::new(10.0, 10.0).unwrap();
    let clip = Rect::from_min_max(pos2(20.0, 10.0), pos2(820.0, 610.0));
    let level = GridLevel::new(&options, 2.0, clip);
    for world_center in [-1234.5, -1250.0, -25.0, 0.0, 25.0, 1250.0] {
        let axis = GridAxis::new(
            clip.left(),
            clip.right(),
            clip.center().x,
            world_center,
            level,
        );
        for (screen, rank) in axis.marks() {
            let world = f64::from(world_center) + f64::from(screen - clip.center().x) / 2.0;
            let index = world / level.world_step;
            assert!((index - index.round()).abs() < 1e-5);
            assert_eq!(
                rank == GridRank::Major,
                index.round().rem_euclid(25.0) == 0.0
            );
            assert_eq!(
                rank != GridRank::Minor,
                index.round().rem_euclid(5.0) == 0.0
            );
        }
    }
}

#[test]
fn level_boundary_preserves_the_visible_lattice() {
    let options = GridOptions::new(10.0, 10.0).unwrap();
    let clip = Rect::from_min_max(pos2(-400.0, -300.0), pos2(400.0, 300.0));
    let fine = GridLevel::new(&options, 1.0001, clip);
    let coarse = GridLevel::new(&options, 0.9999, clip);
    assert!(fine.fade < 0.001);
    assert!(coarse.fade > 0.999);
    assert!((coarse.world_step / fine.world_step - 5.0).abs() < 1e-10);
    let fine_axis = GridAxis::new(-400.0, 400.0, 0.0, -123.0, fine);
    let coarse_axis = GridAxis::new(-400.0, 400.0, 0.0, -123.0, coarse);
    let mut fine_visible = fine_axis
        .marks()
        .filter(|(_, rank)| *rank != GridRank::Minor);
    let fine_strokes = GridStrokes::new(&options, fine.fade);
    let coarse_strokes = GridStrokes::new(&options, coarse.fade);
    for (coarse, coarse_rank) in coarse_axis.marks() {
        let (fine, fine_rank) = fine_visible.next().expect("Matching grid division");
        assert!((fine - coarse).abs() < 0.1);
        assert_eq!(fine_rank == GridRank::Major, coarse_rank != GridRank::Minor);
        let fine_stroke = fine_strokes.get(fine_rank);
        let coarse_stroke = coarse_strokes.get(coarse_rank);
        assert!((fine_stroke.width - coarse_stroke.width).abs() < 0.001);
        assert!(
            fine_stroke
                .color
                .to_array()
                .into_iter()
                .zip(coarse_stroke.color.to_array())
                .all(|(fine, coarse)| fine.abs_diff(coarse) <= 1)
        );
    }
    assert!(fine_visible.next().is_none());
}
