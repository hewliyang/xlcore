use ooxmlsdk::parts::workbook_styles_part::WorkbookStylesPart;
use ooxmlsdk::simple_type::BooleanValue;
use xlcore_io::spreadsheetml as x;
pub use xlcore_types::{
    AlignmentPatch, BorderLinePatch, BorderLineStyle, BorderPatch, FillPatch, FontPatch,
    FontScheme, GradientFillPatch, GradientType, HorizontalAlign, NamedStyleInfo, NamedStylePatch,
    PatternType, ProtectionPatch, ReadingOrder, StylePatch, UnderlinePatch, VertAlign,
    VerticalAlign,
};

use crate::errors::sdk_err_to_api;
use crate::ooxml_header;
use crate::{ApiError, ApiErrorCode, Result, Workbook};

pub(crate) fn ensure_styles_part(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> Result<WorkbookStylesPart> {
    let wb_part = doc.workbook_part().map_err(sdk_err_to_api)?.clone();
    if let Some(part) = wb_part.workbook_styles_part(doc) {
        return Ok(part.clone());
    }
    let part: WorkbookStylesPart = wb_part.add_new_part_auto_id(doc).map_err(sdk_err_to_api)?;
    part.set_root_element(doc, default_stylesheet())
        .map_err(sdk_err_to_api)?;
    Ok(part)
}

fn default_stylesheet() -> x::Stylesheet {
    x::Stylesheet {
        xmlns: ooxml_header::spreadsheetml_default_only(),
        xml_header: ooxml_header::STANDALONE,
        fonts: Some(x::Fonts {
            count: Some(1),
            font: vec![default_font()],
            ..Default::default()
        }),
        fills: Some(x::Fills {
            count: Some(2),
            fill: vec![pattern_fill_none(), pattern_fill_gray125()],
        }),
        borders: Some(x::Borders {
            count: Some(1),
            border: vec![x::Border::default()],
        }),
        cell_style_formats: Some(x::CellStyleFormats {
            count: Some(1),
            cell_format: vec![x::CellFormat {
                number_format_id: Some(0),
                font_id: Some(0),
                fill_id: Some(0),
                border_id: Some(0),
                ..Default::default()
            }],
        }),
        cell_formats: Some(x::CellFormats {
            count: Some(1),
            cell_format: vec![x::CellFormat {
                number_format_id: Some(0),
                font_id: Some(0),
                fill_id: Some(0),
                border_id: Some(0),
                format_id: Some(0),
                ..Default::default()
            }],
        }),
        cell_styles: Some(x::CellStyles {
            count: Some(1),
            cell_style: vec![x::CellStyle {
                name: Some("Normal".to_string()),
                format_id: 0,
                builtin_id: Some(0),
                ..Default::default()
            }],
        }),
        ..Default::default()
    }
}

fn default_font() -> x::Font {
    x::Font {
        font_choice: vec![
            x::FontChoice::FontSize(Box::new(x::FontSize {
                val: 11.0,
                ..Default::default()
            })),
            x::FontChoice::Color(Box::new(x::Color {
                theme: Some(1),
                ..Default::default()
            })),
            x::FontChoice::FontName(Box::new(x::FontName {
                val: "Calibri".to_string(),
                ..Default::default()
            })),
            x::FontChoice::FontFamilyNumbering(Box::new(x::FontFamilyNumbering {
                val: 2,
                ..Default::default()
            })),
            x::FontChoice::FontScheme(Box::new(x::FontScheme {
                val: x::FontSchemeValues::Minor,
                ..Default::default()
            })),
        ],
        ..Default::default()
    }
}

fn pattern_fill_none() -> x::Fill {
    x::Fill {
        fill_choice: Some(x::FillChoice::PatternFill(Box::new(x::PatternFill {
            pattern_type: Some(x::PatternValues::None),
            ..Default::default()
        }))),
        ..Default::default()
    }
}

fn pattern_fill_gray125() -> x::Fill {
    x::Fill {
        fill_choice: Some(x::FillChoice::PatternFill(Box::new(x::PatternFill {
            pattern_type: Some(x::PatternValues::Gray125),
            ..Default::default()
        }))),
        ..Default::default()
    }
}

pub(crate) fn resolve_style_index(
    doc: &mut xlcore_io::SpreadsheetDocument,
    existing_index: Option<u32>,
    patch: &StylePatch,
) -> Result<u32> {
    let part = ensure_styles_part(doc)?;
    let sheet = part.root_element_mut(doc).map_err(sdk_err_to_api)?;
    ensure_default_collections(sheet);
    let base = existing_index
        .map(|i| i as usize)
        .filter(|i| {
            *i < sheet
                .cell_formats
                .as_ref()
                .map(|cf| cf.cell_format.len())
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let base_xf = sheet
        .cell_formats
        .as_ref()
        .and_then(|cf| cf.cell_format.get(base).cloned())
        .unwrap_or_default();

    let mut new_xf = if let Some(name) = patch.named_style.as_deref() {
        let master_id = find_named_style_master(sheet, name).ok_or_else(|| {
            ApiError::new(
                ApiErrorCode::UnsupportedStyle,
                format!("named style not found: {name}"),
            )
        })?;
        let mut xf = sheet
            .cell_style_formats
            .as_ref()
            .and_then(|cf| cf.cell_format.get(master_id as usize).cloned())
            .unwrap_or_default();
        xf.format_id = Some(master_id);
        xf.apply_number_format = Some(BooleanValue::from_bool(true));
        xf.apply_font = Some(BooleanValue::from_bool(true));
        xf.apply_fill = Some(BooleanValue::from_bool(true));
        xf.apply_border = Some(BooleanValue::from_bool(true));
        if xf.alignment.is_some() {
            xf.apply_alignment = Some(BooleanValue::from_bool(true));
        }
        if xf.protection.is_some() {
            xf.apply_protection = Some(BooleanValue::from_bool(true));
        }
        xf
    } else {
        let mut xf = base_xf;
        if xf.format_id.is_none() {
            xf.format_id = Some(0);
        }
        xf
    };

    apply_patch_to_xf(sheet, &mut new_xf, patch)?;

    Ok(intern_cell_format(sheet, new_xf))
}

fn apply_patch_to_xf(
    sheet: &mut x::Stylesheet,
    xf: &mut x::CellFormat,
    patch: &StylePatch,
) -> Result<()> {
    if let Some(font_patch) = patch.font.as_ref() {
        let current = xf.font_id.unwrap_or(0) as usize;
        let new_font = build_font(sheet, current, font_patch)?;
        xf.font_id = Some(intern_font(sheet, new_font));
        xf.apply_font = Some(BooleanValue::from_bool(true));
    }
    if let Some(fill_patch) = patch.fill.as_ref() {
        let new_fill = build_fill(fill_patch)?;
        xf.fill_id = Some(intern_fill(sheet, new_fill));
        xf.apply_fill = Some(BooleanValue::from_bool(true));
    }
    if let Some(border_patch) = patch.border.as_ref() {
        let current = xf.border_id.unwrap_or(0) as usize;
        let new_border = build_border(sheet, current, border_patch)?;
        xf.border_id = Some(intern_border(sheet, new_border));
        xf.apply_border = Some(BooleanValue::from_bool(true));
    }
    if let Some(num_fmt) = patch.number_format.as_deref() {
        let new_num_fmt_id = intern_num_fmt(sheet, num_fmt);
        xf.number_format_id = Some(new_num_fmt_id);
        xf.apply_number_format = Some(BooleanValue::from_bool(true));
    }
    if let Some(align_patch) = patch.alignment.as_ref() {
        apply_alignment(xf, align_patch);
        xf.apply_alignment = Some(BooleanValue::from_bool(true));
    }
    if let Some(prot_patch) = patch.protection.as_ref() {
        apply_protection(&mut xf.protection, prot_patch);
        xf.apply_protection = Some(BooleanValue::from_bool(true));
    }
    Ok(())
}

fn find_named_style_master(sheet: &x::Stylesheet, name: &str) -> Option<u32> {
    sheet.cell_styles.as_ref().and_then(|styles| {
        styles
            .cell_style
            .iter()
            .find(|cs| cs.name.as_deref() == Some(name))
            .map(|cs| cs.format_id)
    })
}

pub(crate) fn resolve_num_fmt_id(
    doc: &mut xlcore_io::SpreadsheetDocument,
    code: &str,
) -> Result<u32> {
    let part = ensure_styles_part(doc)?;
    let sheet = part.root_element_mut(doc).map_err(sdk_err_to_api)?;
    Ok(intern_num_fmt(sheet, code))
}

pub(crate) fn upsert_dxf(
    doc: &mut xlcore_io::SpreadsheetDocument,
    patch: &StylePatch,
) -> Result<u32> {
    let part = ensure_styles_part(doc)?;
    let sheet = part.root_element_mut(doc).map_err(sdk_err_to_api)?;
    let mut dxf = x::DifferentialFormat::default();
    if let Some(font_patch) = patch.font.as_ref() {
        dxf.font = Some(build_font(sheet, usize::MAX, font_patch)?);
    }
    if let Some(fill_patch) = patch.fill.as_ref() {
        dxf.fill = Some(Box::new(build_fill(fill_patch)?));
    }
    if let Some(border_patch) = patch.border.as_ref() {
        dxf.border = Some(Box::new(build_border(sheet, usize::MAX, border_patch)?));
    }
    if let Some(num_fmt) = patch.number_format.as_deref() {
        let id = builtin_num_fmt_id(num_fmt).unwrap_or(0);
        dxf.numbering_format = Some(x::NumberingFormat {
            number_format_id: id,
            format_code: num_fmt.to_string(),
            ..Default::default()
        });
    }
    if let Some(align_patch) = patch.alignment.as_ref() {
        let mut tmp = x::CellFormat::default();
        apply_alignment(&mut tmp, align_patch);
        dxf.alignment = tmp.alignment;
    }
    if let Some(prot_patch) = patch.protection.as_ref() {
        apply_protection(&mut dxf.protection, prot_patch);
    }
    let dxfs = sheet
        .differential_formats
        .get_or_insert_with(x::DifferentialFormats::default);
    if let Some(idx) = dxfs.differential_format.iter().position(|d| d == &dxf) {
        return Ok(idx as u32);
    }
    dxfs.differential_format.push(dxf);
    let idx = dxfs.differential_format.len() - 1;
    dxfs.count = Some(dxfs.differential_format.len() as u32);
    Ok(idx as u32)
}

fn ensure_default_collections(sheet: &mut x::Stylesheet) {
    if sheet.fonts.is_none() {
        sheet.fonts = Some(x::Fonts {
            count: Some(1),
            font: vec![default_font()],
            ..Default::default()
        });
    }
    if sheet.fills.is_none() {
        sheet.fills = Some(x::Fills {
            count: Some(2),
            fill: vec![pattern_fill_none(), pattern_fill_gray125()],
        });
    } else {
        let fills = sheet.fills.as_mut().unwrap();
        if fills.fill.is_empty() {
            fills.fill.push(pattern_fill_none());
            fills.fill.push(pattern_fill_gray125());
        }
    }
    if sheet.borders.is_none() {
        sheet.borders = Some(x::Borders {
            count: Some(1),
            border: vec![x::Border::default()],
        });
    }
    if sheet.cell_formats.is_none() {
        sheet.cell_formats = Some(x::CellFormats {
            count: Some(1),
            cell_format: vec![x::CellFormat {
                number_format_id: Some(0),
                font_id: Some(0),
                fill_id: Some(0),
                border_id: Some(0),
                format_id: Some(0),
                ..Default::default()
            }],
        });
    }
    if sheet.cell_style_formats.is_none() {
        sheet.cell_style_formats = Some(x::CellStyleFormats {
            count: Some(1),
            cell_format: vec![x::CellFormat {
                number_format_id: Some(0),
                font_id: Some(0),
                fill_id: Some(0),
                border_id: Some(0),
                ..Default::default()
            }],
        });
    }
    if sheet.cell_styles.is_none() {
        sheet.cell_styles = Some(x::CellStyles {
            count: Some(1),
            cell_style: vec![x::CellStyle {
                name: Some("Normal".to_string()),
                format_id: 0,
                builtin_id: Some(0),
                ..Default::default()
            }],
        });
    }
}

fn build_font(sheet: &x::Stylesheet, current: usize, patch: &FontPatch) -> Result<x::Font> {
    let base = sheet
        .fonts
        .as_ref()
        .and_then(|f| f.font.get(current).cloned())
        .unwrap_or_else(default_font);
    let mut font = base;
    if let Some(name) = patch.name.as_deref() {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::FontName(_) | x::FontChoice::FontScheme(_)));
        font.font_choice
            .push(x::FontChoice::FontName(Box::new(x::FontName {
                val: name.to_string(),
                ..Default::default()
            })));
    }
    if let Some(size) = patch.size {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::FontSize(_)));
        font.font_choice
            .push(x::FontChoice::FontSize(Box::new(x::FontSize {
                val: size,
                ..Default::default()
            })));
    }
    if let Some(bold) = patch.bold {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::Bold(_)));
        if bold {
            font.font_choice.push(x::FontChoice::Bold(Box::new(x::Bold {
                val: None,
                ..Default::default()
            })));
        }
    }
    if let Some(italic) = patch.italic {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::Italic(_)));
        if italic {
            font.font_choice
                .push(x::FontChoice::Italic(Box::new(x::Italic {
                    val: None,
                    ..Default::default()
                })));
        }
    }
    if let Some(strike) = patch.strike {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::Strike(_)));
        if strike {
            font.font_choice
                .push(x::FontChoice::Strike(Box::new(x::Strike {
                    val: None,
                    ..Default::default()
                })));
        }
    }
    if let Some(underline) = patch.underline {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::Underline(_)));
        match underline {
            UnderlinePatch::None => {}
            UnderlinePatch::Single => {
                font.font_choice
                    .push(x::FontChoice::Underline(Box::new(x::Underline {
                        val: None,
                        ..Default::default()
                    })))
            }
            UnderlinePatch::Double => {
                font.font_choice
                    .push(x::FontChoice::Underline(Box::new(x::Underline {
                        val: Some(x::UnderlineValues::Double),
                        ..Default::default()
                    })))
            }
        }
    }
    if let Some(color) = patch.color.as_deref() {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::Color(_)));
        font.font_choice
            .push(x::FontChoice::Color(Box::new(parse_color(color)?)));
    }
    if let Some(vert) = patch.vert_align {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::VerticalTextAlignment(_)));
        if !matches!(vert, VertAlign::Baseline) {
            font.font_choice
                .push(x::FontChoice::VerticalTextAlignment(Box::new(
                    x::VerticalTextAlignment {
                        val: match vert {
                            VertAlign::Baseline => x::VerticalAlignmentRunValues::Baseline,
                            VertAlign::Superscript => x::VerticalAlignmentRunValues::Superscript,
                            VertAlign::Subscript => x::VerticalAlignmentRunValues::Subscript,
                        },
                    },
                )));
        }
    }
    if let Some(family) = patch.family {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::FontFamilyNumbering(_)));
        font.font_choice
            .push(x::FontChoice::FontFamilyNumbering(Box::new(
                x::FontFamilyNumbering {
                    val: family as i32,
                    ..Default::default()
                },
            )));
    }
    if let Some(scheme) = patch.scheme {
        font.font_choice
            .retain(|c| !matches!(c, x::FontChoice::FontScheme(_)));
        font.font_choice
            .push(x::FontChoice::FontScheme(Box::new(x::FontScheme {
                val: match scheme {
                    FontScheme::None => x::FontSchemeValues::None,
                    FontScheme::Major => x::FontSchemeValues::Major,
                    FontScheme::Minor => x::FontSchemeValues::Minor,
                },
            })));
    }
    Ok(font)
}

