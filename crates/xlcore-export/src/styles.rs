//! Styles extraction from `<x:styleSheet>`.
//!
//! ooxmlsdk uses distinct types per border side (LeftBorder/RightBorder/...)
//! so we can't share a single helper without traits — repeat ourselves and
//! keep it readable.

use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
// Disambiguate ooxmlsdk types from our schema types of the same name.
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main::{
    Border as XBorder, CellFormat as XCellFormat, DifferentialFormat as XDxf, Fill as XFill,
    Font as XFont,
};

pub fn extract(s: &x::Stylesheet) -> Styles {
    let fonts: Vec<Font> = s
        .fonts
        .as_ref()
        .map(|f| f.x_font.iter().map(extract_font).collect())
        .unwrap_or_default();

    let fills: Vec<Fill> = s
        .fills
        .as_ref()
        .map(|f| f.x_fill.iter().map(extract_fill).collect())
        .unwrap_or_default();

    let borders: Vec<Border> = s
        .borders
        .as_ref()
        .map(|b| b.x_border.iter().map(extract_border).collect())
        .unwrap_or_default();

    // Parse `<cellStyleXfs>` first — these are the "named style" parents
    // (e.g. "Normal", "Title", "Heading 1") that cell xfs inherit from
    // when they set `applyFont="0"` / `applyFill="0"` / etc. and point
    // back via `xfId`. ECMA-376 §18.8.10. They're never referenced
    // directly by cells — only through inheritance — so we don't expose
    // them on `Styles`; we just resolve and discard.
    let cell_style_xfs: Vec<CellFormat> = s
        .cell_style_formats
        .as_ref()
        .map(|cf| cf.x_xf.iter().map(extract_xf).collect())
        .unwrap_or_default();

    let cell_xfs: Vec<CellFormat> = s
        .cell_formats
        .as_ref()
        .map(|cf| {
            cf.x_xf
                .iter()
                .map(|xf| extract_xf_with_inheritance(xf, &cell_style_xfs))
                .collect()
        })
        .unwrap_or_default();

    let num_fmts: Vec<NumberFormat> = s
        .numbering_formats
        .as_ref()
        .map(|n| {
            n.x_num_fmt
                .iter()
                .map(|nf| NumberFormat {
                    id: nf.number_format_id,
                    format_code: nf.format_code.as_str().to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    let default_font = fonts
        .first()
        .and_then(|f| f.name.clone())
        .unwrap_or_else(|| "Calibri".to_string());
    let default_font_size = fonts.first().and_then(|f| f.size).unwrap_or(11.0);

    Styles {
        fonts,
        fills,
        borders,
        cell_xfs,
        num_fmts,
        default_font,
        default_font_size,
    }
}

/// Extract the workbook's `<dxfs>` block (differential formats). These
/// are referenced by `CfRule.dxf_id` and table styles. We only carry the
/// subset the renderer actually uses today: font color/bold/italic/
/// underline/strike, solid fill foreground, and num-format override.
pub fn extract_dxfs(s: &x::Stylesheet) -> Vec<crate::schema::Dxf> {
    let Some(dxfs) = s.differential_formats.as_ref() else {
        return Vec::new();
    };
    dxfs.x_dxf.iter().map(extract_dxf).collect()
}

/// Extract the workbook's `<tableStyles>` block — named custom table
/// styles (workbooks may ship their own beyond Excel's built-ins).
/// We resolve each style's element list (`<tableStyleElement type="…"
/// dxfId="N"/>`) into the named slots on `CustomTableStyle` that the
/// renderer actually consults. The dxfId references the parallel
/// `<dxfs>` block surfaced by `extract_dxfs`.
pub fn extract_table_styles(s: &x::Stylesheet) -> Vec<crate::schema::CustomTableStyle> {
    let Some(ts) = s.table_styles.as_ref() else {
        return Vec::new();
    };
    ts.x_table_style
        .iter()
        .map(|style| {
            let mut out = crate::schema::CustomTableStyle {
                name: style.name.as_str().to_string(),
                ..Default::default()
            };
            for el in &style.x_table_style_element {
                let Some(dxf_id) = el.format_id else { continue };
                use x::TableStyleValues as T;
                match el.r#type {
                    T::WholeTable => out.whole_table = Some(dxf_id),
                    T::HeaderRow => out.header_row = Some(dxf_id),
                    T::TotalRow => out.total_row = Some(dxf_id),
                    T::FirstRowStripe => out.first_row_stripe = Some(dxf_id),
                    T::SecondRowStripe => out.second_row_stripe = Some(dxf_id),
                    T::FirstColumn => out.first_column = Some(dxf_id),
                    T::LastColumn => out.last_column = Some(dxf_id),
                    // Bands we don't paint yet (column stripes, subtotal
                    // rows, page-field cells, header/total corner cells)
                    // are dropped on the floor. Add a field when the
                    // renderer learns to paint that band.
                    _ => {}
                }
            }
            out
        })
        .collect()
}

fn extract_dxf(d: &XDxf) -> crate::schema::Dxf {
    let mut out = crate::schema::Dxf::default();
    if let Some(f) = d.font.as_ref() {
        if let Some(b) = f.bold.as_ref() {
            out.bold = Some(b.val.unwrap_or(true));
        }
        if let Some(i) = f.italic.as_ref() {
            out.italic = Some(i.val.unwrap_or(true));
        }
        if let Some(s) = f.strike.as_ref() {
            out.strike = Some(s.val.unwrap_or(true));
        }
        if let Some(u) = f.underline.as_ref() {
            match crate::underline_variant(u.val) {
                Some("none") => {}
                Some(v) => {
                    out.underline = Some(true);
                    if v != "single" {
                        out.underline_style = Some(v.to_string());
                    }
                }
                None => {
                    out.underline = Some(true);
                }
            }
        }
        if let Some(c) = f.color.as_ref() {
            out.font_color = extract_color_x(c);
        }
        if let Some(v) = f.vertical_text_alignment.as_ref() {
            out.vert_align = crate::vert_align_variant(v.val);
        }
    }
    if let Some(fill) = d.fill.as_ref() {
        if let Some(x::FillChoice::XPatternFill(pf)) = &fill.fill_choice {
            // dxf fills are weird: when the pattern is `solid` Excel
            // stores the visible color in `bgColor`, not `fgColor`.
            // (Same quirk SpreadJS works around in convertDxfToStyle.)
            // Try fg first, then bg, so non-solid patterns still resolve.
            let fg = pf.foreground_color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            let bg = pf.background_color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            out.fill_color = fg.or(bg);
        }
    }
    if let Some(nf) = d.numbering_format.as_ref() {
        out.num_fmt = Some(nf.format_code.as_str().to_string());
    }
    out
}

fn extract_font(f: &XFont) -> Font {
    // CT_BooleanProperty: element present with no `val` attr defaults to
    // true; `val="0"`/`val="false"` *unsets* the property. ooxmlsdk parses
    // BooleanValue via Display/Debug; convert through string compare since
    // the inner repr isn't part of the public API.
    Font {
        name: f.font_name.as_ref().map(|n| n.val.as_str().to_string()),
        size: f.font_size.as_ref().map(|s| s.val as f32),
        bold: f
            .bold
            .as_ref()
            .map(|b| b.val.unwrap_or(true))
            .unwrap_or(false),
        italic: f
            .italic
            .as_ref()
            .map(|i| i.val.unwrap_or(true))
            .unwrap_or(false),
        underline: match f.underline.as_ref() {
            Some(u) => !matches!(crate::underline_variant(u.val), Some("none")),
            None => false,
        },
        underline_style: f
            .underline
            .as_ref()
            .and_then(|u| match crate::underline_variant(u.val) {
                Some(v) if v != "single" && v != "none" => Some(v.to_string()),
                _ => None,
            }),
        strike: f
            .strike
            .as_ref()
            .map(|s| s.val.unwrap_or(true))
            .unwrap_or(false),
        color: f.color.as_ref().and_then(extract_color_x),
        vert_align: f
            .vertical_text_alignment
            .as_ref()
            .and_then(|v| crate::vert_align_variant(v.val)),
        family: f.font_family_numbering.as_ref().and_then(|fm| {
            let v = fm.val;
            if (0..=5).contains(&v) {
                Some(v as u8)
            } else {
                None
            }
        }),
        scheme: f
            .font_scheme
            .as_ref()
            .and_then(|s| crate::font_scheme_variant(s.val)),
    }
}

fn extract_fill(f: &XFill) -> Fill {
    match &f.fill_choice {
        Some(x::FillChoice::XPatternFill(pf)) => {
            let pattern_type = pf
                .pattern_type
                .as_ref()
                .map(|p| pattern_type_to_str(p).to_string());
            let fg_color = pf.foreground_color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            let bg_color = pf.background_color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            Fill {
                pattern_type,
                fg_color,
                bg_color,
                ..Fill::default()
            }
        }
        Some(x::FillChoice::XGradientFill(gf)) => {
            use crate::schema::GradientStop as Gs;
            let gradient_stops: Vec<Gs> = gf
                .x_stop
                .iter()
                .filter_map(|stop| {
                    extract_color_x(&stop.color).map(|color| Gs {
                        position: stop.position,
                        color,
                    })
                })
                .collect();
            let gradient_type = match gf.r#type {
                Some(x::GradientValues::Path) => Some("path".to_string()),
                _ => Some("linear".to_string()),
            };
            // OOXML `degree` is required by the schema but optional in our
            // model; we omit it (defaults to 0 = left→right) when missing.
            let gradient_degree = gf.degree;
            let conv = |v: &Option<ooxmlsdk::simple_type::DoubleValue>| *v;
            // The path-convergence attrs (left/right/top/bottom) only make
            // sense for `path` gradients; suppress them otherwise so the
            // JSON stays clean.
            let is_path = matches!(gf.r#type, Some(x::GradientValues::Path));
            Fill {
                pattern_type: Some("gradient".to_string()),
                fg_color: gradient_stops.first().map(|s| s.color.clone()),
                bg_color: gradient_stops.last().map(|s| s.color.clone()),
                gradient_stops,
                gradient_type,
                gradient_degree: if is_path { None } else { gradient_degree },
                gradient_left: if is_path { conv(&gf.left) } else { None },
                gradient_right: if is_path { conv(&gf.right) } else { None },
                gradient_top: if is_path { conv(&gf.top) } else { None },
                gradient_bottom: if is_path { conv(&gf.bottom) } else { None },
            }
        }
        None => Fill::default(),
    }
}

fn pattern_type_to_str(p: &x::PatternValues) -> &'static str {
    use x::PatternValues as P;
    match p {
        P::None => "none",
        P::Solid => "solid",
        P::MediumGray => "mediumGray",
        P::DarkGray => "darkGray",
        P::LightGray => "lightGray",
        P::DarkHorizontal => "darkHorizontal",
        P::DarkVertical => "darkVertical",
        P::DarkDown => "darkDown",
        P::DarkUp => "darkUp",
        P::DarkGrid => "darkGrid",
        P::DarkTrellis => "darkTrellis",
        P::LightHorizontal => "lightHorizontal",
        P::LightVertical => "lightVertical",
        P::LightDown => "lightDown",
        P::LightUp => "lightUp",
        P::LightGrid => "lightGrid",
        P::LightTrellis => "lightTrellis",
        P::Gray125 => "gray125",
        P::Gray0625 => "gray0625",
    }
}

fn extract_border(b: &XBorder) -> Border {
    Border {
        left: b.left_border.as_ref().and_then(|s| {
            let style = border_style_str(s.style.as_ref().map(|s| format!("{s:?}")).as_deref());
            let color = s.color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            style.map(|st| BorderLine { style: st, color })
        }),
        right: b.right_border.as_ref().and_then(|s| {
            let style = border_style_str(s.style.as_ref().map(|s| format!("{s:?}")).as_deref());
            let color = s.color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            style.map(|st| BorderLine { style: st, color })
        }),
        top: b.top_border.as_ref().and_then(|s| {
            let style = border_style_str(s.style.as_ref().map(|s| format!("{s:?}")).as_deref());
            let color = s.color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            style.map(|st| BorderLine { style: st, color })
        }),
        bottom: b.bottom_border.as_ref().and_then(|s| {
            let style = border_style_str(s.style.as_ref().map(|s| format!("{s:?}")).as_deref());
            let color = s.color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            style.map(|st| BorderLine { style: st, color })
        }),
        diagonal_up: b.diagonal_up.unwrap_or(false),
        diagonal_down: b.diagonal_down.unwrap_or(false),
        diagonal: b.diagonal_border.as_ref().and_then(|s| {
            let style = border_style_str(s.style.as_ref().map(|s| format!("{s:?}")).as_deref());
            let color = s.color.as_ref().and_then(|c| {
                extract_color_x(&x::Color {
                    auto: c.auto,
                    indexed: c.indexed,
                    rgb: c.rgb.clone(),
                    theme: c.theme,
                    tint: c.tint,
                })
            });
            style.map(|st| BorderLine { style: st, color })
        }),
    }
}

fn border_style_str(dbg: Option<&str>) -> Option<String> {
    let dbg = dbg?;
    let lower = dbg.to_ascii_lowercase();
    // Order matters: longer / more-specific names must be tested
    // before any shorter substring they contain. Notably
    // `slantDashDot` contains `dashDot`, and `mediumDashDotDot`
    // contains `mediumDashDot` which contains `mediumDashed` is
    // false but contains `mediumDash` (not a real value). The
    // ordering below is the topological sort.
    let s = if lower.contains("none") {
        return None;
    } else if lower.contains("slantdashdot") {
        "slantDashDot"
    } else if lower.contains("mediumdashdotdot") {
        "mediumDashDotDot"
    } else if lower.contains("mediumdashdot") {
        "mediumDashDot"
    } else if lower.contains("mediumdashed") {
        "mediumDashed"
    } else if lower.contains("medium") {
        "medium"
    } else if lower.contains("thick") {
        "thick"
    } else if lower.contains("double") {
        "double"
    } else if lower.contains("dotted") {
        "dotted"
    } else if lower.contains("dashdotdot") {
        "dashDotDot"
    } else if lower.contains("dashdot") {
        "dashDot"
    } else if lower.contains("dashed") {
        "dashed"
    } else if lower.contains("hair") {
        "hair"
    } else if lower.contains("thin") {
        "thin"
    } else {
        return None;
    };
    Some(s.to_string())
}

/// Apply `apply*="0"` inheritance from the cell xf's parent
/// `cellStyleXf` (resolved via `xfId`). When the flag is missing or
/// `"1"`, the cell xf's own value wins; when `"0"`, the parent style's
/// value is used. ECMA-376 §18.8.45.
fn extract_xf_with_inheritance(xf: &XCellFormat, parents: &[CellFormat]) -> CellFormat {
    let mut cf = extract_xf(xf);
    let parent = xf.format_id.and_then(|id| parents.get(id as usize));
    let Some(parent) = parent else {
        return cf;
    };
    if xf.apply_font == Some(false) {
        cf.font_id = parent.font_id;
    }
    if xf.apply_fill == Some(false) {
        cf.fill_id = parent.fill_id;
    }
    if xf.apply_border == Some(false) {
        cf.border_id = parent.border_id;
    }
    if xf.apply_number_format == Some(false) {
        cf.num_fmt_id = parent.num_fmt_id;
    }
    if xf.apply_alignment == Some(false) {
        cf.horizontal_alignment = parent.horizontal_alignment.clone();
        cf.vertical_alignment = parent.vertical_alignment.clone();
        cf.wrap_text = parent.wrap_text;
        cf.indent = parent.indent;
        cf.text_rotation = parent.text_rotation;
    }
    cf
}

fn extract_xf(xf: &XCellFormat) -> CellFormat {
    let mut cf = CellFormat {
        font_id: xf.font_id,
        fill_id: xf.fill_id,
        border_id: xf.border_id,
        num_fmt_id: xf.number_format_id,
        ..Default::default()
    };
    if let Some(align) = &xf.alignment {
        if let Some(h) = &align.horizontal {
            let dbg = format!("{h:?}").to_ascii_lowercase();
            cf.horizontal_alignment = Some(
                if dbg.contains("center") {
                    "center"
                } else if dbg.contains("right") {
                    "right"
                } else if dbg.contains("justify") {
                    "justify"
                } else if dbg.contains("fill") {
                    "fill"
                } else {
                    "left"
                }
                .to_string(),
            );
        }
        if let Some(v) = &align.vertical {
            let dbg = format!("{v:?}").to_ascii_lowercase();
            cf.vertical_alignment = Some(
                if dbg.contains("center") {
                    "center"
                } else if dbg.contains("top") {
                    "top"
                } else {
                    "bottom"
                }
                .to_string(),
            );
        }
        cf.wrap_text = align.wrap_text.unwrap_or(false);
        cf.indent = align.indent;
        cf.text_rotation = align.text_rotation.map(|v| v as i32);
    }
    cf
}

fn extract_color_x(c: &x::Color) -> Option<Color> {
    let any = c.rgb.is_some() || c.theme.is_some() || c.indexed.is_some();
    if !any {
        return None;
    }
    Some(Color {
        rgb: c.rgb.as_ref().map(|s| s.as_str().to_string()),
        theme: c.theme,
        indexed: c.indexed,
        tint: c.tint,
    })
}
