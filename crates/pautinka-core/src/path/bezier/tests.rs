use emath::pos2;

use super::*;

fn reference_position(controls: &[Pos2], t: f32) -> Pos2 {
    let mut work = [Pos2::ZERO; 4];
    work[..controls.len()].copy_from_slice(controls);
    for count in (1..controls.len()).rev() {
        for i in 0..count {
            work[i] = work[i].lerp(work[i + 1], t);
        }
    }
    work[0]
}

fn bits(point: Pos2) -> [u32; 2] {
    [point.x.to_bits(), point.y.to_bits()]
}

#[test]
fn interpolation_preserves_de_casteljau_rounding_at_all_scales() {
    let subnormal = f32::from_bits(1);
    let controls = [
        [
            pos2(-0.0, 0.0),
            pos2(0.0, -0.0),
            pos2(-0.0, -0.0),
            pos2(0.0, 0.0),
        ],
        [
            pos2(subnormal, -subnormal),
            pos2(-subnormal * 3.0, subnormal * 8.0),
            pos2(subnormal * 16.0, -subnormal * 5.0),
            pos2(-subnormal * 7.0, subnormal * 2.0),
        ],
        [
            pos2(-f32::MAX, f32::MAX),
            pos2(f32::MAX, -f32::MAX),
            pos2(-f32::MAX, -f32::MAX),
            pos2(f32::MAX, f32::MAX),
        ],
    ];
    let scaled = [1e-25, 0.0001, 1.0, 1000.0, 1e25].map(|scale| {
        [
            pos2(-20.0, 10.0),
            pos2(100.0, 400.0),
            pos2(700.0, -200.0),
            pos2(1000.0, 10.0),
        ]
        .map(|point| point * scale)
    });
    for controls in controls.into_iter().chain(scaled) {
        for controls in [&controls[..3], &controls[..]] {
            for t in [
                -0.0,
                0.0,
                subnormal,
                f32::MIN_POSITIVE,
                0.25,
                (1.0_f32 / 3.0).next_down(),
                1.0 / 3.0,
                (1.0_f32 / 3.0).next_up(),
                0.5,
                1.0_f32.next_down(),
                1.0,
            ] {
                let expected = bits(reference_position(controls, t));
                assert_eq!(bits(position(controls, t)), expected, "{controls:?} {t}");
                assert_eq!(bits(sample(controls, t).position), expected);
            }
            for (i, point) in positions(controls).into_iter().enumerate() {
                let t = i as f32 / (SAMPLES - 1) as f32;
                assert_eq!(bits(point), bits(reference_position(controls, t)));
            }
        }
    }
}
