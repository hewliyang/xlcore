use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::parts::image_part::ImagePart;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use ooxmlsdk::simple_type::CoordinateValue;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, ChartAnchor, ImageFormat, ImageInfo, ImagePatch,
};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

impl Workbook {
    pub fn images(&mut self, sheet: Option<&str>) -> Result<Vec<ImageInfo>> {
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
            let image_by_rid: Vec<(String, ImagePart)> = drawings_part
                .image_parts(&self.doc)
                .filter_map(|p| p.relationship_id().map(|r| (r.to_string(), p.clone())))
                .collect();

            let drawing_root = drawings_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?
                .clone();

            for (idx, choice) in drawing_root.worksheet_drawing_choice.iter().enumerate() {
                let (pic, anchor) = match choice {
                    xdr::WorksheetDrawingChoice::TwoCellAnchor(a) => {
                        let Some(xdr::TwoCellAnchorChoice::Picture(p)) = a.two_cell_anchor_choice.as_ref() else { continue };
                        (p.as_ref(), two_cell_anchor_to_chart_anchor(a))
                    }
                    xdr::WorksheetDrawingChoice::OneCellAnchor(a) => {
                        let Some(xdr::OneCellAnchorChoice::Picture(p)) = a.one_cell_anchor_choice.as_ref() else { continue };
                        (p.as_ref(), one_cell_anchor_to_chart_anchor(a))
                    }
                    _ => continue,
                };
                let Some(rid) = picture_embed_rid(pic) else { continue };
                let Some((_, ipart)) = image_by_rid.iter().find(|(r, _)| r == &rid) else { continue };
                let bytes = ipart.data(&self.doc).map(|b| b.len() as u64).unwrap_or(0);
                let format = ipart
                    .data(&self.doc)
                    .and_then(ImageFormat::sniff)
                    .unwrap_or(ImageFormat::Png);
                let name_raw = pic
                    .non_visual_picture_properties
                    .non_visual_drawing_properties
                    .name
                    .as_str();
                let name = if name_raw.is_empty() {
                    format!("Image {}", idx + 1)
                } else {
                    name_raw.to_string()
                };
                out.push(ImageInfo {
                    sheet: sheet_name.clone(),
                    id: rid,
                    name,
                    anchor,
                    format,
                    byte_len: bytes,
                });
            }
        }
        Ok(out)
    }

    pub fn set_image(&mut self, patch: ImagePatch) -> Result<ImageInfo> {
        if patch.bytes.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidImage,
                "image bytes must not be empty",
            )
            .with_sheet(&patch.sheet));
        }
        if !self.sheet_exists(&patch.sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {}", patch.sheet),
            )
            .with_sheet(&patch.sheet));
        }
        let format = match patch.format {
            Some(f) => f,
            None => ImageFormat::sniff(&patch.bytes).ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::InvalidImage,
                    "unable to detect image format; specify `format` explicitly",
                )
                .with_sheet(&patch.sheet)
            })?,
        };

        let ws_part = self.worksheet_part_for_sheet(&patch.sheet)?;
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

        let image_part = drawings_part
            .add_image_part(&mut self.doc, format.content_type())
            .map_err(sdk_err_to_api)?;
        image_part
            .set_data(&mut self.doc, patch.bytes.clone())
            .map_err(sdk_err_to_api)?;
        let image_rid = image_part
            .relationship_id()
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new image part missing rid"))?
            .to_string();

        let drawing = drawings_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let pic_index = drawing.worksheet_drawing_choice.len() + 1;
        let pic_name = patch.name.clone().unwrap_or_else(|| format!("Image {pic_index}"));

        let anchor = build_picture_two_cell_anchor(&patch.anchor, pic_index, &pic_name, &image_rid);

        let drawing_mut = drawings_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        drawing_mut
            .worksheet_drawing_choice
            .push(xdr::WorksheetDrawingChoice::TwoCellAnchor(Box::new(anchor)));

        Ok(ImageInfo {
            sheet: patch.sheet.clone(),
            id: image_rid,
            name: pic_name,
            anchor: patch.anchor,
            format,
            byte_len: patch.bytes.len() as u64,
        })
    }

    pub fn remove_image(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
    ) -> Result<Option<ImageInfo>> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let all = self.images(Some(&sheet))?;
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
            let pic = match choice {
                xdr::WorksheetDrawingChoice::TwoCellAnchor(a) => match a.two_cell_anchor_choice.as_ref() {
                    Some(xdr::TwoCellAnchorChoice::Picture(p)) => Some(p.as_ref()),
                    _ => None,
                },
                xdr::WorksheetDrawingChoice::OneCellAnchor(a) => match a.one_cell_anchor_choice.as_ref() {
                    Some(xdr::OneCellAnchorChoice::Picture(p)) => Some(p.as_ref()),
                    _ => None,
                },
                _ => None,
            };
            match pic.and_then(picture_embed_rid) {
                Some(rid) => rid != id,
                None => true,
            }
        });

        let _ = drawings_part
            .delete_part_by_id(&mut self.doc, id.as_str())
            .map_err(sdk_err_to_api)?;

        Ok(Some(info))
    }
}

