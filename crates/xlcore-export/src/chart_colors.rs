use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;

pub(crate) fn line_has_no_fill(props: &c::ChartShapeProperties) -> bool {
    let dbg = format!("{:?}", props);
    let Some(ln_pos) = dbg.find("a_ln: Some(Outline {") else {
        return false;
    };
    let block = scope_from_open_brace(&dbg[ln_pos..]);

    block.contains("ANoFill")
}

pub(crate) fn shape_has_no_fill(props: &c::ChartShapeProperties) -> bool {
    let dbg = format!("{:?}", props);
    dbg.contains("chart_shape_properties_choice2: Some(ANoFill")
}

pub(crate) fn series_color_via_debug(
    props: Option<&c::ChartShapeProperties>,
    theme: Option<&Theme>,
) -> Option<String> {
    let props = props?;
    let dbg = format!("{:?}", props);

    let fill_pos = dbg.find("chart_shape_properties_choice2: Some(ASolidFill(SolidFill {")?;
    let fill_block = &dbg[fill_pos..];

    let end = fill_block
        .find("})),")
        .or_else(|| fill_block.find("}))"))
        .unwrap_or(fill_block.len());
    let fill_block = &fill_block[..end];

    if let Some(p) = fill_block.find("RgbColorModelHex {") {
        let rgb_scope = scope_from_open_brace(&fill_block[p..]);
        let rest = rgb_scope;
        if let Some(v) = rest.find("val: \"") {
            let body = &rest[v + 6..];
            if let Some(e) = body.find('"') {
                let hex = &body[..e];
                if hex.len() == 6 {
                    return Some(apply_color_modifiers(&format!("#{}", hex), rgb_scope));
                }
            }
        }
    }
    if let Some(p) = fill_block.find("SchemeColor {") {
        let scheme_scope = scope_from_open_brace(&fill_block[p..]);
        if let Some(base) = theme_scheme_color(scheme_scope, theme) {
            return Some(apply_color_modifiers(&base, scheme_scope));
        }
    }
    None
}

pub(crate) fn theme_scheme_color(scheme_scope: &str, theme: Option<&Theme>) -> Option<String> {
    for n in 1..=6u32 {
        let needle = format!("Accent{n}");
        if scheme_scope.contains(&needle) {
            return Some(theme_accent_color(n, theme));
        }
    }

    let by_slot = |slot: usize| -> String {
        theme
            .and_then(|t| t.colors.get(slot).cloned())
            .filter(|h| h.len() == 6)
            .map(|h| format!("#{}", h))
            .unwrap_or_else(|| match slot {
                0 => "#FFFFFF".to_string(),
                1 => "#000000".to_string(),
                2 => "#E7E6E6".to_string(),
                3 => "#44546A".to_string(),
                10 => "#0563C1".to_string(),
                11 => "#954F72".to_string(),
                _ => "#000000".to_string(),
            })
    };

    if scheme_scope.contains("val: Light1") {
        return Some(by_slot(0));
    }
    if scheme_scope.contains("val: Dark1") {
        return Some(by_slot(1));
    }
    if scheme_scope.contains("val: Light2") {
        return Some(by_slot(2));
    }
    if scheme_scope.contains("val: Dark2") {
        return Some(by_slot(3));
    }

    if scheme_scope.contains("val: Background1") {
        return Some(by_slot(0));
    }
    if scheme_scope.contains("val: Text1") {
        return Some(by_slot(1));
    }
    if scheme_scope.contains("val: Background2") {
        return Some(by_slot(2));
    }
    if scheme_scope.contains("val: Text2") {
        return Some(by_slot(3));
    }
    if scheme_scope.contains("val: Hyperlink") {
        return Some(by_slot(10));
    }
    if scheme_scope.contains("val: FollowedHyperlink") {
        return Some(by_slot(11));
    }

    if scheme_scope.contains("val: WindowText") {
        return Some("#000000".to_string());
    }
    if scheme_scope.contains("val: Window") {
        return Some("#FFFFFF".to_string());
    }
    None
}

