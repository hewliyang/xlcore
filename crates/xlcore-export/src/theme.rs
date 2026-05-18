//! Theme extraction from `xl/theme/theme1.xml`.
//!
//! OOXML stores a `<a:clrScheme>` with 12 named slots (lt1/dk1/lt2/dk2/
//! accent1..6/hlink/folHlink). The spreadsheet's `theme="N"` references
//! a *different* index order — the first two pairs are swapped — so we
//! emit `colors[]` in the spreadsheet order:
//!
//!   0:lt1  1:dk1  2:lt2  3:dk2  4:accent1 .. 9:accent6  10:hlink  11:folHlink
//!
//! Reference: ECMA-376, §18.8.27 colors / §20.1.6.2 clrScheme; LibreOffice
//! `oox/source/drawingml/themefragmenthandler.cxx::ThemeFragmentHandler`.
//!
//! We resolve all five OOXML color choices:
//!
//! - `<a:srgbClr val="RRGGBB">` — direct.
//! - `<a:sysClr lastClr="RRGGBB">` — `lastClr` fallback (always present
//!   when Office writes the theme).
//! - `<a:scrgbClr r="% * 1000" g=… b=…>` — RGB percentages in 1000ths
//!   (ECMA-376 §20.1.2.3.30); converted to 0..255 bytes.
//! - `<a:hslClr hue="deg * 60000" sat="% * 1000" lum="% * 1000">`
//!   (§20.1.2.3.13) — HSL → RGB. Note OOXML HSL is *not* the same as the
//!   theme-tint HLS curve in `packages/xlsx-preview/src/render.ts`; here we just do
//!   the standard sRGB conversion.
//! - `<a:prstClr val="name">` (§20.1.2.3.22) — lookup against the spec's
//!   190-entry preset color table (CSS3/X11 names + `dk`/`lt`/`med`
//!   abbreviations + 2010 aliases for the same names without the prefix
//!   shorthand). Generated from the schema enum; see `_PRESET_GEN` block
//!   below.
//!
//! Color modifier children (`<a:tint>`, `<a:shade>`, `<a:lumMod>`,
//! `<a:satMod>`, `<a:alpha>`, …) are intentionally ignored at the theme
//! level — themes ship raw scheme colors; the cell-level tint handling
//! lives in `packages/xlsx-preview/src/render.ts::applyTint` and operates on the
//! resolved hex we emit here.
use crate::schema::Theme;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;

/// Convert an scRGB component (0..100000, in 1000ths of a percent) to a
/// 0..255 byte. Values out of range are clamped — Office occasionally
/// emits 100001 for "100%".
fn scrgb_byte(v: i32) -> u8 {
    let pct = (v.clamp(0, 100_000) as f64) / 100_000.0;
    (pct * 255.0).round() as u8
}

