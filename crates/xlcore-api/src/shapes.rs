use std::str::FromStr;

use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::simple_type::{BooleanValue, CoordinateValue};
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, ApiWarning, ChartAnchor, ShapeInfo, ShapeLineEnd, ShapePatch,
};

use crate::errors::sdk_err_to_api;
use crate::refs::resolve_anchor;
use crate::{Result, Workbook};

impl Workbook {
    pub fn shapes(&mut self, sheet: Option<&str>) -> Result<Vec<ShapeInfo>> {
        let sheet_names: Vec<String> = match sheet {
            Some(name) => {
                if !self.sheet_exists(name)? {
                    return Err(ApiError::new(
                        ApiErrorCode::MissingSheet,
                        format!("sheet not found: {name}"),
                    )
                    .with_sheet(name));
                }
                vec![name.to_string()]
            }
            None => self
                .workbook_sheets()?
                .iter()
                .map(|s| s.name.as_str().to_string())
                .collect(),
        };

        let mut out = Vec::new();
        for sheet_name in &sheet_names {
            let ws_part = self.worksheet_part_for_sheet(sheet_name)?;
            let Some(drawings_part) = ws_part.drawings_part(&self.doc) else {
                continue;
            };
            let drawings_part = drawings_part.clone();
            let drawing_root = drawings_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?
                .clone();

            for (idx, choice) in drawing_root.worksheet_drawing_choice.iter().enumerate() {
                let (sp, anchor) = match choice {
                    xdr::WorksheetDrawingChoice::TwoCellAnchor(a) => {
                        let Some(xdr::TwoCellAnchorChoice::Shape(s)) =
                            a.two_cell_anchor_choice.as_ref()
                        else {
                            continue;
                        };
                        (s.as_ref(), two_cell_anchor_to_chart_anchor(a))
                    }
                    xdr::WorksheetDrawingChoice::OneCellAnchor(a) => {
                        let Some(xdr::OneCellAnchorChoice::Shape(s)) =
                            a.one_cell_anchor_choice.as_ref()
                        else {
                            continue;
                        };
                        (s.as_ref(), one_cell_anchor_to_chart_anchor(a))
                    }
                    _ => continue,
                };

                let nv = &sp.non_visual_shape_properties.non_visual_drawing_properties;
                let id = nv.id.to_string();
                let name = if nv.name.is_empty() {
                    format!("Shape {}", idx + 1)
                } else {
                    nv.name.as_str().to_string()
                };
                let preset = shape_preset(sp);
                let (rotation_degrees, flip_horizontal, flip_vertical) = shape_rotation_flip(sp);
                out.push(ShapeInfo {
                    sheet: sheet_name.clone(),
                    id,
                    name,
                    anchor,
                    preset,
                    fill_color: shape_fill_color(sp),
                    line_color: shape_line_color(sp),
                    text: shape_text(sp),
                    rotation_degrees,
                    flip_horizontal,
                    flip_vertical,
                });
            }
        }
        Ok(out)
    }

