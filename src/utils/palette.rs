use color_thief::ColorFormat;

#[derive(Debug, Clone)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn extract_palette(bytes: &[u8]) -> Option<Vec<PaletteColor>> {
    let palette = color_thief::get_palette(bytes, ColorFormat::Rgb, 5, 4).ok()?;
    Some(
        palette
            .iter()
            .map(|c| PaletteColor {
                r: c.r,
                g: c.g,
                b: c.b,
            })
            .collect(),
    )
}
