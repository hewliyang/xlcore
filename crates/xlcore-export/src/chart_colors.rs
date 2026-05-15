use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;

pub(crate) fn line_has_no_fill(props: &c::ChartShapeProperties) -> bool {
    let dbg = format!("{:?}", props);
    let Some(ln_pos) = dbg.find("a_ln: Some(Outline {") else {
        return false;
    };
    let block = scope_from_open_brace(&dbg[ln_pos..]);
    // ANoFill is the Debug-repr variant name for `<a:noFill/>`.
    block.contains("ANoFill")
}

/// Detect `<c:spPr><a:noFill/></c:spPr>` — a *shape-level* noFill
/// (the fill choice itself is `<a:noFill/>` rather than
/// `<a:solidFill>...`). Used by per-data-point fill resolution to
/// distinguish "explicitly transparent" from "no override".
///
/// We anchor on `chart_shape_properties_choice2: Some(ANoFill` —
/// the same choice enum `series_color_via_debug` anchors on for
/// `ASolidFill`. Critically we do *not* match the `ANoFill` that
/// sometimes sits inside `a_ln: Some(Outline { ... ANoFill })`
/// (that's the outline noFill, not the shape fill).
pub(crate) fn shape_has_no_fill(props: &c::ChartShapeProperties) -> bool {
    let dbg = format!("{:?}", props);
    dbg.contains("chart_shape_properties_choice2: Some(ANoFill")
}

// Pull explicit fill colors out of a `<c:spPr>` block via the struct's
// Debug repr. ooxmlsdk's choice enums are a moving target across
// versions, so a string scan is pragmatic. The Debug repr uses Rust
// type names (`RgbColorModelHex`, `SchemeColor`, `ASolidFill`) rather
// than the XML qnames (`srgbClr`, `schemeClr`, `solidFill`) — so we
// anchor on `ASolidFill(SolidFill {` to lock onto the fill (and skip
// e.g. line-color `<a:ln>` blocks), then look for the first
// `RgbColorModelHex { ... val: "<6 hex>"` or `SchemeColor { val:
// AccentN` underneath it.
pub(crate) fn series_color_via_debug(
    props: Option<&c::ChartShapeProperties>,
    theme: Option<&Theme>,
) -> Option<String> {
    let props = props?;
    let dbg = format!("{:?}", props);
    // Scope to the *shape's* solid fill (chart_shape_properties_choice2).
    // Important: we deliberately don't look inside `a_ln: Some(Outline {
    // ... ASolidFill(...) })` — that's the outline color, not the fill.
    // Series-level spPr commonly has only an outline, no shape fill, in
    // which case the function correctly returns None and the caller
    // falls back to the theme accent.
    let fill_pos = dbg.find("chart_shape_properties_choice2: Some(ASolidFill(SolidFill {")?;
    let fill_block = &dbg[fill_pos..];
    // Cap the scan at the close of the SolidFill struct (best-effort:
    // first `}))` after the open brace covers the typical shape
    // `ASolidFill(SolidFill { ... Some(ASrgbClr(... { ... }))` with the
    // outer `}))` ending the SolidFill).
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
        for n in 1..=6u32 {
            let needle = format!("Accent{n}");
            if scheme_scope.contains(&needle) {
                let base = theme_accent_color(n, theme);
                return Some(apply_color_modifiers(&base, scheme_scope));
            }
        }
    }
    None
}

/// Sibling of `series_color_via_debug` that scopes to the *outline*
/// solid fill (`<c:spPr><a:ln><a:solidFill>...`). Line and scatter
/// series are commonly authored with outline-only spPr (no shape
/// fill), and Excel treats the outline color as the series color.
/// `<a:ln>` blocks frequently set `<a:noFill/>` on the outline of
/// shapes (bars etc.) too, so we anchor on the ASolidFill sub-block
/// inside the `a_ln` Outline scope and skip cases where it's absent.
/// Translate ooxmlsdk's `MarkerStyleValues` enum into the schema's
/// string form (e.g. `"none"` / `"circle"` / `"square"`). Mirrors the
/// XML attribute values per ECMA-376 §21.2.3.10. We could derive this
/// from the Debug repr but doing it via `match` keeps it robust to
/// ooxmlsdk variant renames.
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
        for n in 1..=6u32 {
            let needle = format!("Accent{n}");
            if scheme_scope.contains(&needle) {
                let base = theme_accent_color(n, theme);
                return Some(apply_color_modifiers(&base, scheme_scope));
            }
        }
    }
    None
}