    pub fn set_shape(&mut self, sheet: impl AsRef<str>, patch: ShapePatch) -> Result<ShapeInfo> {
        let sheet = sheet.as_ref();
        if !self.sheet_exists(sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet}"),
            )
            .with_sheet(sheet));
        }
        let resolved = resolve_anchor(&patch.anchor)?;
        let preset = a::ShapeTypeValues::from_str(&patch.preset).map_err(|_| {
            ApiError::new(
                ApiErrorCode::InvalidShape,
                format!("unknown shape preset: {}", patch.preset),
            )
            .with_sheet(sheet)
        })?;
        let rotation_emu = match patch.rotation_degrees {
            Some(d) => Some(degrees_to_rot60000(d, sheet)?),
            None => None,
        };

        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let (drawings_part, fresh_drawings) = match ws_part.drawings_part(&self.doc) {
            Some(p) => (p.clone(), false),
            None => {
                let p: DrawingsPart = ws_part
                    .add_new_part_auto_id(&mut self.doc)
                    .map_err(sdk_err_to_api)?;
                let empty = xdr::WorksheetDrawing {
                    xmlns: crate::ooxml_header::drawing_root(),
                    xml_header: crate::ooxml_header::STANDALONE,
                    ..Default::default()
                };
                p.set_root_element(&mut self.doc, empty)
                    .map_err(sdk_err_to_api)?;
                (p, true)
            }
        };

        if fresh_drawings {
            let rid = drawings_part
                .relationship_id()
                .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new drawings part missing rid"))?
                .to_string();
            let ws = ws_part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            ws.drawing = Some(x::Drawing {
                xml_other_attrs: Vec::new(),
                id: rid,
            });
        }

        let drawing = drawings_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let shape_id = drawing.worksheet_drawing_choice.len() as u32 + 2;
        let shape_name = patch
            .name
            .clone()
            .unwrap_or_else(|| format!("Shape {shape_id}"));

        for warning in self.anchor_offset_overflow_warnings(sheet, &resolved)? {
            self.push_warning(warning);
        }

        let flip_h = patch.flip_horizontal.unwrap_or(false);
        let flip_v = patch.flip_vertical.unwrap_or(false);
        let xfrm_box = self.shape_box_emu(sheet, &resolved)?;
        let anchor = build_shape_two_cell_anchor(
            &patch,
            &resolved,
            &preset,
            shape_id,
            &shape_name,
            rotation_emu,
            flip_h,
            flip_v,
            xfrm_box,
        );

        let drawing_mut = drawings_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        drawing_mut
            .worksheet_drawing_choice
            .push(xdr::WorksheetDrawingChoice::TwoCellAnchor(Box::new(anchor)));

        Ok(ShapeInfo {
            sheet: sheet.to_string(),
            id: shape_id.to_string(),
            name: shape_name,
            anchor: resolved,
            preset: preset.to_string(),
            fill_color: patch.fill_color.map(normalize_hex),
            line_color: patch.line_color.map(normalize_hex),
            text: patch.text,
            rotation_degrees: patch.rotation_degrees.unwrap_or(0.0),
            flip_horizontal: flip_h,
            flip_vertical: flip_v,
        })
    }

    pub fn remove_shape(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
    ) -> Result<Option<ShapeInfo>> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let all = self.shapes(Some(&sheet))?;
        let Some(info) = all.iter().find(|c| c.id == id).cloned() else {
            return Ok(None);
        };

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let Some(drawings_part) = ws_part.drawings_part(&self.doc).map(|p| p.clone()) else {
            return Ok(None);
        };

        let drawing_mut = drawings_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        drawing_mut.worksheet_drawing_choice.retain(|choice| {
            let sp = match choice {
                xdr::WorksheetDrawingChoice::TwoCellAnchor(a) => {
                    match a.two_cell_anchor_choice.as_ref() {
                        Some(xdr::TwoCellAnchorChoice::Shape(s)) => Some(s.as_ref()),
                        _ => None,
                    }
                }
                xdr::WorksheetDrawingChoice::OneCellAnchor(a) => {
                    match a.one_cell_anchor_choice.as_ref() {
                        Some(xdr::OneCellAnchorChoice::Shape(s)) => Some(s.as_ref()),
                        _ => None,
                    }
                }
                _ => None,
            };
            match sp {
                Some(s) => {
                    s.non_visual_shape_properties
                        .non_visual_drawing_properties
                        .id
                        .to_string()
                        != id
                }
                None => true,
            }
        });

        Ok(Some(info))
    }

    fn anchor_offset_overflow_warnings(
        &mut self,
        sheet: &str,
        anchor: &ChartAnchor,
    ) -> Result<Vec<ApiWarning>> {
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let default_col_chars = ws
            .sheet_format_properties
            .as_ref()
            .and_then(|f| f.default_column_width)
            .unwrap_or(DEFAULT_COL_WIDTH_CHARS);
        let default_row_pt = ws
            .sheet_format_properties
            .as_ref()
            .map(|f| f.default_row_height)
            .filter(|h| *h > 0.0)
            .unwrap_or(DEFAULT_ROW_HEIGHT_PT);
        let col_width_emu = |col0: u32| -> i64 {
            let col1 = col0 + 1;
            let chars = ws
                .columns
                .first()
                .and_then(|cols| {
                    cols.column
                        .iter()
                        .find(|c| c.min <= col1 && col1 <= c.max)
                        .and_then(|c| c.width)
                })
                .unwrap_or(default_col_chars);
            col_chars_to_emu(chars)
        };
        let row_height_emu = |row0: u32| -> i64 {
            let row1 = row0 + 1;
            let pt = ws
                .sheet_data
                .row
                .iter()
                .find(|r| r.row_index == Some(row1))
                .and_then(|r| r.height)
                .unwrap_or(default_row_pt);
            (pt * EMU_PER_POINT as f64).round() as i64
        };

        let mut warnings = Vec::new();
        let mut check = |label: &str, axis: &str, offset: i64, extent: i64| {
            if offset > extent {
                warnings.push(
                    ApiWarning::new(
                        ApiErrorCode::LossyOperation,
                        format!(
                            "{label} {axis} offset {offset} EMU exceeds the referenced cell ({extent} EMU); Excel clamps anchor offsets to the cell, so the rendered position will differ"
                        ),
                    )
                    .with_sheet(sheet),
                );
            }
        };
        check(
            "from",
            "column",
            anchor.from_column_offset_emu.unwrap_or(0),
            col_width_emu(anchor.from_column),
        );
        check(
            "from",
            "row",
            anchor.from_row_offset_emu.unwrap_or(0),
            row_height_emu(anchor.from_row),
        );
        check(
            "to",
            "column",
            anchor.to_column_offset_emu.unwrap_or(0),
            col_width_emu(anchor.to_column),
        );
        check(
            "to",
            "row",
            anchor.to_row_offset_emu.unwrap_or(0),
            row_height_emu(anchor.to_row),
        );
        Ok(warnings)
    }

    fn shape_box_emu(&mut self, sheet: &str, anchor: &ChartAnchor) -> Result<(i64, i64, i64, i64)> {
        let ws_part = self.worksheet_part_for_sheet(sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let default_col_chars = ws
            .sheet_format_properties
            .as_ref()
            .and_then(|f| f.default_column_width)
            .unwrap_or(DEFAULT_COL_WIDTH_CHARS);
        let default_row_pt = ws
            .sheet_format_properties
            .as_ref()
            .map(|f| f.default_row_height)
            .filter(|h| *h > 0.0)
            .unwrap_or(DEFAULT_ROW_HEIGHT_PT);
        let col_width_chars = |col0: u32| -> f64 {
            let col1 = col0 + 1;
            ws.columns
                .first()
                .and_then(|cols| {
                    cols.column
                        .iter()
                        .find(|c| c.min <= col1 && col1 <= c.max)
                        .and_then(|c| c.width)
                })
                .unwrap_or(default_col_chars)
        };
        let row_height_pt = |row0: u32| -> f64 {
            let row1 = row0 + 1;
            ws.sheet_data
                .row
                .iter()
                .find(|r| r.row_index == Some(row1))
                .and_then(|r| r.height)
                .unwrap_or(default_row_pt)
        };
        let col_left_emu = |col0: u32| -> i64 {
            (0..col0)
                .map(|c| col_chars_to_emu(col_width_chars(c)))
                .sum()
        };
        let row_top_emu = |row0: u32| -> i64 {
            (0..row0)
                .map(|r| (row_height_pt(r) * EMU_PER_POINT as f64).round() as i64)
                .sum()
        };

        let from_x = col_left_emu(anchor.from_column) + anchor.from_column_offset_emu.unwrap_or(0);
        let from_y = row_top_emu(anchor.from_row) + anchor.from_row_offset_emu.unwrap_or(0);
        let to_x = col_left_emu(anchor.to_column) + anchor.to_column_offset_emu.unwrap_or(0);
        let to_y = row_top_emu(anchor.to_row) + anchor.to_row_offset_emu.unwrap_or(0);
        Ok((
            from_x,
            from_y,
            (to_x - from_x).max(0),
            (to_y - from_y).max(0),
        ))
    }
}

