#![allow(dead_code)]
use xlcore_io::spreadsheetml as x;

const INDEXED_PALETTE: [(u32, &str); 66] = [
    (0, "000000"),
    (1, "FFFFFF"),
    (2, "FF0000"),
    (3, "00FF00"),
    (4, "0000FF"),
    (5, "FFFF00"),
    (6, "FF00FF"),
    (7, "00FFFF"),
    (8, "000000"),
    (9, "FFFFFF"),
    (10, "FF0000"),
    (11, "00FF00"),
    (12, "0000FF"),
    (13, "FFFF00"),
    (14, "FF00FF"),
    (15, "00FFFF"),
    (16, "800000"),
    (17, "008000"),
    (18, "000080"),
    (19, "808000"),
    (20, "800080"),
    (21, "008080"),
    (22, "C0C0C0"),
    (23, "808080"),
    (24, "9999FF"),
    (25, "993366"),
    (26, "FFFFCC"),
    (27, "CCFFFF"),
    (28, "660066"),
    (29, "FF8080"),
    (30, "0066CC"),
    (31, "CCCCFF"),
    (32, "000080"),
    (33, "FF00FF"),
    (34, "FFFF00"),
    (35, "00FFFF"),
    (36, "800080"),
    (37, "800000"),
    (38, "008080"),
    (39, "0000FF"),
    (40, "00CCFF"),
    (41, "CCFFFF"),
    (42, "CCFFCC"),
    (43, "FFFF99"),
    (44, "99CCFF"),
    (45, "FF99CC"),
    (46, "CC99FF"),
    (47, "FFCC99"),
    (48, "3366FF"),
    (49, "33CCCC"),
    (50, "99CC00"),
    (51, "FFCC00"),
    (52, "FF9900"),
    (53, "FF6600"),
    (54, "666699"),
    (55, "969696"),
    (56, "003366"),
    (57, "339966"),
    (58, "003300"),
    (59, "333300"),
    (60, "993300"),
    (61, "993366"),
    (62, "333399"),
    (63, "333333"),
    (64, "000000"),
    (65, "FFFFFF"),
];

const DEFAULT_THEME_PALETTE: [&str; 12] = [
    "FFFFFF", "000000", "E7E6E6", "44546A", "4472C4", "ED7D31", "A5A5A5", "FFC000", "5B9BD5",
    "70AD47", "0563C1", "954F72",
];

fn indexed_hex(idx: u32) -> Option<String> {
    INDEXED_PALETTE
        .iter()
        .find(|(i, _)| *i == idx)
        .map(|(_, hex)| (*hex).to_string())
}

fn theme_palette(doc: &mut xlcore_io::SpreadsheetDocument) -> Vec<String> {
    let mut palette: Vec<String> = DEFAULT_THEME_PALETTE.iter().map(|s| s.to_string()).collect();
    let Ok(wb_part) = doc.workbook_part() else {
        return palette;
    };
    let wb_part = wb_part.clone();
    let Some(tp) = wb_part.theme_part(doc) else {
        return palette;
    };
    let tp = tp.clone();
    if let Ok(theme) = tp.root_element(doc) {
        let extracted = xlcore_export::extract_theme(theme);
        for (i, hex) in extracted.colors.iter().enumerate() {
            if is_hex6(hex) {
                if i < palette.len() {
                    palette[i] = hex.to_ascii_uppercase();
                } else {
                    palette.push(hex.to_ascii_uppercase());
                }
            }
        }
    }
    palette
}

fn is_hex6(s: &str) -> bool {
    s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn theme_hex(doc: &mut xlcore_io::SpreadsheetDocument, idx: u32) -> Option<String> {
    let palette = theme_palette(doc);
    palette.get(idx as usize).cloned()
}

fn normalize_rgb(rgb: &str) -> Option<String> {
    let s = rgb.trim().trim_start_matches('#').to_ascii_uppercase();
    match s.len() {
        8 => Some(s[2..].to_string()),
        6 => Some(s),
        _ => None,
    }
}

fn apply_tint(hex: &str, tint: f64) -> String {
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let mut h = 0.0;
    let mut s = 0.0;
    if (max - min).abs() > f64::EPSILON {
        let d = max - min;
        s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        h = if max == r {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if max == g {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        h /= 6.0;
    }

    let mut l2 = if tint < 0.0 {
        l * (1.0 + tint)
    } else {
        l * (1.0 - tint) + tint
    };
    l2 = l2.clamp(0.0, 1.0);

    let (r2, g2, b2) = if s == 0.0 {
        (l2, l2, l2)
    } else {
        let q = if l2 < 0.5 {
            l2 * (1.0 + s)
        } else {
            l2 + s - l2 * s
        };
        let p = 2.0 * l2 - q;
        (
            hue2rgb(p, q, h + 1.0 / 3.0),
            hue2rgb(p, q, h),
            hue2rgb(p, q, h - 1.0 / 3.0),
        )
    };

    format!(
        "{:02X}{:02X}{:02X}",
        (r2 * 255.0).round() as u8,
        (g2 * 255.0).round() as u8,
        (b2 * 255.0).round() as u8
    )
}

fn hue2rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

pub(crate) fn resolve_color_hex(
    doc: &mut xlcore_io::SpreadsheetDocument,
    c: &x::Color,
) -> Option<String> {
    let base = if let Some(rgb) = c.rgb.as_deref() {
        normalize_rgb(rgb)
    } else if let Some(theme) = c.theme {
        theme_hex(doc, theme)
    } else if let Some(indexed) = c.indexed {
        indexed_hex(indexed)
    } else {
        None
    }?;

    match c.tint {
        Some(t) if t != 0.0 => Some(apply_tint(&base, t)),
        _ => Some(base),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_blank() -> xlcore_io::SpreadsheetDocument {
        crate::Workbook::new().unwrap().doc
    }

    #[test]
    fn rgb_without_alpha() {
        let mut doc = open_blank();
        let c = x::Color {
            rgb: Some("FF8800".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_color_hex(&mut doc, &c), Some("FF8800".to_string()));
    }

    #[test]
    fn rgb_with_alpha() {
        let mut doc = open_blank();
        let c = x::Color {
            rgb: Some("FFFF8800".to_string()),
            ..Default::default()
        };
        assert_eq!(resolve_color_hex(&mut doc, &c), Some("FF8800".to_string()));
    }

    #[test]
    fn indexed_color() {
        let mut doc = open_blank();
        let c = x::Color {
            indexed: Some(2),
            ..Default::default()
        };
        assert_eq!(resolve_color_hex(&mut doc, &c), Some("FF0000".to_string()));
    }

    #[test]
    fn theme_slot() {
        let mut doc = open_blank();
        let c = x::Color {
            theme: Some(4),
            ..Default::default()
        };
        let got = resolve_color_hex(&mut doc, &c).unwrap();
        assert!(is_hex6(&got), "got {got}");
    }

    #[test]
    fn theme_with_tint() {
        let mut doc = open_blank();
        let plain = x::Color {
            theme: Some(0),
            ..Default::default()
        };
        let tinted = x::Color {
            theme: Some(0),
            tint: Some(-0.25),
            ..Default::default()
        };
        let a = resolve_color_hex(&mut doc, &plain).unwrap();
        let b = resolve_color_hex(&mut doc, &tinted).unwrap();
        assert_ne!(a, b);
        assert!(is_hex6(&b));
    }

    #[test]
    fn auto_unresolved() {
        let mut doc = open_blank();
        let c = x::Color::default();
        assert_eq!(resolve_color_hex(&mut doc, &c), None);
    }
}