pub(crate) fn marker_symbol_str(v: &c::MarkerStyleValues) -> String {
    match v {
        c::MarkerStyleValues::Auto => "auto",
        c::MarkerStyleValues::Circle => "circle",
        c::MarkerStyleValues::Dash => "dash",
        c::MarkerStyleValues::Diamond => "diamond",
        c::MarkerStyleValues::Dot => "dot",
        c::MarkerStyleValues::None => "none",
        c::MarkerStyleValues::Picture => "picture",
        c::MarkerStyleValues::Plus => "plus",
        c::MarkerStyleValues::Square => "square",
        c::MarkerStyleValues::Star => "star",
        c::MarkerStyleValues::Triangle => "triangle",
        c::MarkerStyleValues::X => "x",
    }
    .to_string()
}

pub(crate) fn line_color_via_debug(
    props: Option<&c::ChartShapeProperties>,
    theme: Option<&Theme>,
) -> Option<String> {
    let props = props?;
    let dbg = format!("{:?}", props);
    let ln_pos = dbg.find("a_ln: Some(Outline {")?;
    let ln_block = scope_from_open_brace(&dbg[ln_pos..]);
    let fill_pos = ln_block.find("ASolidFill(SolidFill {")?;
    let fill_block = &ln_block[fill_pos..];
    let end = fill_block
        .find("})),")
        .or_else(|| fill_block.find("}))"))
        .unwrap_or(fill_block.len());
    let fill_block = &fill_block[..end];

    if let Some(p) = fill_block.find("RgbColorModelHex {") {
        let rgb_scope = scope_from_open_brace(&fill_block[p..]);
        if let Some(v) = rgb_scope.find("val: \"") {
            let body = &rgb_scope[v + 6..];
            if let Some(e) = body.find('"') {
                let hex = &body[..e];
                if hex.len() == 6 {
                    return Some(apply_color_modifiers(&format!("#{}", hex), rgb_scope));
                }
            }
        }
    }
    if let Some(p) = fill_block.find("SchemeColor {") {
        let scheme_scope = scope_from_open_brace(&fill_block[p..]);
        if let Some(base) = theme_scheme_color(scheme_scope, theme) {
            return Some(apply_color_modifiers(&base, scheme_scope));
        }
    }
    None
}

pub(crate) fn scope_from_open_brace(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut started = false;
    let mut in_str = false;
    let mut prev_escape = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if b == b'\\' && !prev_escape {
                prev_escape = true;
                continue;
            }
            if b == b'"' && !prev_escape {
                in_str = false;
            }
            prev_escape = false;
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => {
                depth += 1;
                started = true;
            }
            b'}' => {
                depth -= 1;
                if started && depth == 0 {
                    return &s[..=i];
                }
            }
            _ => {}
        }
    }
    s
}