fn pattern_type_to_x(p: PatternType) -> x::PatternValues {
    match p {
        PatternType::None => x::PatternValues::None,
        PatternType::Solid => x::PatternValues::Solid,
        PatternType::MediumGray => x::PatternValues::MediumGray,
        PatternType::DarkGray => x::PatternValues::DarkGray,
        PatternType::LightGray => x::PatternValues::LightGray,
        PatternType::DarkHorizontal => x::PatternValues::DarkHorizontal,
        PatternType::DarkVertical => x::PatternValues::DarkVertical,
        PatternType::DarkDown => x::PatternValues::DarkDown,
        PatternType::DarkUp => x::PatternValues::DarkUp,
        PatternType::DarkGrid => x::PatternValues::DarkGrid,
        PatternType::DarkTrellis => x::PatternValues::DarkTrellis,
        PatternType::LightHorizontal => x::PatternValues::LightHorizontal,
        PatternType::LightVertical => x::PatternValues::LightVertical,
        PatternType::LightDown => x::PatternValues::LightDown,
        PatternType::LightUp => x::PatternValues::LightUp,
        PatternType::LightGrid => x::PatternValues::LightGrid,
        PatternType::LightTrellis => x::PatternValues::LightTrellis,
        PatternType::Gray125 => x::PatternValues::Gray125,
        PatternType::Gray0625 => x::PatternValues::Gray0625,
    }
}