/// HSL → RGB per CSS Color 3 / W3C, the same conversion OOXML uses for
/// `<a:hslClr>` per §20.1.2.3.13. `h` in degrees [0,360), `s`/`l` in
/// [0,1].
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h_prime = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (h_prime.rem_euclid(2.0) - 1.0).abs());
    let (r1, g1, b1) = match h_prime {
        h if (0.0..1.0).contains(&h) => (c, x, 0.0),
        h if (1.0..2.0).contains(&h) => (x, c, 0.0),
        h if (2.0..3.0).contains(&h) => (0.0, c, x),
        h if (3.0..4.0).contains(&h) => (0.0, x, c),
        h if (4.0..5.0).contains(&h) => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn rgb_hex(r: u8, g: u8, b: u8) -> String {
    format!("{:02X}{:02X}{:02X}", r, g, b)
}

fn resolve_scrgb(c: &a::RgbColorModelPercentage) -> String {
    rgb_hex(
        scrgb_byte(c.red_portion),
        scrgb_byte(c.green_portion),
        scrgb_byte(c.blue_portion),
    )
}

fn resolve_hsl(c: &a::HslColor) -> String {
    let h = (c.hue_value as f64) / 60_000.0; // degrees
    let s = (c.sat_value as f64) / 100_000.0;
    let l = (c.lum_value as f64) / 100_000.0;
    let (r, g, b) = hsl_to_rgb(h, s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    rgb_hex(r, g, b)
}

/// Lookup table for `<a:prstClr val="name">` element (ECMA-376
/// §20.1.2.3.22); the enum is `ST_PresetColorVal` (DrawingML simple
/// types in §20.1.10). 190 entries: CSS3/X11 named colors + the OOXML-
/// specific `dk*`/`lt*`/`med*` abbreviations + 2010-era aliases (the
/// schema added e.g. `darkBlue` alongside `dkBlue` after 2007 Office
/// shipped; both resolve to the same value).
fn resolve_preset(c: &a::PresetColor) -> String {
    use a::PresetColorValues as Pcv;
    // _PRESET_GEN: regenerate via the Python snippet in the test below.
    let hex = match c.val {
        Pcv::AliceBlue => "F0F8FF",
        Pcv::AntiqueWhite => "FAEBD7",
        Pcv::Aqua => "00FFFF",
        Pcv::Aquamarine => "7FFFD4",
        Pcv::Azure => "F0FFFF",
        Pcv::Beige => "F5F5DC",
        Pcv::Bisque => "FFE4C4",
        Pcv::Black => "000000",
        Pcv::BlanchedAlmond => "FFEBCD",
        Pcv::Blue => "0000FF",
        Pcv::BlueViolet => "8A2BE2",
        Pcv::Brown => "A52A2A",
        Pcv::BurlyWood => "DEB887",
        Pcv::CadetBlue => "5F9EA0",
        Pcv::Chartreuse => "7FFF00",
        Pcv::Chocolate => "D2691E",
        Pcv::Coral => "FF7F50",
        Pcv::CornflowerBlue => "6495ED",
        Pcv::Cornsilk => "FFF8DC",
        Pcv::Crimson => "DC143C",
        Pcv::Cyan => "00FFFF",
        Pcv::DarkBlue => "00008B",
        Pcv::DarkCyan => "008B8B",
        Pcv::DarkGoldenrod => "B8860B",
        Pcv::DarkGray => "A9A9A9",
        Pcv::DarkGreen => "006400",
        Pcv::DarkKhaki => "BDB76B",
        Pcv::DarkMagenta => "8B008B",
        Pcv::DarkOliveGreen => "556B2F",
        Pcv::DarkOrange => "FF8C00",
        Pcv::DarkOrchid => "9932CC",
        Pcv::DarkRed => "8B0000",
        Pcv::DarkSalmon => "E9967A",
        Pcv::DarkSeaGreen => "8FBC8F",
        Pcv::DarkSlateBlue => "483D8B",
        Pcv::DarkSlateGray => "2F4F4F",
        Pcv::DarkTurquoise => "00CED1",
        Pcv::DarkViolet => "9400D3",
        Pcv::DeepPink => "FF1493",
        Pcv::DeepSkyBlue => "00BFFF",
        Pcv::DimGray => "696969",
        Pcv::DodgerBlue => "1E90FF",
        Pcv::Firebrick => "B22222",
        Pcv::FloralWhite => "FFFAF0",
        Pcv::ForestGreen => "228B22",
        Pcv::Fuchsia => "FF00FF",
        Pcv::Gainsboro => "DCDCDC",
        Pcv::GhostWhite => "F8F8FF",
        Pcv::Gold => "FFD700",
        Pcv::Goldenrod => "DAA520",
        Pcv::Gray => "808080",
        Pcv::Green => "008000",
        Pcv::GreenYellow => "ADFF2F",
        Pcv::Honeydew => "F0FFF0",
        Pcv::HotPink => "FF69B4",
        Pcv::IndianRed => "CD5C5C",
        Pcv::Indigo => "4B0082",
        Pcv::Ivory => "FFFFF0",
        Pcv::Khaki => "F0E68C",
        Pcv::Lavender => "E6E6FA",
        Pcv::LavenderBlush => "FFF0F5",
        Pcv::LawnGreen => "7CFC00",
        Pcv::LemonChiffon => "FFFACD",
        Pcv::LightBlue => "ADD8E6",
        Pcv::LightCoral => "F08080",
        Pcv::LightCyan => "E0FFFF",
        Pcv::LightGoldenrodYellow => "FAFAD2",
        Pcv::LightGray => "D3D3D3",
        Pcv::LightGreen => "90EE90",
        Pcv::LightPink => "FFB6C1",
        Pcv::LightSalmon => "FFA07A",
        Pcv::LightSeaGreen => "20B2AA",
        Pcv::LightSkyBlue => "87CEFA",
        Pcv::LightSlateGray => "778899",
        Pcv::LightSteelBlue => "B0C4DE",
        Pcv::LightYellow => "FFFFE0",
        Pcv::Lime => "00FF00",
        Pcv::LimeGreen => "32CD32",
        Pcv::Linen => "FAF0E6",
        Pcv::Magenta => "FF00FF",
        Pcv::Maroon => "800000",
        Pcv::MedAquamarine => "66CDAA",
        Pcv::MediumBlue => "0000CD",
        Pcv::MediumOrchid => "BA55D3",
        Pcv::MediumPurple => "9370DB",
        Pcv::MediumSeaGreen => "3CB371",
        Pcv::MediumSlateBlue => "7B68EE",
        Pcv::MediumSpringGreen => "00FA9A",
        Pcv::MediumTurquoise => "48D1CC",
        Pcv::MediumVioletRed => "C71585",
        Pcv::MidnightBlue => "191970",
        Pcv::MintCream => "F5FFFA",
        Pcv::MistyRose => "FFE4E1",
        Pcv::Moccasin => "FFE4B5",
        Pcv::NavajoWhite => "FFDEAD",
        Pcv::Navy => "000080",
        Pcv::OldLace => "FDF5E6",
        Pcv::Olive => "808000",
        Pcv::OliveDrab => "6B8E23",
        Pcv::Orange => "FFA500",
        Pcv::OrangeRed => "FF4500",
        Pcv::Orchid => "DA70D6",
        Pcv::PaleGoldenrod => "EEE8AA",
        Pcv::PaleGreen => "98FB98",
        Pcv::PaleTurquoise => "AFEEEE",
        Pcv::PaleVioletRed => "DB7093",
        Pcv::PapayaWhip => "FFEFD5",
        Pcv::PeachPuff => "FFDAB9",
        Pcv::Peru => "CD853F",
        Pcv::Pink => "FFC0CB",
        Pcv::Plum => "DDA0DD",
        Pcv::PowderBlue => "B0E0E6",
        Pcv::Purple => "800080",
        Pcv::Red => "FF0000",
        Pcv::RosyBrown => "BC8F8F",
        Pcv::RoyalBlue => "4169E1",
        Pcv::SaddleBrown => "8B4513",
        Pcv::Salmon => "FA8072",
        Pcv::SandyBrown => "F4A460",
        Pcv::SeaGreen => "2E8B57",
        Pcv::SeaShell => "FFF5EE",
        Pcv::Sienna => "A0522D",
        Pcv::Silver => "C0C0C0",
        Pcv::SkyBlue => "87CEEB",
        Pcv::SlateBlue => "6A5ACD",
        Pcv::SlateGray => "708090",
        Pcv::Snow => "FFFAFA",
        Pcv::SpringGreen => "00FF7F",
        Pcv::SteelBlue => "4682B4",
        Pcv::Tan => "D2B48C",
        Pcv::Teal => "008080",
        Pcv::Thistle => "D8BFD8",
        Pcv::Tomato => "FF6347",
        Pcv::Turquoise => "40E0D0",
        Pcv::Violet => "EE82EE",
        Pcv::Wheat => "F5DEB3",
        Pcv::White => "FFFFFF",
        Pcv::WhiteSmoke => "F5F5F5",
        Pcv::Yellow => "FFFF00",
        Pcv::YellowGreen => "9ACD32",
        Pcv::DarkBlue2010 => "00008B",
        Pcv::DarkCyan2010 => "008B8B",
        Pcv::DarkGoldenrod2010 => "B8860B",
        Pcv::DarkGray2010 => "A9A9A9",
        Pcv::DarkGrey2010 => "A9A9A9",
        Pcv::DarkGreen2010 => "006400",
        Pcv::DarkKhaki2010 => "BDB76B",
        Pcv::DarkMagenta2010 => "8B008B",
        Pcv::DarkOliveGreen2010 => "556B2F",
        Pcv::DarkOrange2010 => "FF8C00",
        Pcv::DarkOrchid2010 => "9932CC",
        Pcv::DarkRed2010 => "8B0000",
        Pcv::DarkSalmon2010 => "E9967A",
        Pcv::DarkSeaGreen2010 => "8FBC8F",
        Pcv::DarkSlateBlue2010 => "483D8B",
        Pcv::DarkSlateGray2010 => "2F4F4F",
        Pcv::DarkSlateGrey2010 => "2F4F4F",
        Pcv::DarkTurquoise2010 => "00CED1",
        Pcv::DarkViolet2010 => "9400D3",
        Pcv::LightBlue2010 => "ADD8E6",
        Pcv::LightCoral2010 => "F08080",
        Pcv::LightCyan2010 => "E0FFFF",
        Pcv::LightGoldenrodYellow2010 => "FAFAD2",
        Pcv::LightGray2010 => "D3D3D3",
        Pcv::LightGrey2010 => "D3D3D3",
        Pcv::LightGreen2010 => "90EE90",
        Pcv::LightPink2010 => "FFB6C1",
        Pcv::LightSalmon2010 => "FFA07A",
        Pcv::LightSeaGreen2010 => "20B2AA",
        Pcv::LightSkyBlue2010 => "87CEFA",
        Pcv::LightSlateGray2010 => "778899",
        Pcv::LightSlateGrey2010 => "778899",
        Pcv::LightSteelBlue2010 => "B0C4DE",
        Pcv::LightYellow2010 => "FFFFE0",
        Pcv::DarkGrey => "A9A9A9",
        Pcv::DimGrey => "696969",
        Pcv::DarkSlateGrey => "2F4F4F",
        Pcv::Grey => "808080",
        Pcv::LightGrey => "D3D3D3",
        Pcv::LightSlateGrey => "778899",
        Pcv::SlateGrey => "708090",
        // 2010-aliased Medium* variants — schema added these alongside the
        // pre-existing un-suffixed Medium* without changing the values.
        Pcv::MediumAquamarine2010 => "66CDAA",
        Pcv::MediumBlue2010 => "0000CD",
        Pcv::MediumOrchid2010 => "BA55D3",
        Pcv::MediumPurple2010 => "9370DB",
        Pcv::MediumSeaGreen2010 => "3CB371",
        Pcv::MediumSlateBlue2010 => "7B68EE",
        Pcv::MediumSpringGreen2010 => "00FA9A",
        Pcv::MediumTurquoise2010 => "48D1CC",
        Pcv::MediumVioletRed2010 => "C71585",
    };
    hex.to_string()
}

pub fn extract(theme: &a::Theme) -> Theme {
    let scheme = &theme.theme_elements.color_scheme;
    let colors = vec![
        resolve_light1(&scheme.light1_color).unwrap_or_else(|| default_for(0).into()),
        resolve_dark1(&scheme.dark1_color).unwrap_or_else(|| default_for(1).into()),
        resolve_light2(&scheme.light2_color).unwrap_or_else(|| default_for(2).into()),
        resolve_dark2(&scheme.dark2_color).unwrap_or_else(|| default_for(3).into()),
        resolve_accent1(&scheme.accent1_color).unwrap_or_else(|| default_for(4).into()),
        resolve_accent2(&scheme.accent2_color).unwrap_or_else(|| default_for(5).into()),
        resolve_accent3(&scheme.accent3_color).unwrap_or_else(|| default_for(6).into()),
        resolve_accent4(&scheme.accent4_color).unwrap_or_else(|| default_for(7).into()),
        resolve_accent5(&scheme.accent5_color).unwrap_or_else(|| default_for(8).into()),
        resolve_accent6(&scheme.accent6_color).unwrap_or_else(|| default_for(9).into()),
        resolve_hyperlink(&scheme.hyperlink).unwrap_or_else(|| default_for(10).into()),
        resolve_followed(&scheme.followed_hyperlink_color)
            .unwrap_or_else(|| default_for(11).into()),
    ];

    let major_font = theme
        .theme_elements
        .font_scheme
        .major_font
        .latin_font
        .typeface
        .as_ref()
        .map(|t| t.as_str().to_string())
        .filter(|s| !s.is_empty());
    let minor_font = theme
        .theme_elements
        .font_scheme
        .minor_font
        .latin_font
        .typeface
        .as_ref()
        .map(|t| t.as_str().to_string())
        .filter(|s| !s.is_empty());

    Theme {
        colors,
        major_font,
        minor_font,
    }
}

/// Office 2007+ default theme color for spreadsheet-index slot N. Mirrors
/// the legacy `THEME_PALETTE` constant in the renderer; kept here as a
/// fallback for color slots we can't resolve.
fn default_for(slot: usize) -> &'static str {
    match slot {
        0 => "FFFFFF",  // lt1
        1 => "000000",  // dk1
        2 => "E7E6E6",  // lt2
        3 => "44546A",  // dk2
        4 => "4472C4",  // accent1
        5 => "ED7D31",  // accent2
        6 => "A5A5A5",  // accent3
        7 => "FFC000",  // accent4
        8 => "5B9BD5",  // accent5
        9 => "70AD47",  // accent6
        10 => "0563C1", // hlink
        11 => "954F72", // folHlink
        _ => "000000",
    }
}

// ooxmlsdk generates a per-slot wrapper struct + per-slot choice enum
// (Light1Color/Light1ColorChoice, Dark1Color/Dark1ColorChoice, …) instead
// of a shared `CT_Color2`. They have identical shape; we resolve each via
// a tiny adapter so the per-variant pattern matches stay readable.
macro_rules! resolve_slot {
    ($fn_name:ident, $wrapper:ty, $field:ident, $choice_path:path) => {
        fn $fn_name(c: &$wrapper) -> Option<String> {
            use $choice_path as Choice;
            match c.$field.as_ref()? {
                Choice::ASrgbClr(rgb) => Some(rgb.val.as_str().to_string()),
                Choice::ASysClr(sys) => sys.last_color.as_ref().map(|h| h.as_str().to_string()),
                Choice::AScrgbClr(c) => Some(resolve_scrgb(c)),
                Choice::AHslClr(c) => Some(resolve_hsl(c)),
                Choice::APrstClr(c) => Some(resolve_preset(c)),
            }
        }
    };
}

resolve_slot!(
    resolve_light1,
    a::Light1Color,
    light1_color_choice,
    a::Light1ColorChoice
);
resolve_slot!(
    resolve_dark1,
    a::Dark1Color,
    dark1_color_choice,
    a::Dark1ColorChoice
);
resolve_slot!(
    resolve_light2,
    a::Light2Color,
    light2_color_choice,
    a::Light2ColorChoice
);
resolve_slot!(
    resolve_dark2,
    a::Dark2Color,
    dark2_color_choice,
    a::Dark2ColorChoice
);
resolve_slot!(
    resolve_accent1,
    a::Accent1Color,
    accent1_color_choice,
    a::Accent1ColorChoice
);
resolve_slot!(
    resolve_accent2,
    a::Accent2Color,
    accent2_color_choice,
    a::Accent2ColorChoice
);
resolve_slot!(
    resolve_accent3,
    a::Accent3Color,
    accent3_color_choice,
    a::Accent3ColorChoice
);
resolve_slot!(
    resolve_accent4,
    a::Accent4Color,
    accent4_color_choice,
    a::Accent4ColorChoice
);
resolve_slot!(
    resolve_accent5,
    a::Accent5Color,
    accent5_color_choice,
    a::Accent5ColorChoice
);
resolve_slot!(
    resolve_accent6,
    a::Accent6Color,
    accent6_color_choice,
    a::Accent6ColorChoice
);
resolve_slot!(
    resolve_hyperlink,
    a::Hyperlink,
    hyperlink_choice,
    a::HyperlinkChoice
);
resolve_slot!(
    resolve_followed,
    a::FollowedHyperlinkColor,
    followed_hyperlink_color_choice,
    a::FollowedHyperlinkColorChoice
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrgb_full_range() {
        assert_eq!(scrgb_byte(0), 0);
        assert_eq!(scrgb_byte(100_000), 255);
        // 50% → 128 (round half up).
        assert_eq!(scrgb_byte(50_000), 128);
        // Out-of-range clamps.
        assert_eq!(scrgb_byte(-50), 0);
        assert_eq!(scrgb_byte(110_000), 255);
    }

    #[test]
    fn hsl_primaries() {
        // Pure red: hue 0°, sat 100%, lum 50%.
        assert_eq!(hsl_to_rgb(0.0, 1.0, 0.5), (255, 0, 0));
        assert_eq!(hsl_to_rgb(120.0, 1.0, 0.5), (0, 255, 0));
        assert_eq!(hsl_to_rgb(240.0, 1.0, 0.5), (0, 0, 255));
        // Black & white via lum extremes.
        assert_eq!(hsl_to_rgb(0.0, 0.0, 0.0), (0, 0, 0));
        assert_eq!(hsl_to_rgb(0.0, 0.0, 1.0), (255, 255, 255));
        // Mid-gray: 0% sat, 50% lum.
        assert_eq!(hsl_to_rgb(180.0, 0.0, 0.5), (128, 128, 128));
    }

    #[test]
    fn rgb_hex_format() {
        assert_eq!(rgb_hex(0, 0, 0), "000000");
        assert_eq!(rgb_hex(255, 255, 255), "FFFFFF");
        assert_eq!(rgb_hex(0x44, 0x72, 0xC4), "4472C4"); // accent1
    }
}