const EMU_PER_PIXEL: i64 = 9525;
const EMU_PER_POINT: i64 = 12700;
const DEFAULT_COL_WIDTH_CHARS: f64 = 8.43;
const DEFAULT_ROW_HEIGHT_PT: f64 = 15.0;
const MAX_DIGIT_WIDTH: f64 = 7.0;

fn col_chars_to_emu(chars: f64) -> i64 {
    let px =
        (((256.0 * chars + (128.0 / MAX_DIGIT_WIDTH).trunc()) / 256.0) * MAX_DIGIT_WIDTH).trunc();
    px as i64 * EMU_PER_PIXEL
}

fn shape_preset(sp: &xdr::Shape) -> String {
    match sp.shape_properties.shape_properties_choice1.as_ref() {
        Some(xdr::ShapePropertiesChoice::PresetGeometry(g)) => g.preset.to_string(),
        _ => "rect".to_string(),
    }
}

fn shape_rotation_flip(sp: &xdr::Shape) -> (f64, bool, bool) {
    let Some(xfrm) = sp.shape_properties.transform2_d.as_ref() else {
        return (0.0, false, false);
    };
    let rot = xfrm.rotation.map(|r| r as f64 / 60_000.0).unwrap_or(0.0);
    let fh = xfrm.horizontal_flip.map(bool::from).unwrap_or(false);
    let fv = xfrm.vertical_flip.map(bool::from).unwrap_or(false);
    (rot, fh, fv)
}

