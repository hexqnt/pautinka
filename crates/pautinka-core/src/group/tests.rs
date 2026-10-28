use super::*;
use emath::pos2;

#[test]
fn enclosing_reserves_header_space_and_accepts_points() {
    let frame = GroupFrame::enclosing(
        [
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 40.0)),
            Rect::from_min_max(pos2(120.0, 60.0), pos2(120.0, 60.0)),
        ],
        Vec2::splat(10.0),
        20.0,
    )
    .unwrap();
    assert_eq!(
        frame.bounds(),
        Rect::from_min_max(pos2(-10.0, -30.0), pos2(130.0, 70.0))
    );
    assert_eq!(frame.header().height(), 20.0);
}

#[test]
fn invalid_external_geometry_is_rejected() {
    assert_eq!(
        GroupFrame::enclosing([], Vec2::ZERO, 20.0),
        Err(GroupFrameError::EmptyContent)
    );
    for size in [Vec2::ZERO, Vec2::splat(-1.0), Vec2::splat(f32::INFINITY)] {
        assert!(GroupFrame::new(Rect::from_min_size(emath::Pos2::ZERO, size), 20.0).is_err());
    }
    let bounds = Rect::from_min_size(emath::Pos2::ZERO, Vec2::splat(100.0));
    for height in [0.0, -1.0, 101.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            GroupFrame::new(bounds, height),
            Err(GroupFrameError::InvalidHeaderHeight)
        );
    }
    assert!(GroupFrame::new(bounds, 100.0).is_ok());
    assert_eq!(
        GroupFrame::enclosing([bounds], Vec2::splat(-1.0), 20.0),
        Err(GroupFrameError::InvalidPadding)
    );
    assert_eq!(
        GroupFrame::enclosing([Rect::EVERYTHING], Vec2::ZERO, 20.0),
        Err(GroupFrameError::InvalidBounds)
    );
    assert_eq!(
        GroupFrame::new(
            Rect::from_min_max(pos2(-f32::MAX, 0.0), pos2(f32::MAX, 100.0)),
            20.0
        ),
        Err(GroupFrameError::InvalidBounds)
    );
}
