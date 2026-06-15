use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;

use crate::chart_colors::{apply_color_modifiers, scope_from_open_brace, theme_scheme_color};
use crate::schema::{ShapeGradient, ShapeGradientStop, ShapeOuterShadow, Theme};
use crate::shapes::preset_color_hex;

#[derive(Clone, Debug, Default)]
pub struct FmtScheme {
    pub fills: Vec<FmtFill>,
    pub lines: Vec<FmtLine>,
    pub effects: Vec<FmtEffect>,
}

#[derive(Clone, Debug, Default)]
pub enum FmtFill {
    #[default]
    None,
    Solid(FmtColor),
    Gradient(FmtGradient),
    Other,
}

#[derive(Clone, Debug)]
pub struct FmtGradient {
    pub stops: Vec<(f32, FmtColor)>,
    pub kind: String,
    pub angle_deg: Option<f64>,
    pub path: Option<String>,
    pub fill_to_rect: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Default)]
pub struct FmtLine {
    pub width_emu: Option<i32>,
    pub dash: Option<String>,
    pub cap: Option<String>,
    pub join: Option<String>,
    pub fill: Option<FmtFill>,
}

#[derive(Clone, Debug, Default)]
pub struct FmtEffect {
    pub outer_shadow: Option<FmtOuterShadow>,
}

#[derive(Clone, Debug)]
pub struct FmtOuterShadow {
    pub color: FmtColor,
    pub blur_emu: i64,
    pub dist_emu: i64,
    pub dir_deg: f32,
}

#[derive(Clone, Debug)]
pub struct FmtColor {
    pub source: FmtColorSource,
    pub modifiers_scope: String,
}

#[derive(Clone, Debug)]
pub enum FmtColorSource {
    PhClr,
    Resolved(String),
}

pub fn extract(theme: &a::Theme) -> FmtScheme {
    let fmt = &theme.theme_elements.format_scheme;

    let mut fills = Vec::new();
    for choice in &fmt.fill_style_list.fill_style_list_choice {
        fills.push(extract_fill_choice(choice));
    }

    let mut lines = Vec::new();
    for ln in &fmt.line_style_list.outline {
        lines.push(extract_line(ln));
    }

    let mut effects = Vec::new();
    for es in &fmt.effect_style_list.effect_style {
        effects.push(extract_effect(es));
    }

    FmtScheme {
        fills,
        lines,
        effects,
    }
}

fn extract_fill_choice(c: &a::FillStyleListChoice) -> FmtFill {
    use a::FillStyleListChoice as F;
    match c {
        F::NoFill(_) => FmtFill::None,
        F::SolidFill(sf) => match sf.solid_fill_choice.as_ref() {
            Some(c) => FmtColor::from_solid_choice(c)
                .map(FmtFill::Solid)
                .unwrap_or(FmtFill::Other),
            None => FmtFill::Other,
        },
        F::GradientFill(g) => extract_gradient(g)
            .map(FmtFill::Gradient)
            .unwrap_or(FmtFill::Other),
        F::BlipFill(_) | F::PatternFill(_) | F::GroupFill => FmtFill::Other,
    }
}

fn extract_gradient(g: &a::GradientFill) -> Option<FmtGradient> {
    let mut stops: Vec<(f32, FmtColor)> = Vec::new();
    if let Some(lst) = g.gradient_stop_list.as_ref() {
        for gs in &lst.gradient_stop {
            let pos = (gs.position.as_drawingml_percent() as f32) / 100_000.0;
            if let Some(c) = gs.gradient_stop_choice.as_ref() {
                if let Some(fc) = FmtColor::from_gradient_stop_choice(c) {
                    stops.push((pos, fc));
                }
            }
        }
    }
    if stops.len() < 2 {
        return None;
    }
    use a::GradientFillChoice as GC;
    let (kind, angle_deg, path, fill_to_rect) = match g.gradient_fill_choice.as_ref() {
        Some(GC::LinearGradientFill(lin)) => (
            "linear".to_string(),
            Some((lin.angle.unwrap_or(0) as f64) / 60_000.0),
            None,
            None,
        ),
        Some(GC::PathGradientFill(p)) => {
            let path_tok = p
                .path
                .as_ref()
                .map(|v| format!("{:?}", v).to_ascii_lowercase());
            let ftr = p.fill_to_rectangle.as_ref().map(|r| {
                vec![
                    rect_pct(r.left.map(|v| v.as_drawingml_percent())),
                    rect_pct(r.top.map(|v| v.as_drawingml_percent())),
                    rect_pct(r.right.map(|v| v.as_drawingml_percent())),
                    rect_pct(r.bottom.map(|v| v.as_drawingml_percent())),
                ]
            });
            ("path".to_string(), None, path_tok, ftr)
        }
        None => ("linear".to_string(), Some(0.0), None, None),
    };
    Some(FmtGradient {
        stops,
        kind,
        angle_deg,
        path,
        fill_to_rect,
    })
}