fn solid_fill_hex(fill: &a::SolidFill) -> Option<String> {
    match fill.solid_fill_choice.as_ref()? {
        a::SolidFillChoice::RgbColorModelHex(c) => Some(format!("#{}", c.val.to_uppercase())),
        _ => None,
    }
}

fn shape_fill_color(sp: &xdr::Shape) -> Option<String> {
    match sp.shape_properties.shape_properties_choice2.as_ref()? {
        xdr::ShapePropertiesChoice2::SolidFill(f) => solid_fill_hex(f),
        _ => None,
    }
}

fn shape_line_color(sp: &xdr::Shape) -> Option<String> {
    let outline = sp.shape_properties.outline.as_ref()?;
    match outline.outline_choice1.as_ref()? {
        a::OutlineChoice::SolidFill(f) => solid_fill_hex(f),
        _ => None,
    }
}

fn shape_text(sp: &xdr::Shape) -> Option<String> {
    let tb = sp.text_body.as_ref()?;
    let mut parts: Vec<String> = Vec::new();
    for p in &tb.paragraph {
        let mut line = String::new();
        for c in &p.paragraph_choice {
            if let a::ParagraphChoice::Run(r) = c {
                line.push_str(&r.text);
            }
        }
        parts.push(line);
    }
    let joined = parts.join("\n");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn normalize_hex(s: String) -> String {
    let trimmed = s.trim_start_matches('#');
    format!("#{}", trimmed.to_uppercase())
}

fn hex_to_srgb(s: &str) -> a::SolidFill {
    let val = s.trim_start_matches('#').to_uppercase();
    a::SolidFill {
        solid_fill_choice: Some(a::SolidFillChoice::RgbColorModelHex(Box::new(
            a::RgbColorModelHex {
                val,
                ..Default::default()
            },
        ))),
        ..Default::default()
    }
}

fn degrees_to_rot60000(deg: f64, sheet: &str) -> Result<i32> {
    if !deg.is_finite() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidShape,
            "rotation_degrees must be finite",
        )
        .with_sheet(sheet));
    }
    let normalized = deg.rem_euclid(360.0);
    Ok((normalized * 60_000.0).round() as i32)
}

fn two_cell_anchor_to_chart_anchor(anchor: &xdr::TwoCellAnchor) -> ChartAnchor {
    let from = &anchor.from_marker;
    let to = &anchor.to_marker;
    ChartAnchor {
        from_column: from.column_id.max(0) as u32,
        from_row: from.row_id.max(0) as u32,
        to_column: to.column_id.max(0) as u32,
        to_row: to.row_id.max(0) as u32,
        from_column_offset_emu: Some(from.column_offset.to_emu()),
        from_row_offset_emu: Some(from.row_offset.to_emu()),
        to_column_offset_emu: Some(to.column_offset.to_emu()),
        to_row_offset_emu: Some(to.row_offset.to_emu()),
    }
}

fn one_cell_anchor_to_chart_anchor(anchor: &xdr::OneCellAnchor) -> ChartAnchor {
    let from = &anchor.from_marker;
    ChartAnchor {
        from_column: from.column_id.max(0) as u32,
        from_row: from.row_id.max(0) as u32,
        to_column: from.column_id.max(0) as u32,
        to_row: from.row_id.max(0) as u32,
        from_column_offset_emu: Some(from.column_offset.to_emu()),
        from_row_offset_emu: Some(from.row_offset.to_emu()),
        to_column_offset_emu: None,
        to_row_offset_emu: None,
    }
}