fn build_gradient_fill(patch: &GradientFillPatch) -> Result<x::Fill> {
    let path = matches!(patch.kind, Some(GradientType::Path));
    let mut stops = Vec::with_capacity(patch.stops.len());
    for stop in &patch.stops {
        if !(0.0..=1.0).contains(&stop.position) {
            return Err(ApiError::new(
                ApiErrorCode::UnsupportedStyle,
                format!(
                    "gradient stop position out of range 0..=1: {}",
                    stop.position
                ),
            ));
        }
        stops.push(x::GradientStop {
            position: stop.position,
            color: Box::new(parse_color(&stop.color)?),
        });
    }
    Ok(x::Fill {
        fill_choice: Some(x::FillChoice::GradientFill(Box::new(x::GradientFill {
            r#type: path.then_some(x::GradientValues::Path),
            degree: if path { None } else { patch.degree },
            left: if path { patch.left } else { None },
            right: if path { patch.right } else { None },
            top: if path { patch.top } else { None },
            bottom: if path { patch.bottom } else { None },
            gradient_stop: stops,
        }))),
        ..Default::default()
    })
}

fn build_fill(patch: &FillPatch) -> Result<x::Fill> {
    if let Some(gradient) = patch.gradient.as_ref() {
        return build_gradient_fill(gradient);
    }
    let pattern = match patch.pattern {
        Some(p) => pattern_type_to_x(p),
        None => {
            if patch.color.is_some() || patch.foreground.is_some() || patch.background.is_some() {
                x::PatternValues::Solid
            } else {
                return Ok(pattern_fill_none());
            }
        }
    };
    if matches!(pattern, x::PatternValues::None) {
        return Ok(pattern_fill_none());
    }
    let solid = matches!(pattern, x::PatternValues::Solid);
    let fg_src = patch.foreground.as_deref().or(patch.color.as_deref());
    let foreground_color = match fg_src {
        Some(c) => {
            let parsed = parse_color(c)?;
            Some(x::ForegroundColor {
                rgb: parsed.rgb,
                theme: parsed.theme,
                tint: parsed.tint,
                indexed: parsed.indexed,
                auto: parsed.auto,
            })
        }
        None => None,
    };
    let background_color = match patch.background.as_deref() {
        Some(c) => {
            let parsed = parse_color(c)?;
            Some(x::BackgroundColor {
                rgb: parsed.rgb,
                theme: parsed.theme,
                tint: parsed.tint,
                indexed: parsed.indexed,
                auto: parsed.auto,
            })
        }
        None if solid => Some(x::BackgroundColor {
            indexed: Some(64),
            ..Default::default()
        }),
        None => None,
    };
    Ok(x::Fill {
        fill_choice: Some(x::FillChoice::PatternFill(Box::new(x::PatternFill {
            pattern_type: Some(pattern),
            foreground_color,
            background_color,
        }))),
        ..Default::default()
    })
}

