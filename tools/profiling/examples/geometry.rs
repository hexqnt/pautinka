use std::{
    env,
    error::Error,
    hint::black_box,
    num::{NonZeroU32, NonZeroUsize},
    time::Instant,
};

use pautinka_core::{EdgePath, HitPoint, Pos2, Vec2, pos2};

fn measure(name: &str, iterations: NonZeroU32, samples: NonZeroUsize, mut operation: impl FnMut()) {
    for _ in 0..iterations.get().min(1024) {
        operation();
    }
    let mut times = Vec::with_capacity(samples.get());
    for _ in 0..samples.get() {
        let start = Instant::now();
        for _ in 0..iterations.get() {
            operation();
        }
        times.push(start.elapsed().as_secs_f64() * 1e9 / f64::from(iterations.get()));
    }
    times.sort_unstable_by(f64::total_cmp);
    println!(
        "{name},{iterations},{samples},{:.3},{:.3}",
        times[(times.len() - 1) / 2],
        times[(times.len() * 95).div_ceil(100) - 1],
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let iterations = args.next().map_or(Ok(100_000), |arg| arg.parse())?;
    let samples = args.next().map_or(Ok(5), |arg| arg.parse())?;
    if args.next().is_some() {
        return Err("Expected at most two arguments: iterations and samples".into());
    }
    let iterations = NonZeroU32::new(iterations).ok_or("Iterations must be positive")?;
    let samples = NonZeroUsize::new(samples).ok_or("Samples must be positive")?;
    let controls: [[Pos2; 4]; 16] = std::array::from_fn(|i| {
        let shift = Vec2::new(i as f32 * 17.0, i as f32 * -9.0);
        [
            pos2(0.0, 0.0),
            pos2(20.0, 170.0),
            pos2(-140.0, -280.0),
            pos2(175.0, 90.0),
        ]
        .map(|point| point + shift)
    });
    println!("scenario,iterations,samples,p50_ns_per_operation,p95_ns_per_operation");
    for cubic in [false, true] {
        let mut curves = controls.iter().cycle();
        measure(
            if cubic {
                "build_cubic"
            } else {
                "build_quadratic"
            },
            iterations,
            samples,
            || {
                let &[a, b, c, d] = black_box(curves.next().unwrap());
                black_box(if cubic {
                    EdgePath::try_cubic([a, b, c, d])
                } else {
                    EdgePath::try_quadratic([a, b, d])
                })
                .unwrap();
            },
        );
    }
    for (name, path) in [
        (
            "pick_line",
            EdgePath::try_line(controls[0][0], controls[0][3])?,
        ),
        (
            "pick_quadratic_24_segments",
            EdgePath::try_quadratic([pos2(0.0, 0.0), pos2(100.0, 200.0), pos2(200.0, 0.0)])?,
        ),
        ("pick_cubic_6_segments", EdgePath::try_cubic(controls[0])?),
    ] {
        let queries = [0.0, 0.2, 0.5, 0.8, 1.0].map(|fraction| {
            let position = path.at_distance(path.length() * fraction).unwrap().position;
            HitPoint::from_world(position + Vec2::Y * 0.125).unwrap()
        });
        let mut queries = queries.iter().cycle();
        measure(name, iterations, samples, || {
            black_box(path.hit_distance_sq_precise(black_box(*queries.next().unwrap()), 0.35));
        });
    }
    Ok(())
}