fn extract_line(ln: &a::Outline) -> FmtLine {
    let width_emu = ln.width;
    let cap = ln
        .cap_type
        .as_ref()
        .and_then(|v| enum_token(&format!("{:?}", v)));
    let join = ln.outline_choice3.as_ref().map(|c| match c {
        a::OutlineChoice3::Round => "round".to_string(),
        a::OutlineChoice3::LineJoinBevel => "bevel".to_string(),
        a::OutlineChoice3::Miter(_) => "miter".to_string(),
    });
    let dash = ln.outline_choice2.as_ref().and_then(|c| match c {
        a::OutlineChoice2::PresetDash(d) => d
            .val
            .as_ref()
            .map(|v| crate::shapes::prst_dash_token(v).to_string()),
        _ => None,
    });
    let fill = ln.outline_choice1.as_ref().and_then(|c| match c {
        a::OutlineChoice::SolidFill(sf) => sf
            .solid_fill_choice
            .as_ref()
            .and_then(FmtColor::from_solid_choice)
            .map(FmtFill::Solid),
        a::OutlineChoice::GradientFill(g) => extract_gradient(g).map(FmtFill::Gradient),
        a::OutlineChoice::NoFill(_) => Some(FmtFill::None),
        a::OutlineChoice::PatternFill(_) => Some(FmtFill::Other),
    });

    FmtLine {
        width_emu,
        dash,
        cap,
        join,
        fill,
    }
}