fn build_border(sheet: &x::Stylesheet, current: usize, patch: &BorderPatch) -> Result<x::Border> {
    let mut border = sheet
        .borders
        .as_ref()
        .and_then(|b| b.border.get(current).cloned())
        .unwrap_or_default();
    let resolved = |side: &Option<BorderLinePatch>| -> Option<BorderLinePatch> {
        side.clone().or_else(|| patch.all.clone())
    };
    apply_side(&mut border.left_border, resolved(&patch.left).as_ref())?;
    apply_side(&mut border.right_border, resolved(&patch.right).as_ref())?;
    apply_side(&mut border.top_border, resolved(&patch.top).as_ref())?;
    apply_side(&mut border.bottom_border, resolved(&patch.bottom).as_ref())?;
    apply_side(&mut border.diagonal_border, patch.diagonal.as_ref())?;
    if let Some(up) = patch.diagonal_up {
        border.diagonal_up = Some(BooleanValue::from_bool(up));
    }
    if let Some(down) = patch.diagonal_down {
        border.diagonal_down = Some(BooleanValue::from_bool(down));
    }
    Ok(border)
}

fn apply_side<T>(slot: &mut Option<Box<T>>, patch: Option<&BorderLinePatch>) -> Result<()>
where
    T: BorderSide + Default,
{
    let Some(patch) = patch else { return Ok(()) };
    if matches!(patch.style, BorderLineStyle::None) {
        *slot = None;
        return Ok(());
    }
    let mut side = T::default();
    side.set_style(border_style_to_x(patch.style));
    if let Some(color) = patch.color.as_deref() {
        side.set_color(parse_color(color)?);
    }
    *slot = Some(Box::new(side));
    Ok(())
}

