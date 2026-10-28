use resvg::{tiny_skia, usvg};

use super::ExportError;

/// Проверенный коэффициент разрешения; не зависит от геометрии схемы.
#[derive(Clone, Copy)]
pub(super) struct RasterScale(f32);

impl TryFrom<f32> for RasterScale {
    type Error = ExportError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if !value.is_finite() || value <= 0.0 {
            return Err(ExportError::InvalidOptions);
        }
        Ok(Self(value))
    }
}

struct RasterSize {
    width: u32,
    height: u32,
}

impl RasterSize {
    const MAX_PIXELS: f64 = 64_000_000.0;

    fn new(width: f32, height: f32, scale: RasterScale) -> Result<Self, ExportError> {
        let width = (f64::from(width) * f64::from(scale.0)).ceil();
        let height = (f64::from(height) * f64::from(scale.0)).ceil();
        let dimensions = 1.0..=f64::from(u32::MAX);
        if !dimensions.contains(&width)
            || !dimensions.contains(&height)
            || width * height > Self::MAX_PIXELS
        {
            return Err(ExportError::InvalidOptions);
        }
        // Проверка диапазона выше исключает NaN, отрицательные значения и переполнение u32.
        #[allow(clippy::cast_sign_loss)]
        Ok(Self {
            width: width as u32,
            height: height as u32,
        })
    }
}

pub(super) fn render(svg: &str, scale: RasterScale) -> Result<Vec<u8>, ExportError> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|error| ExportError::Rasterization(error.to_string()))?;
    let size = RasterSize::new(tree.size().width(), tree.size().height(), scale)?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width, size.height).ok_or(ExportError::InvalidOptions)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale.0, scale.0),
        &mut pixmap.as_mut(),
    );
    pixmap
        .encode_png()
        .map_err(|error| ExportError::Rasterization(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_dimensions_round_up_and_reject_overflow_before_allocation() {
        let scale = RasterScale::try_from(1.5).unwrap();
        let size = RasterSize::new(3.0, 5.0, scale).unwrap();
        assert_eq!((size.width, size.height), (5, 8));
        for (width, height) in [
            (f32::NAN, 1.0),
            (f32::INFINITY, 1.0),
            (-1.0, 1.0),
            (0.0, 1.0),
            (f32::MAX, 1.0),
            (10_000.0, 10_000.0),
        ] {
            assert!(RasterSize::new(width, height, scale).is_err());
        }
        assert!(RasterSize::new(8_000.0, 8_000.0, RasterScale::try_from(1.0).unwrap()).is_ok());
    }
}