fn build_text_body(patch: &ShapePatch) -> Option<Box<xdr::TextBody>> {
    let text = patch.text.as_ref()?;
    let mk_run_props = || {
        let mut rp = a::RunProperties {
            language: Some("en-US".to_string()),
            ..Default::default()
        };
        if let Some(sz) = patch.font_size_pt {
            rp.font_size = Some((sz * 100.0).round() as i32);
        }
        if patch.bold == Some(true) {
            rp.bold = Some(BooleanValue::from_bool(true));
        }
        if patch.italic == Some(true) {
            rp.italic = Some(BooleanValue::from_bool(true));
        }
        if let Some(color) = patch.text_color.as_ref() {
            rp.run_properties_choice1 = Some(a::RunPropertiesChoice::SolidFill(Box::new(
                hex_to_srgb(color),
            )));
        }
        if patch.underline == Some(true) {
            rp.underline = Some(a::TextUnderlineValues::Single);
        }
        rp
    };
    let alignment = patch.align.as_deref().and_then(text_alignment);
    let mk_para_props = || {
        alignment.map(|al| {
            Box::new(a::ParagraphProperties {
                alignment: Some(al),
                ..Default::default()
            })
        })
    };
    let paragraphs = text
        .split('\n')
        .map(|line| {
            let run = a::Run {
                run_properties: Some(Box::new(mk_run_props())),
                text: line.to_string(),
                ..Default::default()
            };
            a::Paragraph {
                paragraph_properties: mk_para_props(),
                paragraph_choice: vec![a::ParagraphChoice::Run(Box::new(run))],
                ..Default::default()
            }
        })
        .collect();
    let body_properties = a::BodyProperties {
        anchor: patch.vertical_align.as_deref().and_then(text_anchor),
        ..Default::default()
    };
    Some(Box::new(xdr::TextBody {
        body_properties: Box::new(body_properties),
        paragraph: paragraphs,
        ..Default::default()
    }))
}

fn text_alignment(v: &str) -> Option<a::TextAlignmentTypeValues> {
    Some(match v {
        "l" => a::TextAlignmentTypeValues::Left,
        "ctr" => a::TextAlignmentTypeValues::Center,
        "r" => a::TextAlignmentTypeValues::Right,
        "just" => a::TextAlignmentTypeValues::Justified,
        _ => return None,
    })
}

fn text_anchor(v: &str) -> Option<a::TextAnchoringTypeValues> {
    Some(match v {
        "t" => a::TextAnchoringTypeValues::Top,
        "ctr" => a::TextAnchoringTypeValues::Center,
        "b" => a::TextAnchoringTypeValues::Bottom,
        _ => return None,
    })
}

fn line_end_type(v: &str) -> Option<a::LineEndValues> {
    Some(match v {
        "none" => a::LineEndValues::None,
        "triangle" => a::LineEndValues::Triangle,
        "stealth" => a::LineEndValues::Stealth,
        "diamond" => a::LineEndValues::Diamond,
        "oval" => a::LineEndValues::Oval,
        "arrow" => a::LineEndValues::Arrow,
        _ => return None,
    })
}

fn line_end_width(v: &str) -> Option<a::LineEndWidthValues> {
    Some(match v {
        "sm" => a::LineEndWidthValues::Small,
        "med" => a::LineEndWidthValues::Medium,
        "lg" => a::LineEndWidthValues::Large,
        _ => return None,
    })
}

fn line_end_length(v: &str) -> Option<a::LineEndLengthValues> {
    Some(match v {
        "sm" => a::LineEndLengthValues::Small,
        "med" => a::LineEndLengthValues::Medium,
        "lg" => a::LineEndLengthValues::Large,
        _ => return None,
    })
}

fn build_head_end(e: &ShapeLineEnd) -> a::HeadEnd {
    a::HeadEnd {
        r#type: e.r#type.as_deref().and_then(line_end_type),
        width: e.w.as_deref().and_then(line_end_width),
        length: e.len.as_deref().and_then(line_end_length),
    }
}

fn build_tail_end(e: &ShapeLineEnd) -> a::TailEnd {
    a::TailEnd {
        r#type: e.r#type.as_deref().and_then(line_end_type),
        width: e.w.as_deref().and_then(line_end_width),
        length: e.len.as_deref().and_then(line_end_length),
    }
}