pub(crate) fn apply_color_modifiers(hex_in: &str, scan_block: &str) -> String {
    if hex_in.len() != 7 || !hex_in.starts_with('#') {
        return hex_in.to_string();
    }
    let r0 = u8::from_str_radix(&hex_in[1..3], 16).unwrap_or(0) as f64 / 255.0;
    let g0 = u8::from_str_radix(&hex_in[3..5], 16).unwrap_or(0) as f64 / 255.0;
    let b0 = u8::from_str_radix(&hex_in[5..7], 16).unwrap_or(0) as f64 / 255.0;
    let (h, mut s, mut l) = rgb_to_hsl(r0, g0, b0);

    let patterns: &[(&str, &str)] = &[
        ("ALumMod(LuminanceModulation { val: ", "lumMod"),
        ("ALumOff(LuminanceOffset { val: ", "lumOff"),
        ("ASatMod(SaturationModulation { val: ", "satMod"),
        ("ASatOff(SaturationOffset { val: ", "satOff"),
        ("AShade(Shade { val: ", "shade"),
        ("ATint(Tint { val: ", "tint"),
    ];

    let mut events: Vec<(usize, &str, i32)> = Vec::new();
    for (needle, kind) in patterns {
        let mut start = 0usize;
        while let Some(rel) = scan_block[start..].find(needle) {
            let abs = start + rel;
            let after = abs + needle.len();
            let tail = &scan_block[after..];

            let mut end = 0usize;
            for (i, c) in tail.bytes().enumerate() {
                let digit = c.is_ascii_digit();
                if !(digit || (i == 0 && (c == b'-' || c == b'+'))) {
                    end = i;
                    break;
                }
            }
            if end > 0 {
                if let Ok(v) = tail[..end].parse::<i32>() {
                    events.push((abs, *kind, v));
                }
            }
            start = after;
        }
    }
    events.sort_by_key(|(pos, _, _)| *pos);

    for (_, kind, val) in events {
        let v = (val as f64) / 100_000.0;
        match kind {
            "lumMod" => l = (l * v).clamp(0.0, 1.0),
            "lumOff" => l = (l + v).clamp(0.0, 1.0),
            "satMod" => s = (s * v).clamp(0.0, 1.0),
            "satOff" => s = (s + v).clamp(0.0, 1.0),

            "shade" => l = (l * v).clamp(0.0, 1.0),

            "tint" => l = (l * (1.0 - v) + v).clamp(0.0, 1.0),
            _ => {}
        }
    }

    let (r2, g2, b2) = hsl_to_rgb(h, s, l);
    format!(
        "#{:02X}{:02X}{:02X}",
        (r2 * 255.0).round() as u8,
        (g2 * 255.0).round() as u8,
        (b2 * 255.0).round() as u8,
    )
}

pub(crate) fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-9 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if max == r {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        ((b - r) / d) + 2.0
    } else {
        ((r - g) / d) + 4.0
    } / 6.0;
    (h, s, l)
}