fn extract_effect(es: &a::EffectStyle) -> FmtEffect {
    let mut out = FmtEffect::default();
    if let Some(a::EffectStyleChoice::EffectList(lst)) = es.effect_style_choice.as_ref() {
        if let Some(sh) = lst.outer_shadow.as_ref() {
            let color = match sh.outer_shadow_choice.as_ref() {
                Some(c) => FmtColor::from_outer_shadow_choice(c),
                None => None,
            };
            if let Some(color) = color {
                let blur_emu = sh.blur_radius.map(|v| v.to_emu()).unwrap_or(0);
                let dist_emu = sh.distance.map(|v| v.to_emu()).unwrap_or(0);
                let dir_deg = (sh.direction.unwrap_or(0) as f32) / 60_000.0;
                out.outer_shadow = Some(FmtOuterShadow {
                    color,
                    blur_emu,
                    dist_emu,
                    dir_deg,
                });
            }
        }
    }
    out
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

fn rect_pct(v: Option<i32>) -> f64 {
    (v.unwrap_or(0) as f64) / 100_000.0
}

impl FmtColor {
    fn from_solid_choice(c: &a::SolidFillChoice) -> Option<FmtColor> {
        use a::SolidFillChoice as S;
        match c {
            S::SchemeColor(sc) => Some(scheme_to_fmt(sc)),
            S::RgbColorModelHex(c) => Some(srgb_to_fmt(c)),
            S::PresetColor(c) => prst_to_fmt(c),
            S::SystemColor(c) => sys_to_fmt(c),
            _ => None,
        }
    }
    fn from_gradient_stop_choice(c: &a::GradientStopChoice) -> Option<FmtColor> {
        use a::GradientStopChoice as G;
        match c {
            G::SchemeColor(sc) => Some(scheme_to_fmt(sc)),
            G::RgbColorModelHex(c) => Some(srgb_to_fmt(c)),
            G::PresetColor(c) => prst_to_fmt(c),
            G::SystemColor(c) => sys_to_fmt(c),
            _ => None,
        }
    }
    fn from_outer_shadow_choice(c: &a::OuterShadowChoice) -> Option<FmtColor> {
        use a::OuterShadowChoice as O;
        match c {
            O::SchemeColor(sc) => Some(scheme_to_fmt(sc)),
            O::RgbColorModelHex(c) => Some(srgb_to_fmt(c)),
            O::PresetColor(c) => prst_to_fmt(c),
            O::SystemColor(c) => sys_to_fmt(c),
            _ => None,
        }
    }
}

fn scheme_to_fmt(sc: &a::SchemeColor) -> FmtColor {
    let dbg_full = format!("{:?}", sc);
    let scope = scope_from_open_brace(&dbg_full).to_string();
    let is_ph = matches!(sc.val, a::SchemeColorValues::PhColor);
    let source = if is_ph {
        FmtColorSource::PhClr
    } else {
        FmtColorSource::Resolved(format!("scheme:{:?}", sc.val))
    };
    FmtColor {
        source,
        modifiers_scope: scope,
    }
}

fn srgb_to_fmt(c: &a::RgbColorModelHex) -> FmtColor {
    let dbg = format!("{:?}", c);
    let scope = scope_from_open_brace(&dbg).to_string();
    let v: &str = &c.val;
    let hex = if v.len() == 6 {
        format!("#{}", v)
    } else {
        "#000000".to_string()
    };
    FmtColor {
        source: FmtColorSource::Resolved(hex),
        modifiers_scope: scope,
    }
}

fn prst_to_fmt(c: &a::PresetColor) -> Option<FmtColor> {
    let dbg = format!("{:?}", c);
    let scope = scope_from_open_brace(&dbg).to_string();
    let val_dbg = format!("{:?}", c.val);
    let hex = preset_color_hex(&val_dbg)?.to_string();
    Some(FmtColor {
        source: FmtColorSource::Resolved(hex),
        modifiers_scope: scope,
    })
}

fn sys_to_fmt(c: &a::SystemColor) -> Option<FmtColor> {
    let dbg = format!("{:?}", c);
    let scope = scope_from_open_brace(&dbg).to_string();
    let last: &str = c.last_color.as_deref()?;
    Some(FmtColor {
        source: FmtColorSource::Resolved(format!("#{}", last)),
        modifiers_scope: scope,
    })
}

pub(crate) fn sniff_alpha_modifier(scope: &str) -> Option<f32> {
    let needle = "AAlpha(Alpha { val: ";
    let p = scope.find(needle)?;
    let tail = &scope[p + needle.len()..];
    let end = tail.find(|c: char| !c.is_ascii_digit() && c != '-')?;
    let raw: i64 = tail[..end].parse().ok()?;
    Some((raw as f32 / 100_000.0).clamp(0.0, 1.0))
}

pub fn resolve_fmt_color(
    fc: &FmtColor,
    ph_hex: &str,
    theme: Option<&Theme>,
) -> Option<(String, f32)> {
    let base = match &fc.source {
        FmtColorSource::PhClr => ph_hex.to_string(),
        FmtColorSource::Resolved(s) => {
            if let Some(rest) = s.strip_prefix("scheme:") {
                let synthetic = format!("{{ val: {} }}", rest);
                theme_scheme_color(&synthetic, theme).unwrap_or_else(|| "#000000".to_string())
            } else {
                s.clone()
            }
        }
    };
    let modded = apply_color_modifiers(&base, &fc.modifiers_scope);
    let alpha = sniff_alpha_modifier(&fc.modifiers_scope).unwrap_or(1.0);
    Some((modded, alpha))
}

pub fn realize_fill(
    f: &FmtFill,
    ph_hex: &str,
    theme: Option<&Theme>,
) -> (Option<String>, Option<ShapeGradient>) {
    match f {
        FmtFill::None | FmtFill::Other => (None, None),
        FmtFill::Solid(c) => (
            resolve_fmt_color(c, ph_hex, theme).map(|(hex, _)| hex),
            None,
        ),
        FmtFill::Gradient(g) => {
            let mut stops: Vec<ShapeGradientStop> = Vec::new();
            for (pos, c) in &g.stops {
                if let Some((hex, _alpha)) = resolve_fmt_color(c, ph_hex, theme) {
                    stops.push(ShapeGradientStop {
                        pos: *pos,
                        color: hex,
                    });
                }
            }
            if stops.len() < 2 {
                return (None, None);
            }
            (
                None,
                Some(ShapeGradient {
                    stops,
                    kind: g.kind.clone(),
                    angle_deg: g.angle_deg,
                    path: g.path.clone(),
                    fill_to_rect: g.fill_to_rect.clone(),
                }),
            )
        }
    }
}

pub fn realize_outer_shadow(
    e: &FmtEffect,
    ph_hex: &str,
    theme: Option<&Theme>,
) -> Option<ShapeOuterShadow> {
    let sh = e.outer_shadow.as_ref()?;
    let (color, alpha) = resolve_fmt_color(&sh.color, ph_hex, theme)?;
    if sh.blur_emu == 0 && sh.dist_emu == 0 {
        return None;
    }
    Some(ShapeOuterShadow {
        color,
        alpha,
        blur_emu: sh.blur_emu,
        dist_emu: sh.dist_emu,
        dir_deg: sh.dir_deg,
    })
}