trait BorderSide {
    fn set_style(&mut self, style: x::BorderStyleValues);
    fn set_color(&mut self, color: x::Color);
}

macro_rules! impl_border_side {
    ($ty:ty) => {
        impl BorderSide for $ty {
            fn set_style(&mut self, style: x::BorderStyleValues) {
                self.style = Some(style);
            }
            fn set_color(&mut self, color: x::Color) {
                self.color = Some(color);
            }
        }
    };
}
impl_border_side!(x::LeftBorder);
impl_border_side!(x::RightBorder);
impl_border_side!(x::TopBorder);
impl_border_side!(x::BottomBorder);
impl_border_side!(x::DiagonalBorder);

fn border_style_to_x(style: BorderLineStyle) -> x::BorderStyleValues {
    match style {
        BorderLineStyle::None => x::BorderStyleValues::None,
        BorderLineStyle::Thin => x::BorderStyleValues::Thin,
        BorderLineStyle::Medium => x::BorderStyleValues::Medium,
        BorderLineStyle::Thick => x::BorderStyleValues::Thick,
        BorderLineStyle::Dashed => x::BorderStyleValues::Dashed,
        BorderLineStyle::Dotted => x::BorderStyleValues::Dotted,
        BorderLineStyle::Double => x::BorderStyleValues::Double,
        BorderLineStyle::Hair => x::BorderStyleValues::Hair,
    }
}

fn apply_alignment(xf: &mut x::CellFormat, patch: &AlignmentPatch) {
    let align = xf.alignment.get_or_insert_with(Default::default);
    if let Some(h) = patch.horizontal {
        align.horizontal = Some(match h {
            HorizontalAlign::General => x::HorizontalAlignmentValues::General,
            HorizontalAlign::Left => x::HorizontalAlignmentValues::Left,
            HorizontalAlign::Center => x::HorizontalAlignmentValues::Center,
            HorizontalAlign::Right => x::HorizontalAlignmentValues::Right,
            HorizontalAlign::Fill => x::HorizontalAlignmentValues::Fill,
            HorizontalAlign::Justify => x::HorizontalAlignmentValues::Justify,
            HorizontalAlign::CenterContinuous => x::HorizontalAlignmentValues::CenterContinuous,
            HorizontalAlign::Distributed => x::HorizontalAlignmentValues::Distributed,
        });
    }
    if let Some(v) = patch.vertical {
        align.vertical = Some(match v {
            VerticalAlign::Top => x::VerticalAlignmentValues::Top,
            VerticalAlign::Center => x::VerticalAlignmentValues::Center,
            VerticalAlign::Bottom => x::VerticalAlignmentValues::Bottom,
            VerticalAlign::Justify => x::VerticalAlignmentValues::Justify,
            VerticalAlign::Distributed => x::VerticalAlignmentValues::Distributed,
        });
    }
    if let Some(wrap) = patch.wrap {
        align.wrap_text = Some(BooleanValue::from_bool(wrap));
    }
    if let Some(indent) = patch.indent {
        align.indent = Some(indent);
    }
    if let Some(rot) = patch.text_rotation {
        let normalized = if rot < 0 {
            (90 - rot) as u32
        } else {
            rot as u32
        };
        align.text_rotation = Some(normalized);
    }
    if let Some(shrink) = patch.shrink_to_fit {
        align.shrink_to_fit = Some(BooleanValue::from_bool(shrink));
    }
    if let Some(justify) = patch.justify_last_line {
        align.justify_last_line = Some(BooleanValue::from_bool(justify));
    }
    if let Some(order) = patch.reading_order {
        align.reading_order = Some(match order {
            ReadingOrder::Context => 0,
            ReadingOrder::LeftToRight => 1,
            ReadingOrder::RightToLeft => 2,
        });
    }
}

