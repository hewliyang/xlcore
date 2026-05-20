use crate::schema::*;
use crate::shapes::{preset_color_hex, prst_dash_token, resolve_solid_fill};
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

pub(crate) fn solid_fill_color(
    choice: &Option<xdr::ShapePropertiesChoice2>,
    theme: Option<&Theme>,
) -> Option<String> {
    use xdr::ShapePropertiesChoice2;
    match choice.as_ref()? {
        ShapePropertiesChoice2::ASolidFill(sf) => resolve_solid_fill(sf, theme),

        _ => None,
    }
}

fn resolve_gradient_stop_color(c: &a::GradientStopChoice, theme: Option<&Theme>) -> Option<String> {
    use a::GradientStopChoice as G;
    match c {
        G::ASrgbClr(c) => {
            let dbg = format!("{:?}", c);
            let v: &str = &c.val;
            if v.len() == 6 {
                Some(crate::chart_colors::apply_color_modifiers(
                    &format!("#{}", v),
                    &dbg,
                ))
            } else {
                None
            }
        }
        G::ASchemeClr(c) => {
            let dbg = format!("{:?}", c);
            crate::chart_colors::theme_scheme_color(&dbg, theme)
                .map(|base| crate::chart_colors::apply_color_modifiers(&base, &dbg))
        }
        G::APrstClr(c) => {
            let dbg = format!("{:?}", c.val);
            preset_color_hex(&dbg).map(|s| s.to_string())
        }
        G::ASysClr(c) => c.last_color.as_deref().map(|s| format!("#{}", s)),
        _ => None,
    }
}

fn rect_pct(v: Option<i32>) -> f64 {
    (v.unwrap_or(0) as f64) / 100_000.0
}

pub(crate) fn gradient_fill(
    choice: &Option<xdr::ShapePropertiesChoice2>,
    theme: Option<&Theme>,
) -> Option<ShapeGradient> {
    use xdr::ShapePropertiesChoice2;
    let gf = match choice.as_ref()? {
        ShapePropertiesChoice2::AGradFill(g) => g,
        _ => return None,
    };
    let mut stops: Vec<ShapeGradientStop> = Vec::new();
    if let Some(gs_lst) = gf.gradient_stop_list.as_ref() {
        for gs in &gs_lst.a_gs {
            let pos = (gs.position as f32) / 100_000.0;
            let color = match gs.gradient_stop_choice.as_ref() {
                Some(c) => resolve_gradient_stop_color(c, theme),
                None => None,
            };
            if let Some(color) = color {
                stops.push(ShapeGradientStop { pos, color });
            }
        }
    }
    if stops.len() < 2 {
        return None;
    }

    use a::GradientFillChoice as GC;
    match gf.gradient_fill_choice.as_ref() {
        Some(GC::ALin(lin)) => {
            let ang = lin.angle.unwrap_or(0);

            let angle_deg = (ang as f64) / 60_000.0;
            Some(ShapeGradient {
                stops,
                kind: "linear".to_string(),
                angle_deg: Some(angle_deg),
                path: None,
                fill_to_rect: None,
            })
        }
        Some(GC::APath(p)) => {
            let path = p
                .path
                .as_ref()
                .map(|v| format!("{:?}", v).to_ascii_lowercase());
            let ftr = p.fill_to_rectangle.as_ref().map(|r| {
                vec![
                    rect_pct(r.left),
                    rect_pct(r.top),
                    rect_pct(r.right),
                    rect_pct(r.bottom),
                ]
            });
            Some(ShapeGradient {
                stops,
                kind: "path".to_string(),
                angle_deg: None,
                path,
                fill_to_rect: ftr,
            })
        }
        None => Some(ShapeGradient {
            stops,
            kind: "linear".to_string(),
            angle_deg: Some(0.0),
            path: None,
            fill_to_rect: None,
        }),
    }
}

fn outer_shadow_color(
    choice: &a::OuterShadowChoice,
    theme: Option<&Theme>,
) -> Option<(String, f32)> {
    use a::OuterShadowChoice as C;
    let (hex, scope) = match choice {
        C::ASrgbClr(c) => {
            let dbg = format!("{:?}", c);
            let v: &str = &c.val;
            if v.len() != 6 {
                return None;
            }
            let base = format!("#{}", v);
            (crate::chart_colors::apply_color_modifiers(&base, &dbg), dbg)
        }
        C::ASchemeClr(c) => {
            let dbg = format!("{:?}", c);
            let base = crate::chart_colors::theme_scheme_color(&dbg, theme)?;
            (crate::chart_colors::apply_color_modifiers(&base, &dbg), dbg)
        }
        C::APrstClr(c) => {
            let dbg = format!("{:?}", c);
            let val_dbg = format!("{:?}", c.val);
            let hex = preset_color_hex(&val_dbg)?.to_string();
            (hex, dbg)
        }
        C::ASysClr(c) => {
            let dbg = format!("{:?}", c);
            let hex = c.last_color.as_deref().map(|s| format!("#{}", s))?;
            (hex, dbg)
        }
        _ => return None,
    };

    let alpha = sniff_alpha_modifier(&scope).unwrap_or(1.0);
    Some((hex, alpha))
}