/// Given a slice starting at `Name {` (i.e. the `{` is somewhere near the
/// start), return the substring ending at the matching `}`. Used to scope
/// modifier-list scans to a single SchemeColor / RgbColorModelHex block.
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

/// Apply OOXML drawingML color-transform children (`<a:lumMod>`,
/// `<a:lumOff>`, `<a:shade>`, `<a:tint>`, `<a:satMod>`) to a base hex
/// in declaration order. Used by chart series color resolution so
/// `<a:schemeClr val="accent5"><a:lumMod val="60000"/><a:lumOff
/// val="40000"/></a:schemeClr>` produces "Accent5, Lighter 40%"
/// instead of the bare accent. ECMA-376 §20.1.2.3.
///
/// Source-order matters; we scan the Debug-repr scope and collect
/// each modifier with its byte position, then sort by position. The
/// SchemeColor/RgbColorModelHex Debug repr emits modifiers in the
/// order they appeared in the XML's `<a:lumMod>` / `<a:lumOff>` /
/// etc. children (preserved by ooxmlsdk via the choice `Vec`).
pub(crate) fn apply_color_modifiers(hex_in: &str, scan_block: &str) -> String {
    if hex_in.len() != 7 || !hex_in.starts_with('#') {
        return hex_in.to_string();
    }
    let r0 = u8::from_str_radix(&hex_in[1..3], 16).unwrap_or(0) as f64 / 255.0;
    let g0 = u8::from_str_radix(&hex_in[3..5], 16).unwrap_or(0) as f64 / 255.0;
    let b0 = u8::from_str_radix(&hex_in[5..7], 16).unwrap_or(0) as f64 / 255.0;
    let (h, mut s, mut l) = rgb_to_hsl(r0, g0, b0);

    // Match each modifier variant and record (position_in_block, kind, val).
    // Kind names: "lumMod" / "lumOff" / "satMod" / "satOff" /
    // "shade" / "tint". The Debug-repr patterns include the wrapper
    // struct name so we don't accidentally match e.g. AlphaModulation.
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
            // Parse the integer literal (allow leading `-`).
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
            // Shade in OOXML drawingML moves luminance toward 0:
            // L' = L * val/100000.
            "shade" => l = (l * v).clamp(0.0, 1.0),
            // Tint moves luminance toward 1 (mix with white):
            // L' = L * (1 - v) + v.  ECMA-376 §20.1.2.3.30.
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
        // Accent1 #4472C4, lumMod=60000 + lumOff=40000 = "Lighter 40%"
        // → Excel renders #8FAADC. Tolerance ±3/255 for HLS rounding
        // (Excel uses 240-step HLSMAX; we work in [0,1] floats).
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 60000 }), ALumOff(LuminanceOffset { val: 40000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#8FAADC", 3), "got {out}");
    }

    #[test]
    fn accent1_lighter_60_matches_excel() {
        // Accent1 #4472C4, lumMod=40000 + lumOff=60000 = "Lighter 60%"
        // → Excel renders #B4C7E7.
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 40000 }), ALumOff(LuminanceOffset { val: 60000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#B4C7E7", 3), "got {out}");
    }

    #[test]
    fn accent1_lighter_80_matches_excel() {
        // Accent1 #4472C4, lumMod=20000 + lumOff=80000 = "Lighter 80%"
        // → Excel renders #D9E1F2.
        let block = "SchemeColor { val: Accent1, scheme_color_choice: [ALumMod(LuminanceModulation { val: 20000 }), ALumOff(LuminanceOffset { val: 80000 })] }";
        let out = apply_color_modifiers("#4472C4", block);
        assert!(approx_eq_hex(&out, "#D9E1F2", 3), "got {out}");
    }

    #[test]
    fn accent1_darker_25_matches_excel() {
        // Accent1 with lumMod=75% alone ("Darker 25%") → #3357A5-ish.
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
}

/// Resolve `accent{n}` against the workbook theme (slots 4..9 in our
/// spreadsheet-indexed `theme.colors`), falling back to the Office
/// 2007+ defaults when the theme didn't ship one.
pub(crate) fn theme_accent_color(n: u32, theme: Option<&Theme>) -> String {
    if let Some(t) = theme {
        let slot = 3 + n as usize; // accent1 -> theme.colors[4]
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