fn picture_embed_rid(pic: &xdr::Picture) -> Option<String> {
    pic.blip_fill
        .blip
        .as_ref()
        .and_then(|b| b.embed.as_ref())
        .map(|s| s.as_str().to_string())
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

fn build_picture_two_cell_anchor(
    anchor: &ChartAnchor,
    pic_index: usize,
    pic_name: &str,
    embed_rid: &str,
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
        id: pic_index as u32 + 1,
        name: pic_name.to_string(),
        ..Default::default()
    };
    let nv_pic = xdr::NonVisualPictureProperties {
        non_visual_drawing_properties: Box::new(nv_drawing),
        non_visual_picture_drawing_properties: Box::new(xdr::NonVisualPictureDrawingProperties {
            prefer_relative_resize: Some(ooxmlsdk::simple_type::BooleanValue::from_bool(false)),
            ..Default::default()
        }),
    };

    let blip = a::Blip {
        embed: Some(embed_rid.to_string()),
        ..Default::default()
    };
    let blip_fill = xdr::BlipFill {
        blip: Some(Box::new(blip)),
        blip_fill_choice: Some(xdr::BlipFillChoice::Stretch(Box::new(a::Stretch {
            fill_rectangle: Some(a::FillRectangle::default()),
            ..Default::default()
        }))),
        ..Default::default()
    };

    let prst = a::PresetGeometry {
        preset: a::ShapeTypeValues::Rectangle,
        adjust_value_list: Some(a::AdjustValueList::default()),
        ..Default::default()
    };
    let xfrm = a::Transform2D {
        offset: Some(a::Offset {
            x: CoordinateValue::Emu(0),
            y: CoordinateValue::Emu(0),
        }),
        extents: Some(a::Extents {
            cx: CoordinateValue::Emu(0),
            cy: CoordinateValue::Emu(0),
        }),
        ..Default::default()
    };
    let sp_pr = xdr::ShapeProperties {
        transform2_d: Some(Box::new(xfrm)),
        shape_properties_choice1: Some(xdr::ShapePropertiesChoice::PresetGeometry(Box::new(prst))),
        ..Default::default()
    };

    let pic = xdr::Picture {
        r#macro: Some(String::new()),
        non_visual_picture_properties: Box::new(nv_pic),
        blip_fill: Box::new(blip_fill),
        shape_properties: Box::new(sp_pr),
        ..Default::default()
    };

    xdr::TwoCellAnchor {
        from_marker: Box::new(from),
        to_marker: Box::new(to),
        two_cell_anchor_choice: Some(xdr::TwoCellAnchorChoice::Picture(Box::new(pic))),
        client_data: Box::new(xdr::ClientData::default()),
        ..Default::default()
    }
}