fn sniff_alpha_modifier(scope: &str) -> Option<f32> {
    let needle = "AAlpha(Alpha { val: ";
    let p = scope.find(needle)?;
    let tail = &scope[p + needle.len()..];
    let end = tail.find(|c: char| !c.is_ascii_digit() && c != '-')?;
    let raw: i64 = tail[..end].parse().ok()?;
    Some((raw as f32 / 100_000.0).clamp(0.0, 1.0))
}

pub(crate) fn outer_shadow(
    choice: &Option<xdr::ShapePropertiesChoice3>,
    theme: Option<&Theme>,
) -> Option<ShapeOuterShadow> {
    use xdr::ShapePropertiesChoice3;
    let lst = match choice.as_ref()? {
        ShapePropertiesChoice3::AEffectLst(l) => l,
        ShapePropertiesChoice3::AEffectDag(_) => return None,
    };
    let sh = lst.outer_shadow.as_ref()?;
    let (color, alpha) = match sh.outer_shadow_choice.as_ref() {
        Some(c) => outer_shadow_color(c, theme)?,
        None => ("#000000".to_string(), 1.0),
    };
    let blur_emu = sh.blur_radius.unwrap_or(0);
    let dist_emu = sh.distance.unwrap_or(0);
    let dir_raw = sh.direction.unwrap_or(0);
    let dir_deg = (dir_raw as f32) / 60_000.0;
    if blur_emu == 0 && dist_emu == 0 {
        return None;
    }
    Some(ShapeOuterShadow {
        color,
        alpha,
        blur_emu,
        dist_emu,
        dir_deg,
    })
}

pub(crate) fn outline_info(
    ln: Option<&a::Outline>,
    theme: Option<&Theme>,
) -> (Option<String>, Option<i32>) {
    let Some(ln) = ln else {
        return (None, None);
    };
    let width = ln.width;
    use a::OutlineChoice;
    let color = match ln.outline_choice1.as_ref() {
        Some(OutlineChoice::ASolidFill(sf)) => resolve_solid_fill(sf, theme),
        Some(OutlineChoice::ANoFill(_)) => None,
        _ => None,
    };
    (color, width)
}

pub(crate) fn line_dash_token(ln: Option<&a::Outline>) -> Option<String> {
    let ln = ln?;
    use a::OutlineChoice2;
    match ln.outline_choice2.as_ref()? {
        OutlineChoice2::APrstDash(d) => d.val.as_ref().map(|v| prst_dash_token(v).to_string()),
        _ => None,
    }
}

pub(crate) fn line_cap_token(ln: Option<&a::Outline>) -> Option<String> {
    let cap = ln?.cap_type.as_ref()?;
    enum_token(&format!("{:?}", cap))
}

pub(crate) fn line_join_token(ln: Option<&a::Outline>) -> Option<String> {
    let c = ln?.outline_choice3.as_ref()?;
    Some(
        match c {
            a::OutlineChoice3::ARound => "round",
            a::OutlineChoice3::ABevel => "bevel",
            a::OutlineChoice3::AMiter(_) => "miter",
        }
        .to_string(),
    )
}

pub(crate) fn line_end_to_schema(
    kind: Option<&a::LineEndValues>,
    w: Option<&a::LineEndWidthValues>,
    len: Option<&a::LineEndLengthValues>,
) -> Option<LineEnd> {
    let kind_tok = kind.and_then(|v| enum_token(&format!("{:?}", v)));
    let w_tok = w.and_then(|v| enum_token(&format!("{:?}", v)));
    let len_tok = len.and_then(|v| enum_token(&format!("{:?}", v)));

    if matches!(kind_tok.as_deref(), Some("none")) && w_tok.is_none() && len_tok.is_none() {
        return None;
    }
    if kind_tok.is_none() && w_tok.is_none() && len_tok.is_none() {
        return None;
    }
    Some(LineEnd {
        kind: kind_tok,
        w: w_tok,
        len: len_tok,
    })
}

fn enum_token(dbg: &str) -> Option<String> {
    let trimmed = dbg.trim_end_matches('_');
    if trimmed.is_empty() {
        return None;
    }
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    let rest: String = chars.collect();
    Some(format!("{}{}", first.to_ascii_lowercase(), rest))
}