fn apply_protection(slot: &mut Option<x::Protection>, patch: &ProtectionPatch) {
    let prot = slot.get_or_insert_with(Default::default);
    if let Some(locked) = patch.locked {
        prot.locked = Some(BooleanValue::from_bool(locked));
    }
    if let Some(hidden) = patch.hidden {
        prot.hidden = Some(BooleanValue::from_bool(hidden));
    }
}

fn intern_font(sheet: &mut x::Stylesheet, font: x::Font) -> u32 {
    let fonts = sheet.fonts.as_mut().expect("fonts ensured");
    if let Some(idx) = fonts.font.iter().position(|f| fonts_equal(f, &font)) {
        return idx as u32;
    }
    fonts.font.push(font);
    let idx = fonts.font.len() - 1;
    fonts.count = Some(fonts.font.len() as u32);
    idx as u32
}

fn intern_fill(sheet: &mut x::Stylesheet, fill: x::Fill) -> u32 {
    let fills = sheet.fills.as_mut().expect("fills ensured");
    if let Some(idx) = fills.fill.iter().position(|f| fills_equal(f, &fill)) {
        return idx as u32;
    }
    fills.fill.push(fill);
    let idx = fills.fill.len() - 1;
    fills.count = Some(fills.fill.len() as u32);
    idx as u32
}

fn intern_border(sheet: &mut x::Stylesheet, border: x::Border) -> u32 {
    let borders = sheet.borders.as_mut().expect("borders ensured");
    if let Some(idx) = borders
        .border
        .iter()
        .position(|b| borders_equal(b, &border))
    {
        return idx as u32;
    }
    borders.border.push(border);
    let idx = borders.border.len() - 1;
    borders.count = Some(borders.border.len() as u32);
    idx as u32
}

fn intern_cell_format(sheet: &mut x::Stylesheet, xf: x::CellFormat) -> u32 {
    let cfs = sheet.cell_formats.as_mut().expect("cellXfs ensured");
    if let Some(idx) = cfs.cell_format.iter().position(|x| xfs_equal(x, &xf)) {
        return idx as u32;
    }
    cfs.cell_format.push(xf);
    let idx = cfs.cell_format.len() - 1;
    cfs.count = Some(cfs.cell_format.len() as u32);
    idx as u32
}

fn intern_num_fmt(sheet: &mut x::Stylesheet, code: &str) -> u32 {
    if let Some(id) = builtin_num_fmt_id(code) {
        return id;
    }
    let formats = sheet.numbering_formats.get_or_insert_with(Default::default);
    if let Some(existing) = formats
        .numbering_format
        .iter()
        .find(|nf| nf.format_code.as_str() == code)
    {
        return existing.number_format_id;
    }
    let next_id = formats
        .numbering_format
        .iter()
        .map(|nf| nf.number_format_id)
        .max()
        .map(|id| id.max(163))
        .unwrap_or(163)
        + 1;
    formats.numbering_format.push(x::NumberingFormat {
        number_format_id: next_id,
        format_code: code.to_string(),
        ..Default::default()
    });
    formats.count = Some(formats.numbering_format.len() as u32);
    next_id
}

fn builtin_num_fmt_id(code: &str) -> Option<u32> {
    match code {
        "General" => Some(0),
        "0" => Some(1),
        "0.00" => Some(2),
        "#,##0" => Some(3),
        "#,##0.00" => Some(4),
        "0%" => Some(9),
        "0.00%" => Some(10),
        "0.00E+00" => Some(11),
        "# ?/?" => Some(12),
        "# ??/??" => Some(13),
        "mm-dd-yy" => Some(14),
        "d-mmm-yy" => Some(15),
        "d-mmm" => Some(16),
        "mmm-yy" => Some(17),
        "h:mm AM/PM" => Some(18),
        "h:mm:ss AM/PM" => Some(19),
        "h:mm" => Some(20),
        "h:mm:ss" => Some(21),
        "m/d/yy h:mm" => Some(22),
        "#,##0 ;(#,##0)" => Some(37),
        "#,##0 ;[Red](#,##0)" => Some(38),
        "#,##0.00;(#,##0.00)" => Some(39),
        "#,##0.00;[Red](#,##0.00)" => Some(40),
        "mm:ss" => Some(45),
        "[h]:mm:ss" => Some(46),
        "mmss.0" => Some(47),
        "##0.0E+0" => Some(48),
        "@" => Some(49),
        _ => None,
    }
}

fn builtin_num_fmt_code(id: u32) -> Option<&'static str> {
    Some(match id {
        0 => "General",
        1 => "0",
        2 => "0.00",
        3 => "#,##0",
        4 => "#,##0.00",
        9 => "0%",
        10 => "0.00%",
        11 => "0.00E+00",
        12 => "# ?/?",
        13 => "# ??/??",
        14 => "mm-dd-yy",
        15 => "d-mmm-yy",
        16 => "d-mmm",
        17 => "mmm-yy",
        18 => "h:mm AM/PM",
        19 => "h:mm:ss AM/PM",
        20 => "h:mm",
        21 => "h:mm:ss",
        22 => "m/d/yy h:mm",
        37 => "#,##0 ;(#,##0)",
        38 => "#,##0 ;[Red](#,##0)",
        39 => "#,##0.00;(#,##0.00)",
        40 => "#,##0.00;[Red](#,##0.00)",
        45 => "mm:ss",
        46 => "[h]:mm:ss",
        47 => "mmss.0",
        48 => "##0.0E+0",
        49 => "@",
        _ => return None,
    })
}

