use ooxmlsdk::parts::workbook_styles_part::WorkbookStylesPart;
use xlcore_io::spreadsheetml as x;
pub use xlcore_types::{
    AlignmentPatch, BorderLinePatch, BorderLineStyle, BorderPatch, FillPatch, FontPatch,
    HorizontalAlign, StylePatch, UnderlinePatch, VerticalAlign,
};

use crate::{sdk_err_to_api, ApiError, ApiErrorCode, Result};

pub(crate) fn ensure_styles_part(
    doc: &mut xlcore_io::SpreadsheetDocument,
) -> Result<WorkbookStylesPart> {
    let wb_part = doc.workbook_part().map_err(sdk_err_to_api)?.clone();
    if let Some(part) = wb_part.workbook_styles_part(doc) {
        return Ok(part.clone());
    }
    let part: WorkbookStylesPart = wb_part
        .add_new_part_auto_id(doc)
        .map_err(sdk_err_to_api)?;
    part.set_root_element(doc, default_stylesheet())
        .map_err(sdk_err_to_api)?;
    Ok(part)
}

fn default_stylesheet() -> x::Stylesheet {
    x::Stylesheet {
        fonts: Some(x::Fonts {
            count: Some(1),
            x_font: vec![default_font()],
            ..Default::default()
        }),
        fills: Some(x::Fills {
            count: Some(2),
            x_fill: vec![pattern_fill_none(), pattern_fill_gray125()],
        }),
        borders: Some(x::Borders {
            count: Some(1),
            x_border: vec![x::Border::default()],
        }),
        cell_style_formats: Some(x::CellStyleFormats {
            count: Some(1),
            x_xf: vec![x::CellFormat {
                number_format_id: Some(0),
                font_id: Some(0),
                fill_id: Some(0),
                border_id: Some(0),
                ..Default::default()
            }],
        }),
        cell_formats: Some(x::CellFormats {
            count: Some(1),
            x_xf: vec![x::CellFormat {
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
            x_cell_style: vec![x::CellStyle {
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
        font_size: Some(x::FontSize { val: 11.0, ..Default::default() }),
        color: Some(x::Color {
            theme: Some(1),
            ..Default::default()
        }),
        font_name: Some(x::FontName {
            val: "Calibri".to_string(),
            ..Default::default()
        }),
        font_family_numbering: Some(x::FontFamilyNumbering {
            val: 2,
            ..Default::default()
        }),
        font_scheme: Some(x::FontScheme {
            val: x::FontSchemeValues::Minor,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn pattern_fill_none() -> x::Fill {
    x::Fill {
        fill_choice: Some(x::FillChoice::XPatternFill(Box::new(x::PatternFill {
            pattern_type: Some(x::PatternValues::None),
            ..Default::default()
        }))),
        ..Default::default()
    }
}

fn pattern_fill_gray125() -> x::Fill {
    x::Fill {
        fill_choice: Some(x::FillChoice::XPatternFill(Box::new(x::PatternFill {
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
        .filter(|i| *i < sheet.cell_formats.as_ref().map(|cf| cf.x_xf.len()).unwrap_or(0))
        .unwrap_or(0);
    let base_xf = sheet
        .cell_formats
        .as_ref()
        .and_then(|cf| cf.x_xf.get(base).cloned())
        .unwrap_or_default();

    let mut new_xf = base_xf;
    new_xf.format_id = Some(0);

    if let Some(font_patch) = patch.font.as_ref() {
        let current = new_xf.font_id.unwrap_or(0) as usize;
        let new_font = build_font(sheet, current, font_patch)?;
        new_xf.font_id = Some(intern_font(sheet, new_font));
        new_xf.apply_font = Some(true);
    }
    if let Some(fill_patch) = patch.fill.as_ref() {
        let new_fill = build_fill(fill_patch)?;
        new_xf.fill_id = Some(intern_fill(sheet, new_fill));
        new_xf.apply_fill = Some(true);
    }
    if let Some(border_patch) = patch.border.as_ref() {
        let current = new_xf.border_id.unwrap_or(0) as usize;
        let new_border = build_border(sheet, current, border_patch)?;
        new_xf.border_id = Some(intern_border(sheet, new_border));
        new_xf.apply_border = Some(true);
    }
    if let Some(num_fmt) = patch.number_format.as_deref() {
        let new_num_fmt_id = intern_num_fmt(sheet, num_fmt);
        new_xf.number_format_id = Some(new_num_fmt_id);
        new_xf.apply_number_format = Some(true);
    }
    if let Some(align_patch) = patch.alignment.as_ref() {
        apply_alignment(&mut new_xf, align_patch);
        new_xf.apply_alignment = Some(true);
    }

    Ok(intern_cell_format(sheet, new_xf))
}

fn ensure_default_collections(sheet: &mut x::Stylesheet) {
    if sheet.fonts.is_none() {
        sheet.fonts = Some(x::Fonts {
            count: Some(1),
            x_font: vec![default_font()],
            ..Default::default()
        });
    }
    if sheet.fills.is_none() {
        sheet.fills = Some(x::Fills {
            count: Some(2),
            x_fill: vec![pattern_fill_none(), pattern_fill_gray125()],
        });
    } else {
        let fills = sheet.fills.as_mut().unwrap();
        if fills.x_fill.is_empty() {
            fills.x_fill.push(pattern_fill_none());
            fills.x_fill.push(pattern_fill_gray125());
        }
    }
    if sheet.borders.is_none() {
        sheet.borders = Some(x::Borders {
            count: Some(1),
            x_border: vec![x::Border::default()],
        });
    }
    if sheet.cell_formats.is_none() {
        sheet.cell_formats = Some(x::CellFormats {
            count: Some(1),
            x_xf: vec![x::CellFormat {
                number_format_id: Some(0),
                font_id: Some(0),
                fill_id: Some(0),
                border_id: Some(0),
                format_id: Some(0),
                ..Default::default()
            }],
        });
    }
}

fn build_font(sheet: &x::Stylesheet, current: usize, patch: &FontPatch) -> Result<x::Font> {
    let base = sheet
        .fonts
        .as_ref()
        .and_then(|f| f.x_font.get(current).cloned())
        .unwrap_or_else(default_font);
    let mut font = base;
    if let Some(name) = patch.name.as_deref() {
        font.font_name = Some(x::FontName {
            val: name.to_string(),
            ..Default::default()
        });
    }
    if let Some(size) = patch.size {
        font.font_size = Some(x::FontSize { val: size, ..Default::default() });
    }
    if let Some(bold) = patch.bold {
        font.bold = if bold {
            Some(x::Bold { val: None, ..Default::default() })
        } else {
            None
        };
    }
    if let Some(italic) = patch.italic {
        font.italic = if italic {
            Some(x::Italic { val: None, ..Default::default() })
        } else {
            None
        };
    }
    if let Some(strike) = patch.strike {
        font.strike = if strike {
            Some(x::Strike { val: None, ..Default::default() })
        } else {
            None
        };
    }
    if let Some(underline) = patch.underline {
        font.underline = match underline {
            UnderlinePatch::None => None,
            UnderlinePatch::Single => Some(x::Underline { val: None, ..Default::default() }),
            UnderlinePatch::Double => Some(x::Underline {
                val: Some(x::UnderlineValues::Double),
                ..Default::default()
            }),
        };
    }
    if let Some(color) = patch.color.as_deref() {
        font.color = Some(parse_color(color)?);
    }
    Ok(font)
}

fn build_fill(patch: &FillPatch) -> Result<x::Fill> {
    let Some(color) = patch.color.as_deref() else {
        return Ok(pattern_fill_none());
    };
    let parsed = parse_color(color)?;
    let fg = x::ForegroundColor {
        rgb: parsed.rgb.clone(),
        theme: parsed.theme,
        tint: parsed.tint,
        indexed: parsed.indexed,
        auto: parsed.auto,
    };
    let bg = x::BackgroundColor {
        indexed: Some(64),
        ..Default::default()
    };
    Ok(x::Fill {
        fill_choice: Some(x::FillChoice::XPatternFill(Box::new(x::PatternFill {
            pattern_type: Some(x::PatternValues::Solid),
            foreground_color: Some(fg),
            background_color: Some(bg),
        }))),
        ..Default::default()
    })
}

fn build_border(sheet: &x::Stylesheet, current: usize, patch: &BorderPatch) -> Result<x::Border> {
    let mut border = sheet
        .borders
        .as_ref()
        .and_then(|b| b.x_border.get(current).cloned())
        .unwrap_or_default();
    let resolved = |side: &Option<BorderLinePatch>| -> Option<BorderLinePatch> {
        side.clone().or_else(|| patch.all.clone())
    };
    apply_side(&mut border.left_border, resolved(&patch.left).as_ref())?;
    apply_side(&mut border.right_border, resolved(&patch.right).as_ref())?;
    apply_side(&mut border.top_border, resolved(&patch.top).as_ref())?;
    apply_side(&mut border.bottom_border, resolved(&patch.bottom).as_ref())?;
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
        align.wrap_text = Some(wrap);
    }
    if let Some(indent) = patch.indent {
        align.indent = Some(indent);
    }
    if let Some(rot) = patch.text_rotation {
        let normalized = if rot < 0 { (90 - rot) as u32 } else { rot as u32 };
        align.text_rotation = Some(normalized);
    }
}

fn intern_font(sheet: &mut x::Stylesheet, font: x::Font) -> u32 {
    let fonts = sheet.fonts.as_mut().expect("fonts ensured");
    if let Some(idx) = fonts.x_font.iter().position(|f| fonts_equal(f, &font)) {
        return idx as u32;
    }
    fonts.x_font.push(font);
    let idx = fonts.x_font.len() - 1;
    fonts.count = Some(fonts.x_font.len() as u32);
    idx as u32
}

fn intern_fill(sheet: &mut x::Stylesheet, fill: x::Fill) -> u32 {
    let fills = sheet.fills.as_mut().expect("fills ensured");
    if let Some(idx) = fills.x_fill.iter().position(|f| fills_equal(f, &fill)) {
        return idx as u32;
    }
    fills.x_fill.push(fill);
    let idx = fills.x_fill.len() - 1;
    fills.count = Some(fills.x_fill.len() as u32);
    idx as u32
}

fn intern_border(sheet: &mut x::Stylesheet, border: x::Border) -> u32 {
    let borders = sheet.borders.as_mut().expect("borders ensured");
    if let Some(idx) = borders.x_border.iter().position(|b| borders_equal(b, &border)) {
        return idx as u32;
    }
    borders.x_border.push(border);
    let idx = borders.x_border.len() - 1;
    borders.count = Some(borders.x_border.len() as u32);
    idx as u32
}

fn intern_cell_format(sheet: &mut x::Stylesheet, xf: x::CellFormat) -> u32 {
    let cfs = sheet.cell_formats.as_mut().expect("cellXfs ensured");
    if let Some(idx) = cfs.x_xf.iter().position(|x| xfs_equal(x, &xf)) {
        return idx as u32;
    }
    cfs.x_xf.push(xf);
    let idx = cfs.x_xf.len() - 1;
    cfs.count = Some(cfs.x_xf.len() as u32);
    idx as u32
}

fn intern_num_fmt(sheet: &mut x::Stylesheet, code: &str) -> u32 {
    if let Some(id) = builtin_num_fmt_id(code) {
        return id;
    }
    let formats = sheet
        .numbering_formats
        .get_or_insert_with(Default::default);
    if let Some(existing) = formats
        .x_num_fmt
        .iter()
        .find(|nf| nf.format_code.as_str() == code)
    {
        return existing.number_format_id;
    }
    let next_id = formats
        .x_num_fmt
        .iter()
        .map(|nf| nf.number_format_id)
        .max()
        .map(|id| id.max(163))
        .unwrap_or(163)
        + 1;
    formats.x_num_fmt.push(x::NumberingFormat {
        number_format_id: next_id,
        format_code: code.to_string(),
        ..Default::default()
    });
    formats.count = Some(formats.x_num_fmt.len() as u32);
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

fn parse_color(text: &str) -> Result<x::Color> {
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
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        f.font_name.as_ref().map(|n| n.val.as_str()).unwrap_or(""),
        f.font_size.as_ref().map(|s| s.val).unwrap_or(0.0),
        f.bold.is_some(),
        f.italic.is_some(),
        f.strike.is_some(),
        f.underline.as_ref().map(|u| format!("{:?}", u.val)).unwrap_or_default(),
        f.color.as_ref().map(color_sig).unwrap_or_default(),
        f.font_scheme.as_ref().map(|s| format!("{:?}", s.val)).unwrap_or_default(),
    )
}

fn fills_equal(a: &x::Fill, b: &x::Fill) -> bool {
    fill_signature(a) == fill_signature(b)
}

fn fill_signature(f: &x::Fill) -> String {
    match &f.fill_choice {
        Some(x::FillChoice::XPatternFill(pf)) => format!(
            "pat|{:?}|{}|{}",
            pf.pattern_type,
            pf.foreground_color.as_ref().map(fg_sig).unwrap_or_default(),
            pf.background_color.as_ref().map(bg_sig).unwrap_or_default(),
        ),
        Some(x::FillChoice::XGradientFill(_)) => "gradient".to_string(),
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
            Some(side) => format!("{:?}|{}", side.style(), side.color().map(color_sig).unwrap_or_default()),
            None => "none".to_string(),
        }
    }
    format!(
        "{}|{}|{}|{}",
        side(&b.left_border),
        side(&b.right_border),
        side(&b.top_border),
        side(&b.bottom_border),
    )
}

trait BorderSideRead {
    fn style(&self) -> Option<x::BorderStyleValues>;
    fn color(&self) -> Option<&x::Color>;
}
macro_rules! impl_border_side_read {
    ($ty:ty) => {
        impl BorderSideRead for $ty {
            fn style(&self) -> Option<x::BorderStyleValues> { self.style }
            fn color(&self) -> Option<&x::Color> { self.color.as_ref() }
        }
    };
}
impl_border_side_read!(x::LeftBorder);
impl_border_side_read!(x::RightBorder);
impl_border_side_read!(x::TopBorder);
impl_border_side_read!(x::BottomBorder);

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
        && alignment_equal(&a.alignment, &b.alignment)
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
        }
        _ => false,
    }
}
