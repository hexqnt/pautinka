use std::{borrow::Cow, collections::BTreeMap, fmt::Write as _};

use egui::{Color32, FontDefinitions, Vec2, epaint::TextShape};

use super::ExportError;

pub(super) struct Fonts<'a> {
    definitions: &'a FontDefinitions,
    faces: BTreeMap<&'a str, ttf_parser::Face<'a>>,
}

impl<'a> Fonts<'a> {
    pub(super) fn new(definitions: &'a FontDefinitions) -> Result<Self, ExportError> {
        let faces = definitions
            .font_data
            .iter()
            .map(|(name, data)| {
                ttf_parser::Face::parse(data.bytes(), data.index)
                    .map(|face| (name.as_str(), face))
                    .map_err(|_| ExportError::InvalidFonts)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self { definitions, faces })
    }
}

struct Outline<'a>(&'a mut String);
impl ttf_parser::OutlineBuilder for Outline<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        write!(self.0, "M{x} {y}").unwrap();
    }
    fn line_to(&mut self, x: f32, y: f32) {
        write!(self.0, "L{x} {y}").unwrap();
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        write!(self.0, "Q{x1} {y1} {x} {y}").unwrap();
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        write!(self.0, "C{x1} {y1} {x2} {y2} {x} {y}").unwrap();
    }
    fn close(&mut self) {
        self.0.push('Z');
    }
}

pub(super) fn paint(
    out: &mut String,
    text: &TextShape,
    fonts: &Fonts<'_>,
) -> Result<(), ExportError> {
    if !text.underline.is_empty() {
        return Err(ExportError::UnsupportedShape("text decorations"));
    }
    write!(
        out,
        "<g transform=\"translate({} {}) rotate({})\" opacity=\"{}\">",
        text.pos.x,
        text.pos.y,
        text.angle.to_degrees(),
        text.opacity_factor
    )
    .unwrap();
    let mut byte = 0;
    let mut sections = text.galley.job.sections.iter().peekable();
    for row in &text.galley.rows {
        for glyph in &row.glyphs {
            let index = egui::text::ByteIndex(byte);
            // Секции упорядочены и не пересекаются; каждый диапазон просматривается один раз.
            while sections
                .next_if(|section| section.byte_range.end <= index)
                .is_some()
            {}
            let format = &sections
                .peek()
                .filter(|section| section.byte_range.contains(&index))
                .ok_or(ExportError::InvalidFonts)?
                .format;
            byte += glyph.chr.len_utf8();
            if glyph.is_color
                || format.background != Color32::TRANSPARENT
                || !format.underline.is_empty()
                || !format.strikethrough.is_empty()
            {
                return Err(ExportError::UnsupportedShape(
                    "color glyphs or decorated text",
                ));
            }
            if glyph.chr.is_whitespace() {
                continue;
            }
            let names = fonts
                .definitions
                .families
                .get(&format.font_id.family)
                .ok_or(ExportError::InvalidFonts)?;
            let selected = names
                .iter()
                .find_map(|name| {
                    let data = fonts.definitions.font_data.get(name)?;
                    let face = fonts.faces.get(name.as_str())?;
                    let id = face.glyph_index(glyph.chr)?;
                    Some((data, face, id))
                })
                .ok_or(ExportError::UnsupportedShape("missing glyph outlines"))?;
            let (data, face, id) = selected;
            let mut face = Cow::Borrowed(face);
            for (tag, coordinate) in data
                .tweak
                .coords
                .as_ref()
                .iter()
                .chain(format.coords.as_ref())
            {
                face.to_mut()
                    .set_variation(ttf_parser::Tag::from_bytes(&tag.to_be_bytes()), *coordinate);
            }
            let scale = format.font_id.size * data.tweak.scale / f32::from(face.units_per_em());
            let position = row.pos
                + glyph.pos.to_vec2()
                + Vec2::new(
                    0.0,
                    format.font_id.size * data.tweak.scale * data.tweak.y_offset_factor
                        + data.tweak.y_offset,
                );
            write!(
                out,
                "<path transform=\"translate({} {}) scale({} {})\" d=\"",
                position.x, position.y, scale, -scale
            )
            .unwrap();
            face.outline_glyph(id, &mut Outline(out))
                .ok_or(ExportError::UnsupportedShape("missing glyph outlines"))?;
            out.push('"');
            let fill =
                text.override_text_color
                    .unwrap_or(if format.color == Color32::PLACEHOLDER {
                        text.fallback_color
                    } else {
                        format.color
                    });
            super::color(out, "fill", fill);
            out.push_str("/>");
        }
        if row.ends_with_newline {
            byte += 1;
        }
    }
    out.push_str("</g>");
    Ok(())
}
