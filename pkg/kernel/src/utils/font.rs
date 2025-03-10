use embedded_graphics::{
    geometry::Size,
    image::ImageRaw,
    mono_font::{DecorationDimensions, MonoFont, mapping::ASCII},
};

pub const JBMONO: MonoFont = MonoFont {
    image: ImageRaw::new(include_bytes!("../../assets/JBMONO.raw"), 15 * 16),
    glyph_mapping: &ASCII,
    character_size: Size::new(15, 30),
    character_spacing: 0,
    baseline: 30,
    underline: DecorationDimensions::new(32, 2),
    strikethrough: DecorationDimensions::new(16, 2),
};

pub const JBMONO_TITLE: MonoFont = MonoFont {
    image: ImageRaw::new(include_bytes!("../../assets/JBMONO_TITLE.raw"), 29 * 16),
    glyph_mapping: &ASCII,
    character_size: Size::new(29, 56),
    character_spacing: 0,
    baseline: 42,
    underline: DecorationDimensions::new(42, 3),
    strikethrough: DecorationDimensions::new(25, 3),
};