pub(crate) fn num_fmt_code(doc: &mut xlcore_io::SpreadsheetDocument, id: u32) -> Option<String> {
    if let Some(code) = builtin_num_fmt_code(id) {
        return Some(code.to_string());
    }
    let wb_part = doc.workbook_part().ok()?.clone();
    let part = wb_part.workbook_styles_part(doc)?;
    let sheet = part.root_element(doc).ok()?;
    sheet.numbering_formats.as_ref().and_then(|fmts| {
        fmts.numbering_format
            .iter()
            .find(|nf| nf.number_format_id == id)
            .map(|nf| nf.format_code.to_string())
    })
}

pub(crate) fn parse_color(text: &str) -> Result<x::Color> {
    let cleaned = text.trim().trim_start_matches('#').to_ascii_uppercase();
    let hex = match cleaned.len() {
        6 => format!("FF{cleaned}"),
        8 => cleaned,
        _ => {
            return Err(ApiError::new(
                ApiErrorCode::UnsupportedStyle,
                format!("invalid color: {text}"),
            ));
        }
    };
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::new(
            ApiErrorCode::UnsupportedStyle,
            format!("invalid color: {text}"),
        ));
    }
    Ok(x::Color {
        rgb: Some(hex),
        ..Default::default()
    })
}

fn fonts_equal(a: &x::Font, b: &x::Font) -> bool {
    font_signature(a) == font_signature(b)
}

fn font_signature(f: &x::Font) -> String {
    let mut name = "";
    let mut size = 0.0;
    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    let mut underline = String::new();
    let mut color = String::new();
    let mut scheme = String::new();
    let mut vert = String::new();
    let mut family = String::new();
    for c in &f.font_choice {
        match c {
            x::FontChoice::FontName(n) => name = n.val.as_str(),
            x::FontChoice::FontSize(s) => size = s.val,
            x::FontChoice::Bold(_) => bold = true,
            x::FontChoice::Italic(_) => italic = true,
            x::FontChoice::Strike(_) => strike = true,
            x::FontChoice::Underline(u) => underline = format!("{:?}", u.val),
            x::FontChoice::Color(c) => color = color_sig(c),
            x::FontChoice::FontScheme(s) => scheme = format!("{:?}", s.val),
            x::FontChoice::VerticalTextAlignment(v) => vert = format!("{:?}", v.val),
            x::FontChoice::FontFamilyNumbering(fm) => family = fm.val.to_string(),
            _ => {}
        }
    }
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        name, size, bold, italic, strike, underline, color, scheme, vert, family
    )
}

fn fills_equal(a: &x::Fill, b: &x::Fill) -> bool {
    fill_signature(a) == fill_signature(b)
}

fn fill_signature(f: &x::Fill) -> String {
    match &f.fill_choice {
        Some(x::FillChoice::PatternFill(pf)) => format!(
            "pat|{:?}|{}|{}",
            pf.pattern_type,
            pf.foreground_color.as_ref().map(fg_sig).unwrap_or_default(),
            pf.background_color.as_ref().map(bg_sig).unwrap_or_default(),
        ),
        Some(x::FillChoice::GradientFill(gf)) => {
            let stops: String = gf
                .gradient_stop
                .iter()
                .map(|s| format!("{}:{}", s.position, color_sig(&s.color)))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "grad|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{}",
                gf.r#type, gf.degree, gf.left, gf.right, gf.top, gf.bottom, stops
            )
        }
        None => "none".to_string(),
    }
}

fn color_sig(c: &x::Color) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{:?}",
        c.rgb.as_deref().unwrap_or(""),
        c.theme,
        c.tint,
        c.indexed,
        c.auto
    )
}
fn fg_sig(c: &x::ForegroundColor) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}",
        c.rgb.as_deref().unwrap_or(""),
        c.theme,
        c.tint,
        c.indexed
    )
}
fn bg_sig(c: &x::BackgroundColor) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}",
        c.rgb.as_deref().unwrap_or(""),
        c.theme,
        c.tint,
        c.indexed
    )
}

fn borders_equal(a: &x::Border, b: &x::Border) -> bool {
    border_signature(a) == border_signature(b)
}

fn border_signature(b: &x::Border) -> String {
    fn side<T>(s: &Option<Box<T>>) -> String
    where
        T: BorderSideRead,
    {
        match s {
            Some(side) => format!(
                "{:?}|{}",
                side.style(),
                side.color().map(color_sig).unwrap_or_default()
            ),
            None => "none".to_string(),
        }
    }
    format!(
        "{}|{}|{}|{}|{}|{:?}|{:?}",
        side(&b.left_border),
        side(&b.right_border),
        side(&b.top_border),
        side(&b.bottom_border),
        side(&b.diagonal_border),
        b.diagonal_up.clone().map(bool::from),
        b.diagonal_down.clone().map(bool::from),
    )
}