fn build_shape_two_cell_anchor(
    patch: &ShapePatch,
    anchor: &ChartAnchor,
    preset: &a::ShapeTypeValues,
    shape_id: u32,
    shape_name: &str,
    rotation_60000ths: Option<i32>,
    flip_h: bool,
    flip_v: bool,
    xfrm_box: (i64, i64, i64, i64),
) -> xdr::TwoCellAnchor {
    let from = xdr::FromMarker {
        column_id: anchor.from_column as i32,
        column_offset: CoordinateValue::Emu(anchor.from_column_offset_emu.unwrap_or(0)),
        row_id: anchor.from_row as i32,
        row_offset: CoordinateValue::Emu(anchor.from_row_offset_emu.unwrap_or(0)),
        ..Default::default()
    };
    let to = xdr::ToMarker {
        column_id: anchor.to_column as i32,
        column_offset: CoordinateValue::Emu(anchor.to_column_offset_emu.unwrap_or(0)),
        row_id: anchor.to_row as i32,
        row_offset: CoordinateValue::Emu(anchor.to_row_offset_emu.unwrap_or(0)),
        ..Default::default()
    };

    let nv_drawing = xdr::NonVisualDrawingProperties {
        id: shape_id,
        name: shape_name.to_string(),
        ..Default::default()
    };
    let nv_sp = xdr::NonVisualShapeProperties {
        non_visual_drawing_properties: Box::new(nv_drawing),
        non_visual_shape_drawing_properties: Box::new(
            xdr::NonVisualShapeDrawingProperties::default(),
        ),
        ..Default::default()
    };

    let prst = a::PresetGeometry {
        preset: preset.clone(),
        adjust_value_list: Some(a::AdjustValueList::default()),
        ..Default::default()
    };
    let has_rotation = rotation_60000ths.map(|r| r != 0).unwrap_or(false);
    let xfrm = (has_rotation || flip_h || flip_v).then(|| {
        let (off_x, off_y, ext_cx, ext_cy) = xfrm_box;
        Box::new(a::Transform2D {
            rotation: rotation_60000ths,
            horizontal_flip: flip_h.then(|| BooleanValue::from_bool(true)),
            vertical_flip: flip_v.then(|| BooleanValue::from_bool(true)),
            offset: Some(a::Offset {
                x: CoordinateValue::Emu(off_x),
                y: CoordinateValue::Emu(off_y),
                ..Default::default()
            }),
            extents: Some(a::Extents {
                cx: CoordinateValue::Emu(ext_cx),
                cy: CoordinateValue::Emu(ext_cy),
                ..Default::default()
            }),
            ..Default::default()
        })
    });

    let fill = patch
        .fill_color
        .as_ref()
        .map(|c| xdr::ShapePropertiesChoice2::SolidFill(Box::new(hex_to_srgb(c))));

    let outline = (patch.line_color.is_some()
        || patch.head_end.is_some()
        || patch.tail_end.is_some())
    .then(|| {
        Box::new(a::Outline {
            width: patch.line_width_emu.map(|w| w as i32),
            outline_choice1: patch
                .line_color
                .as_ref()
                .map(|c| a::OutlineChoice::SolidFill(Box::new(hex_to_srgb(c)))),
            head_end: patch.head_end.as_ref().map(build_head_end),
            tail_end: patch.tail_end.as_ref().map(build_tail_end),
            ..Default::default()
        })
    });

    let sp_pr = xdr::ShapeProperties {
        transform2_d: xfrm,
        shape_properties_choice1: Some(xdr::ShapePropertiesChoice::PresetGeometry(Box::new(prst))),
        shape_properties_choice2: fill,
        outline,
        ..Default::default()
    };

    let sp = xdr::Shape {
        r#macro: Some(String::new()),
        non_visual_shape_properties: Box::new(nv_sp),
        shape_properties: Box::new(sp_pr),
        text_body: build_text_body(patch),
        ..Default::default()
    };

    xdr::TwoCellAnchor {
        from_marker: Box::new(from),
        to_marker: Box::new(to),
        two_cell_anchor_choice: Some(xdr::TwoCellAnchorChoice::Shape(Box::new(sp))),
        client_data: Box::new(xdr::ClientData::default()),
        ..Default::default()
    }
}
