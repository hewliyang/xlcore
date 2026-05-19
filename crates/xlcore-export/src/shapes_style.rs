use crate::schema::*;
use crate::shapes::preset_color_hex;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

fn resolve_ref_color_debug<T: std::fmt::Debug>(
    choice_opt: Option<&T>,
    theme: Option<&Theme>,
) -> Option<String> {
    let dbg = format!("{:?}", choice_opt?);

    if let Some(p) = dbg.find("RgbColorModelHex {") {
        let scope = crate::chart_colors::scope_from_open_brace(&dbg[p..]);
        if let Some(v) = scope.find("val: \"") {
            let body = &scope[v + 6..];
            if let Some(e) = body.find('"') {
                let hex = &body[..e];
                if hex.len() == 6 {
                    return Some(crate::chart_colors::apply_color_modifiers(
                        &format!("#{}", hex),
                        scope,
                    ));
                }
            }
        }
    }
    if let Some(p) = dbg.find("SchemeColor {") {
        let scope = crate::chart_colors::scope_from_open_brace(&dbg[p..]);
        if let Some(base) = crate::chart_colors::theme_scheme_color(scope, theme) {
            return Some(crate::chart_colors::apply_color_modifiers(&base, scope));
        }
    }
    if let Some(p) = dbg.find("SystemColor {") {
        let scope = crate::chart_colors::scope_from_open_brace(&dbg[p..]);
        if let Some(v) = scope.find("last_color: Some(\"") {
            let body = &scope[v + 18..];
            if let Some(e) = body.find('"') {
                let hex = &body[..e];
                if hex.len() == 6 {
                    return Some(format!("#{}", hex));
                }
            }
        }
    }
    if let Some(p) = dbg.find("PresetColor {") {
        let scope = crate::chart_colors::scope_from_open_brace(&dbg[p..]);
        if let Some(v) = scope.find("val: ") {
            let tail = &scope[v + 5..];

            let end = tail
                .find(|ch: char| ch == ',' || ch == ' ' || ch == '}')
                .unwrap_or(tail.len());
            if let Some(hex) = preset_color_hex(&tail[..end]) {
                return Some(hex.to_string());
            }
        }
    }
    None
}

pub(crate) struct StyleRefPaint {
    pub(crate) fill: Option<String>,
    pub(crate) outline: Option<String>,
    pub(crate) outline_width_emu: Option<i32>,
    pub(crate) font_name: Option<String>,
    pub(crate) font_color: Option<String>,
}

fn default_ln_ref_width(idx: u32) -> i32 {
    match idx {
        1 => 6_350,
        2 => 12_700,
        3 => 19_050,
        _ => 12_700,
    }
}

pub(crate) fn resolve_style_refs(
    style: Option<&xdr::ShapeStyle>,
    theme: Option<&Theme>,
) -> Option<StyleRefPaint> {
    let style = style?;

    let fill_idx: u32 = style.fill_reference.index;
    let fill = if fill_idx == 0 {
        None
    } else {
        resolve_ref_color_debug(style.fill_reference.fill_reference_choice.as_ref(), theme)
    };

    let ln_idx: u32 = style.line_reference.index;
    let (outline, outline_width_emu) = if ln_idx == 0 {
        (None, None)
    } else {
        let c = resolve_ref_color_debug(style.line_reference.line_reference_choice.as_ref(), theme);
        (c, Some(default_ln_ref_width(ln_idx)))
    };

    let font_name = match style.font_reference.index {
        a::FontCollectionIndexValues::Major => theme.and_then(|t| t.major_font.clone()),
        a::FontCollectionIndexValues::Minor => theme.and_then(|t| t.minor_font.clone()),
        a::FontCollectionIndexValues::None => None,
    };
    let font_color =
        resolve_ref_color_debug(style.font_reference.font_reference_choice.as_ref(), theme);

    Some(StyleRefPaint {
        fill,
        outline,
        outline_width_emu,
        font_name,
        font_color,
    })
}

pub(crate) fn apply_font_ref_to_runs(
    paragraphs: &mut [ShapeParagraph],
    font_name: &Option<String>,
    font_color: &Option<String>,
) {
    for p in paragraphs.iter_mut() {
        for r in p.runs.iter_mut() {
            if r.font_name.is_none() {
                if let Some(n) = font_name {
                    r.font_name = Some(n.clone());
                }
            }
            if r.color.is_none() {
                if let Some(hex) = font_color {
                    let stripped = hex.trim_start_matches('#');
                    if stripped.len() == 6 {
                        r.color = Some(Color {
                            rgb: Some(stripped.to_string()),
                            theme: None,
                            indexed: None,
                            tint: None,
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> Theme {
        Theme {
            colors: vec![
                "FFFFFF".into(),
                "000000".into(),
                "E7E6E6".into(),
                "44546A".into(),
                "4472C4".into(),
                "ED7D31".into(),
                "A5A5A5".into(),
                "FFC000".into(),
                "5B9BD5".into(),
                "70AD47".into(),
                "0563C1".into(),
                "954F72".into(),
            ],
            major_font: Some("Calibri Light".into()),
            minor_font: Some("Calibri".into()),
        }
    }

    fn rgb_choice(hex: &str) -> a::FillReferenceChoice {
        a::FillReferenceChoice::ASrgbClr(Box::new(a::RgbColorModelHex {
            val: hex.into(),
            ..Default::default()
        }))
    }

    fn scheme_choice(name: &str) -> a::FillReferenceChoice {
        let mut sc = a::SchemeColor::default();

        sc.val = match name {
            "accent1" => a::SchemeColorValues::Accent1,
            "accent2" => a::SchemeColorValues::Accent2,
            "accent3" => a::SchemeColorValues::Accent3,
            "accent4" => a::SchemeColorValues::Accent4,
            "accent5" => a::SchemeColorValues::Accent5,
            "accent6" => a::SchemeColorValues::Accent6,
            "bg1" | "lt1" => a::SchemeColorValues::Light1,
            "dk1" | "tx1" => a::SchemeColorValues::Dark1,
            _ => a::SchemeColorValues::Accent1,
        };
        a::FillReferenceChoice::ASchemeClr(Box::new(sc))
    }

    #[test]
    fn ref_color_resolves_srgb() {
        let c = rgb_choice("ABCDEF");
        let out = resolve_ref_color_debug(Some(&c), Some(&test_theme()));
        assert_eq!(out.as_deref(), Some("#ABCDEF"));
    }

    #[test]
    fn ref_color_resolves_accent_scheme() {
        let c = scheme_choice("accent1");
        let out = resolve_ref_color_debug(Some(&c), Some(&test_theme()));

        assert_eq!(out.as_deref(), Some("#4472C4"));
    }

    #[test]
    fn ref_color_each_accent_picks_correct_slot() {
        let theme = test_theme();
        let cases = [
            ("accent1", "#4472C4"),
            ("accent2", "#ED7D31"),
            ("accent3", "#A5A5A5"),
            ("accent4", "#FFC000"),
            ("accent5", "#5B9BD5"),
            ("accent6", "#70AD47"),
        ];
        for (name, expect) in cases {
            let c = scheme_choice(name);
            let out = resolve_ref_color_debug(Some(&c), Some(&theme));
            assert_eq!(out.as_deref(), Some(expect), "{name}");
        }
    }

    #[test]
    fn default_ln_ref_width_matches_standard_theme() {
        assert_eq!(default_ln_ref_width(1), 6_350);
        assert_eq!(default_ln_ref_width(2), 12_700);
        assert_eq!(default_ln_ref_width(3), 19_050);

        assert_eq!(default_ln_ref_width(99), 12_700);
    }
}