trait BorderSideRead {
    fn style(&self) -> Option<x::BorderStyleValues>;
    fn color(&self) -> Option<&x::Color>;
}
macro_rules! impl_border_side_read {
    ($ty:ty) => {
        impl BorderSideRead for $ty {
            fn style(&self) -> Option<x::BorderStyleValues> {
                self.style
            }
            fn color(&self) -> Option<&x::Color> {
                self.color.as_ref()
            }
        }
    };
}
impl_border_side_read!(x::LeftBorder);
impl_border_side_read!(x::RightBorder);
impl_border_side_read!(x::TopBorder);
impl_border_side_read!(x::BottomBorder);
impl_border_side_read!(x::DiagonalBorder);

fn xfs_equal(a: &x::CellFormat, b: &x::CellFormat) -> bool {
    a.number_format_id == b.number_format_id
        && a.font_id == b.font_id
        && a.fill_id == b.fill_id
        && a.border_id == b.border_id
        && a.format_id == b.format_id
        && a.apply_font == b.apply_font
        && a.apply_fill == b.apply_fill
        && a.apply_border == b.apply_border
        && a.apply_number_format == b.apply_number_format
        && a.apply_alignment == b.apply_alignment
        && a.apply_protection == b.apply_protection
        && alignment_equal(&a.alignment, &b.alignment)
        && a.protection == b.protection
}

fn alignment_equal(a: &Option<x::Alignment>, b: &Option<x::Alignment>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.horizontal == b.horizontal
                && a.vertical == b.vertical
                && a.wrap_text == b.wrap_text
                && a.indent == b.indent
                && a.text_rotation == b.text_rotation
                && a.shrink_to_fit == b.shrink_to_fit
                && a.justify_last_line == b.justify_last_line
                && a.reading_order == b.reading_order
        }
        _ => false,
    }
}

impl Workbook {
    pub fn named_styles(&mut self) -> Result<Vec<NamedStyleInfo>> {
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let Some(part) = wb_part.workbook_styles_part(&mut self.doc) else {
            return Ok(Vec::new());
        };
        let sheet = part.root_element(&mut self.doc).map_err(sdk_err_to_api)?;
        Ok(sheet
            .cell_styles
            .as_ref()
            .map(|styles| {
                styles
                    .cell_style
                    .iter()
                    .filter_map(named_style_info)
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn set_named_style(&mut self, patch: NamedStylePatch) -> Result<NamedStyleInfo> {
        if patch.name.trim().is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::UnsupportedStyle,
                "named style name is empty",
            ));
        }
        let part = ensure_styles_part(&mut self.doc)?;
        let sheet = part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        ensure_default_collections(sheet);

        let existing_master = find_named_style_master(sheet, &patch.name).filter(|id| *id != 0);
        let mut master = sheet
            .cell_style_formats
            .as_ref()
            .and_then(|cf| cf.cell_format.first().cloned())
            .unwrap_or_default();
        master.format_id = None;
        apply_patch_to_xf(sheet, &mut master, &patch.style)?;

        let xfs = sheet
            .cell_style_formats
            .as_mut()
            .expect("cellStyleXfs ensured");
        let master_id = match existing_master {
            Some(id) => {
                xfs.cell_format[id as usize] = master;
                id
            }
            None => {
                xfs.cell_format.push(master);
                let id = xfs.cell_format.len() - 1;
                xfs.count = Some(xfs.cell_format.len() as u32);
                id as u32
            }
        };

        let styles = sheet.cell_styles.as_mut().expect("cellStyles ensured");
        let builtin = patch.builtin_id;
        if let Some(cs) = styles
            .cell_style
            .iter_mut()
            .find(|cs| cs.name.as_deref() == Some(patch.name.as_str()))
        {
            cs.format_id = master_id;
            cs.builtin_id = builtin;
        } else {
            styles.cell_style.push(x::CellStyle {
                name: Some(patch.name.clone()),
                format_id: master_id,
                builtin_id: builtin,
                ..Default::default()
            });
            styles.count = Some(styles.cell_style.len() as u32);
        }
        Ok(NamedStyleInfo {
            name: patch.name,
            builtin_id: builtin,
        })
    }

    pub fn remove_named_style(&mut self, name: impl AsRef<str>) -> Result<Option<NamedStyleInfo>> {
        let name = name.as_ref();
        if name == "Normal" {
            return Err(ApiError::new(
                ApiErrorCode::UnsupportedStyle,
                "cannot remove the built-in Normal style",
            ));
        }
        let wb_part = self.doc.workbook_part().map_err(sdk_err_to_api)?.clone();
        let Some(part) = wb_part.workbook_styles_part(&mut self.doc) else {
            return Ok(None);
        };
        let sheet = part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let Some(styles) = sheet.cell_styles.as_mut() else {
            return Ok(None);
        };
        let Some(pos) = styles
            .cell_style
            .iter()
            .position(|cs| cs.name.as_deref() == Some(name))
        else {
            return Ok(None);
        };
        let removed = styles.cell_style.remove(pos);
        styles.count = Some(styles.cell_style.len() as u32);
        Ok(named_style_info(&removed))
    }
}

fn named_style_info(cs: &x::CellStyle) -> Option<NamedStyleInfo> {
    cs.name.as_ref().map(|name| NamedStyleInfo {
        name: name.to_string(),
        builtin_id: cs.builtin_id,
    })
}