pub(crate) fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s <= 1e-9 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue2rgb = |t: f64| -> f64 {
        let mut t = t;
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            return p + (q - p) * 6.0 * t;
        }
        if t < 0.5 {
            return q;
        }
        if t < 2.0 / 3.0 {
            return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
        }
        p
    };
    (hue2rgb(h + 1.0 / 3.0), hue2rgb(h), hue2rgb(h - 1.0 / 3.0))
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod color_mod_tests {
    use super::*;

    fn approx_eq_hex(a: &str, b: &str, tol: i32) -> bool {
        if a.len() != 7 || b.len() != 7 {
            return false;
        }
        for i in 0..3 {
            let off = 1 + i * 2;
            let va = i32::from_str_radix(&a[off..off + 2], 16).unwrap();
            let vb = i32::from_str_radix(&b[off..off + 2], 16).unwrap();
            if (va - vb).abs() > tol {
                return false;
            }
        }
        true
    }

    #[test]
    fn accent1_lighter_40_matches_excel() {
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 60000 }), ALumOff(LuminanceOffset { val: 40000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#8FAADC", 3), "got {out}");
    }

    #[test]
    fn accent1_lighter_60_matches_excel() {
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 40000 }), ALumOff(LuminanceOffset { val: 60000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#B4C7E7", 3), "got {out}");
    }

    #[test]
    fn accent1_lighter_80_matches_excel() {
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 20000 }), ALumOff(LuminanceOffset { val: 80000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#D9E1F2", 3), "got {out}");
    }

    #[test]
    fn accent1_darker_25_matches_excel() {
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 75000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#2E5395", 4), "got {out}");
    }

    #[test]
    fn no_modifiers_returns_base() {
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert_eq!(out.to_ascii_lowercase(), "#4472c4");
    }

    #[test]
    fn scope_from_open_brace_skips_nested() {
        let s = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 60000 })] } trailing";
        let scoped = scope_from_open_brace(s);
        assert!(scoped.ends_with('}'));
        assert!(!scoped.contains("trailing"));
    }

    fn block_with_val(variant: &str) -> String {
        format!("SchemeColor {{ val: {variant}, scheme_color_choice: [] }}")
    }

    #[test]
    fn scheme_bg1_resolves_to_white_via_default_palette() {
        let block = block_with_val("Background1");
        assert_eq!(theme_scheme_color(&block, None).as_deref(), Some("#FFFFFF"));
    }

    #[test]
    fn scheme_tx1_resolves_to_black() {
        let block = block_with_val("Text1");
        assert_eq!(theme_scheme_color(&block, None).as_deref(), Some("#000000"));
    }

    #[test]
    fn scheme_lt1_dk1_match_bg1_tx1_under_default_clrmap() {
        let lt = theme_scheme_color(&block_with_val("Light1"), None);
        let bg = theme_scheme_color(&block_with_val("Background1"), None);
        assert_eq!(lt, bg);
        let dk = theme_scheme_color(&block_with_val("Dark1"), None);
        let tx = theme_scheme_color(&block_with_val("Text1"), None);
        assert_eq!(dk, tx);
    }

    #[test]
    fn scheme_bg2_uses_default_lt2() {
        let block = block_with_val("Background2");
        assert_eq!(theme_scheme_color(&block, None).as_deref(), Some("#E7E6E6"));
    }

    #[test]
    fn scheme_accent1_still_works() {
        let block = block_with_val("Accent1");
        assert_eq!(theme_scheme_color(&block, None).as_deref(), Some("#4472C4"));
    }

    #[test]
    fn scheme_hyperlink_resolves_to_default_hlink() {
        let block = block_with_val("Hyperlink");
        assert_eq!(theme_scheme_color(&block, None).as_deref(), Some("#0563C1"));
    }

    #[test]
    fn scheme_unknown_variant_returns_none() {
        let block = block_with_val("PhColor");
        assert!(theme_scheme_color(&block, None).is_none());
    }

    #[test]
    fn scheme_bg1_honors_workbook_theme_override() {
        let theme = Theme {
            colors: vec![
                "FFF8E1".to_string(),
                "000000".to_string(),
                "E7E6E6".to_string(),
                "44546A".to_string(),
            ],
            major_font: None,
            minor_font: None,
            fmt_scheme: None,
        };
        let block = block_with_val("Background1");
        assert_eq!(
            theme_scheme_color(&block, Some(&theme)).as_deref(),
            Some("#FFF8E1"),
        );
    }
}

pub(crate) fn theme_accent_color(n: u32, theme: Option<&Theme>) -> String {
    if let Some(t) = theme {
        let slot = 3 + n as usize;
        if let Some(hex) = t.colors.get(slot) {
            if hex.len() == 6 {
                return format!("#{}", hex);
            }
        }
    }
    office_accent_color_default(n)
}

pub(crate) fn office_accent_color_default(n: u32) -> String {
    match n {
        1 => "#4472C4",
        2 => "#ED7D31",
        3 => "#A5A5A5",
        4 => "#FFC000",
        5 => "#5B9BD5",
        6 => "#70AD47",
        _ => "#4472C4",
    }
    .to_string()
}

pub(crate) fn extract_title(t: Option<&c::Title>) -> Option<String> {
    let t = t?;
    let txt = t.chart_text.as_ref()?;
    match txt.chart_text_choice.as_ref()? {
        c::ChartTextChoice::CStrRef(sr) => sr.string_cache.as_ref().and_then(|sc| {
            sc.c_pt
                .first()
                .map(|p| p.numeric_value.as_str().to_string())
        }),
        c::ChartTextChoice::CRich(rich) => {
            let mut s = String::new();
            for p in &rich.a_p {
                for ch in &p.paragraph_choice {
                    if let ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::ParagraphChoice::AR(run) = ch {
                        s.push_str(run.text.as_str());
                    }
                }
            }
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        c::ChartTextChoice::CStrLit(lit) => lit
            .c_pt
            .first()
            .map(|p| p.numeric_value.as_str().to_string()),
    }
}
