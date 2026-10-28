use std::{error::Error, fmt};

#[derive(Debug)]
#[non_exhaustive]
pub enum ExportError {
    InvalidOptions,
    InvalidFonts,
    InvalidGeometry,
    Spatial(crate::SpatialError),
    UnsupportedShape(&'static str),
    #[cfg(feature = "png")]
    Rasterization(String),
}

impl Error for ExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spatial(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::SpatialError> for ExportError {
    fn from(error: crate::SpatialError) -> Self {
        Self::Spatial(error)
    }
}
impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions => f.write_str(
                "Export scale, region, padding, label layout or pixel dimensions are invalid",
            ),
            Self::InvalidFonts => {
                f.write_str("Export fonts must contain valid faces and family references")
            }
            Self::InvalidGeometry => {
                f.write_str("Export geometry must be finite and representable")
            }
            Self::Spatial(error) => error.fmt(f),
            Self::UnsupportedShape(kind) => write!(f, "SVG export does not support {kind}"),
            #[cfg(feature = "png")]
            Self::Rasterization(message) => write!(f, "PNG export failed: {message}"),
        }
    }
}
