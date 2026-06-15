use crate::charts_ex::extract_chart_ex;
use crate::charts_legacy::extract_chart;
use crate::schema::*;
use base64::Engine;
use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
const CHARTEX_GRAPHIC_DATA_URI: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";

enum AnchorTarget {
    Chart(String),
    ChartEx(String),
    Image(String),
    Shape(ShapeRoot),
}

enum ShapeRoot {
    Sp(std::boxed::Box<xdr::Shape>),
    GrpSp(std::boxed::Box<xdr::GroupShape>),
    CxnSp(std::boxed::Box<xdr::ConnectionShape>),
}

struct ParsedAnchor {
    anchor: DrawingAnchor,
    target: AnchorTarget,
    cnv_pr: Option<std::boxed::Box<xdr::NonVisualDrawingProperties>>,
}

pub fn extract(
    doc: &mut SpreadsheetDocument,
    ws_part: &WorksheetPart,
    theme: Option<&Theme>,
) -> Vec<Drawing> {
    let drawings_part = match ws_part.drawings_part(doc) {
        Some(d) => d.clone(),
        None => return Vec::new(),
    };

    let chart_parts: Vec<_> = drawings_part.chart_parts(doc).collect();
    let extended_chart_parts: Vec<_> = drawings_part.extended_chart_parts(doc).collect();
    let image_parts: Vec<_> = drawings_part.image_parts(doc).collect();

    let drawing_root = match drawings_part.root_element(doc) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let anchors_xml: Vec<ParsedAnchor> = drawing_root
        .worksheet_drawing_choice
        .iter()
        .filter_map(parse_worksheet_drawing_choice)
        .collect();

    let mut chart_by_rid: Vec<(String, ooxmlsdk::parts::chart_part::ChartPart)> = chart_parts
        .into_iter()
        .filter_map(|p| Some((part_relationship_id_dbg(&p)?, p)))
        .collect();
    let mut chart_ex_by_rid: Vec<(
        String,
        ooxmlsdk::parts::extended_chart_part::ExtendedChartPart,
    )> = extended_chart_parts
        .into_iter()
        .filter_map(|p| Some((part_relationship_id_dbg(&p)?, p)))
        .collect();
    let image_by_rid: Vec<(String, ooxmlsdk::parts::image_part::ImagePart)> = image_parts
        .into_iter()
        .filter_map(|p| Some((part_relationship_id_dbg(&p)?, p)))
        .collect();

    let image_uri_by_rid: std::collections::HashMap<String, String> = image_by_rid
        .iter()
        .filter_map(|(rid, ip)| {
            let bytes = ip.data(doc)?.to_vec();
            let mime = sniff_image_mime(&bytes).unwrap_or("image/png");
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Some((rid.clone(), format!("data:{};base64,{}", mime, b64)))
        })
        .collect();

    let mut out = Vec::new();
    for parsed in anchors_xml {
        let hyperlink = parsed
            .cnv_pr
            .as_ref()
            .and_then(|cnv| drawing_hyperlink_from_cnvpr(doc, &drawings_part, cnv));

        match parsed.target {
            AnchorTarget::Chart(rid) => {
                let pos = match chart_by_rid.iter().position(|(r, _)| r == &rid) {
                    Some(i) => i,
                    None => continue,
                };
                let (_, cp) = chart_by_rid.remove(pos);
                let space = match cp.root_element(doc) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let chart = extract_chart(space, theme);
                out.push(Drawing {
                    kind: "chart".to_string(),
                    anchor: parsed.anchor,
                    hyperlink,
                    chart,
                    image: None,
                    shape: None,
                });
            }
            AnchorTarget::ChartEx(rid) => {
                let pos = match chart_ex_by_rid.iter().position(|(r, _)| r == &rid) {
                    Some(i) => i,
                    None => continue,
                };
                let (_, cp) = chart_ex_by_rid.remove(pos);
                let space = match cp.root_element(doc) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let chart = extract_chart_ex(space, theme);
                out.push(Drawing {
                    kind: "chart".to_string(),
                    anchor: parsed.anchor,
                    hyperlink,
                    chart,
                    image: None,
                    shape: None,
                });
            }
            AnchorTarget::Shape(root) => {
                let resolver = |rid: &str| image_uri_by_rid.get(rid).cloned();
                let shape_opt = match &root {
                    ShapeRoot::Sp(s) => crate::shapes::extract_shape_tree(
                        crate::shapes::ShapeTreeRoot::Sp(s.as_ref()),
                        theme,
                        &resolver,
                    ),
                    ShapeRoot::GrpSp(g) => crate::shapes::extract_shape_tree(
                        crate::shapes::ShapeTreeRoot::GrpSp(g.as_ref()),
                        theme,
                        &resolver,
                    ),
                    ShapeRoot::CxnSp(c) => crate::shapes::extract_shape_tree(
                        crate::shapes::ShapeTreeRoot::CxnSp(c.as_ref()),
                        theme,
                        &resolver,
                    ),
                };
                let Some(shape) = shape_opt else { continue };
                out.push(Drawing {
                    kind: "shape".to_string(),
                    anchor: parsed.anchor,
                    hyperlink,
                    chart: None,
                    image: None,
                    shape: Some(shape),
                });
            }
            AnchorTarget::Image(rid) => {
                let pos = match image_by_rid.iter().position(|(r, _)| r == &rid) {
                    Some(i) => i,
                    None => continue,
                };
                let ip = &image_by_rid[pos].1;
                let bytes = match ip.data(doc) {
                    Some(b) => b.to_vec(),
                    None => continue,
                };

                let mime = sniff_image_mime(&bytes).unwrap_or("image/png");
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let data_uri = format!("data:{};base64,{}", mime, b64);
                out.push(Drawing {
                    kind: "image".to_string(),
                    anchor: parsed.anchor,
                    hyperlink,
                    chart: None,
                    image: Some(Image { data_uri }),
                    shape: None,
                });
            }
        }
    }
    out
}

fn parse_worksheet_drawing_choice(choice: &xdr::WorksheetDrawingChoice) -> Option<ParsedAnchor> {
    match choice {
        xdr::WorksheetDrawingChoice::TwoCellAnchor(a) => {
            let from = &a.from_marker;
            let to = &a.to_marker;
            let anchor = DrawingAnchor {
                anchor_kind: Some("twoCell".to_string()),
                edit_as: a.edit_as.map(edit_as_token),
                from_col: from.column_id as u32,
                from_col_off_emu: from.column_offset.to_emu(),
                from_row: from.row_id as u32,
                from_row_off_emu: from.row_offset.to_emu(),
                to_col: to.column_id as u32,
                to_col_off_emu: to.column_offset.to_emu(),
                to_row: to.row_id as u32,
                to_row_off_emu: to.row_offset.to_emu(),
                ext_emu_cx: None,
                ext_emu_cy: None,
            };
            let (target, cnv_pr) = anchor_target_from_two_cell(a.two_cell_anchor_choice.as_ref()?)?;
            Some(ParsedAnchor {
                anchor,
                target,
                cnv_pr,
            })
        }
        xdr::WorksheetDrawingChoice::OneCellAnchor(a) => {
            let from = &a.from_marker;
            let ext = &a.extent;
            const EMU_PER_DEFAULT_COL: i64 = 64 * 9525;
            const EMU_PER_DEFAULT_ROW: i64 = 20 * 9525;
            let col_span = ((ext.cx + EMU_PER_DEFAULT_COL - 1) / EMU_PER_DEFAULT_COL).max(1);
            let row_span = ((ext.cy + EMU_PER_DEFAULT_ROW - 1) / EMU_PER_DEFAULT_ROW).max(1);
            let anchor = DrawingAnchor {
                anchor_kind: Some("oneCell".to_string()),
                edit_as: None,
                from_col: from.column_id as u32,
                from_col_off_emu: from.column_offset.to_emu(),
                from_row: from.row_id as u32,
                from_row_off_emu: from.row_offset.to_emu(),
                to_col: from.column_id as u32 + col_span as u32,
                to_col_off_emu: 0,
                to_row: from.row_id as u32 + row_span as u32,
                to_row_off_emu: 0,
                ext_emu_cx: Some(ext.cx),
                ext_emu_cy: Some(ext.cy),
            };
            let (target, cnv_pr) = anchor_target_from_one_cell(a.one_cell_anchor_choice.as_ref()?)?;
            Some(ParsedAnchor {
                anchor,
                target,
                cnv_pr,
            })
        }
        xdr::WorksheetDrawingChoice::AbsoluteAnchor(a) => {
            let pos = &a.position;
            let ext = &a.extent;
            let anchor = DrawingAnchor {
                anchor_kind: Some("absolute".to_string()),
                edit_as: None,
                from_col: 0,
                from_col_off_emu: pos.x,
                from_row: 0,
                from_row_off_emu: pos.y,
                to_col: 0,
                to_col_off_emu: 0,
                to_row: 0,
                to_row_off_emu: 0,
                ext_emu_cx: Some(ext.cx),
                ext_emu_cy: Some(ext.cy),
            };
            let (target, cnv_pr) = anchor_target_from_absolute(a.absolute_anchor_choice.as_ref()?)?;
            Some(ParsedAnchor {
                anchor,
                target,
                cnv_pr,
            })
        }
        xdr::WorksheetDrawingChoice::XmlAny(_) => None,
    }
}

fn edit_as_token(v: xdr::EditAsValues) -> String {
    match v {
        xdr::EditAsValues::TwoCell => "twoCell",
        xdr::EditAsValues::OneCell => "oneCell",
        xdr::EditAsValues::Absolute => "absolute",
    }
    .to_string()
}

fn anchor_target_from_two_cell(
    choice: &xdr::TwoCellAnchorChoice,
) -> Option<(
    AnchorTarget,
    Option<std::boxed::Box<xdr::NonVisualDrawingProperties>>,
)> {
    match choice {
        xdr::TwoCellAnchorChoice::GraphicFrame(gf) => {
            let rid = find_relationship_id(&gf.graphic.graphic_data.graphic_data_choice)?;
            if gf.graphic.graphic_data.uri.as_str() == CHARTEX_GRAPHIC_DATA_URI {
                Some((AnchorTarget::ChartEx(rid), None))
            } else {
                Some((AnchorTarget::Chart(rid), None))
            }
        }
        xdr::TwoCellAnchorChoice::Picture(pic) => {
            let blip = pic.blip_fill.as_ref()?.blip.as_ref()?;
            let embed = blip.embed.as_ref()?;
            let cnv = pic
                .non_visual_picture_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Image(embed.as_str().to_string()), Some(cnv)))
        }
        xdr::TwoCellAnchorChoice::Shape(sp) => {
            let cnv = sp
                .non_visual_shape_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Shape(ShapeRoot::Sp(sp.clone())), Some(cnv)))
        }
        xdr::TwoCellAnchorChoice::GroupShape(g) => {
            let cnv = Some(
                g.non_visual_group_shape_properties
                    .non_visual_drawing_properties
                    .clone(),
            );
            Some((AnchorTarget::Shape(ShapeRoot::GrpSp(g.clone())), cnv))
        }
        xdr::TwoCellAnchorChoice::ConnectionShape(c) => {
            let cnv = c
                .non_visual_connection_shape_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Shape(ShapeRoot::CxnSp(c.clone())), Some(cnv)))
        }
        _ => None,
    }
}

fn anchor_target_from_one_cell(
    choice: &xdr::OneCellAnchorChoice,
) -> Option<(
    AnchorTarget,
    Option<std::boxed::Box<xdr::NonVisualDrawingProperties>>,
)> {
    match choice {
        xdr::OneCellAnchorChoice::GraphicFrame(gf) => {
            let rid = find_relationship_id(&gf.graphic.graphic_data.graphic_data_choice)?;
            if gf.graphic.graphic_data.uri.as_str() == CHARTEX_GRAPHIC_DATA_URI {
                Some((AnchorTarget::ChartEx(rid), None))
            } else {
                Some((AnchorTarget::Chart(rid), None))
            }
        }
        xdr::OneCellAnchorChoice::Picture(pic) => {
            let blip = pic.blip_fill.as_ref()?.blip.as_ref()?;
            let embed = blip.embed.as_ref()?;
            let cnv = pic
                .non_visual_picture_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Image(embed.as_str().to_string()), Some(cnv)))
        }
        xdr::OneCellAnchorChoice::Shape(sp) => {
            let cnv = sp
                .non_visual_shape_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Shape(ShapeRoot::Sp(sp.clone())), Some(cnv)))
        }
        xdr::OneCellAnchorChoice::GroupShape(g) => {
            let cnv = Some(
                g.non_visual_group_shape_properties
                    .non_visual_drawing_properties
                    .clone(),
            );
            Some((AnchorTarget::Shape(ShapeRoot::GrpSp(g.clone())), cnv))
        }
        xdr::OneCellAnchorChoice::ConnectionShape(c) => {
            let cnv = c
                .non_visual_connection_shape_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Shape(ShapeRoot::CxnSp(c.clone())), Some(cnv)))
        }
        _ => None,
    }
}

fn anchor_target_from_absolute(
    choice: &xdr::AbsoluteAnchorChoice,
) -> Option<(
    AnchorTarget,
    Option<std::boxed::Box<xdr::NonVisualDrawingProperties>>,
)> {
    match choice {
        xdr::AbsoluteAnchorChoice::GraphicFrame(gf) => {
            let rid = find_relationship_id(&gf.graphic.graphic_data.graphic_data_choice)?;
            if gf.graphic.graphic_data.uri.as_str() == CHARTEX_GRAPHIC_DATA_URI {
                Some((AnchorTarget::ChartEx(rid), None))
            } else {
                Some((AnchorTarget::Chart(rid), None))
            }
        }
        xdr::AbsoluteAnchorChoice::Picture(pic) => {
            let blip = pic.blip_fill.as_ref()?.blip.as_ref()?;
            let embed = blip.embed.as_ref()?;
            let cnv = pic
                .non_visual_picture_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Image(embed.as_str().to_string()), Some(cnv)))
        }
        xdr::AbsoluteAnchorChoice::Shape(sp) => {
            let cnv = sp
                .non_visual_shape_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Shape(ShapeRoot::Sp(sp.clone())), Some(cnv)))
        }
        xdr::AbsoluteAnchorChoice::GroupShape(g) => {
            let cnv = Some(
                g.non_visual_group_shape_properties
                    .non_visual_drawing_properties
                    .clone(),
            );
            Some((AnchorTarget::Shape(ShapeRoot::GrpSp(g.clone())), cnv))
        }
        xdr::AbsoluteAnchorChoice::ConnectionShape(c) => {
            let cnv = c
                .non_visual_connection_shape_properties
                .non_visual_drawing_properties
                .clone();
            Some((AnchorTarget::Shape(ShapeRoot::CxnSp(c.clone())), Some(cnv)))
        }
        _ => None,
    }
}

fn drawing_hyperlink_from_cnvpr(
    doc: &mut SpreadsheetDocument,
    drawings_part: &DrawingsPart,
    cnv: &xdr::NonVisualDrawingProperties,
) -> Option<DrawingHyperlink> {
    let click = cnv.hyperlink_on_click.as_ref()?;
    let rid = click.id.as_ref()?.as_str();
    let rel = drawings_part.get_hyperlink_relationship(doc, rid)?;
    let mut target = rel.target().to_string();
    if let Some(url) = click.invalid_url.as_ref() {
        target = url.as_str().to_string();
    }
    let location = location_from_hlink_click(click, &target);
    let target = normalize_drawing_hlink_target(&target, location.as_deref());
    Some(DrawingHyperlink {
        target: Some(target),
        location,
        tooltip: click.tooltip.as_ref().map(|t| t.as_str().to_string()),
        display: None,
    })
}

fn location_from_hlink_click(
    click: &ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::HyperlinkOnClick,
    target: &str,
) -> Option<String> {
    if let Some(action) = click.action.as_ref() {
        let action = action.as_str();
        if action.starts_with("ppaction://") {
            return None;
        }
        if !action.is_empty() {
            return Some(action.to_string());
        }
    }
    if let Some(hash) = target.strip_prefix('#') {
        return Some(hash.to_string());
    }
    None
}

fn normalize_drawing_hlink_target(target: &str, location: Option<&str>) -> String {
    if target.starts_with('#') {
        return target.to_string();
    }
    if let Some(loc) = location.filter(|s| !s.is_empty()) {
        if target.is_empty() || !target.contains("://") {
            return format!("#{loc}");
        }
    }
    if !target.is_empty() && !target.contains("://") && target.contains('!') {
        return format!("#{target}");
    }
    target.to_string()
}

fn sniff_image_mime(b: &[u8]) -> Option<&'static str> {
    if b.len() >= 8 && b[..8] == [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some("image/png");
    }
    if b.len() >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF {
        return Some("image/jpeg");
    }
    if b.len() >= 6 && (&b[..6] == b"GIF87a" || &b[..6] == b"GIF89a") {
        return Some("image/gif");
    }
    if b.len() >= 2 && &b[..2] == b"BM" {
        return Some("image/bmp");
    }
    if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Some("image/webp");
    }
    if b.len() >= 5 && (b.starts_with(b"<svg") || b.starts_with(b"<?xml")) {
        return Some("image/svg+xml");
    }
    None
}

fn part_relationship_id_dbg<T: std::fmt::Debug>(p: &T) -> Option<String> {
    let dbg = format!("{:?}", p);
    let key = "relationship_id: Some(\"";
    let idx = dbg.find(key)?;
    let rest = &dbg[idx + key.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn find_relationship_id(
    choices: &[ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::GraphicDataChoice],
) -> Option<String> {
    use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::GraphicDataChoice as G;
    for c in choices {
        match c {
            G::ChartReference(r) => return Some(r.id.as_str().to_string()),
            G::XmlAny(s) => {
                let raw = String::from_utf8_lossy(s);
                if let Some(idx) = raw.find("r:id=\"") {
                    let rest = &raw[idx + 6..];
                    if let Some(end) = rest.find('"') {
                        return Some(rest[..end].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_as_token_maps_spec_values() {
        assert_eq!(edit_as_token(xdr::EditAsValues::TwoCell), "twoCell");
        assert_eq!(edit_as_token(xdr::EditAsValues::OneCell), "oneCell");
        assert_eq!(edit_as_token(xdr::EditAsValues::Absolute), "absolute");
    }

    #[test]
    fn extract_absolute_anchor_and_shape_hyperlink_fixtures() {
        let abs_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/shapes/absolute-anchor.xlsx"
        );
        let mut doc = xlcore_io::open(abs_path).expect("open absolute-anchor fixture");
        let layout = crate::extract_doc(&mut doc).expect("extract");
        let drawings = &layout.sheets[0].drawings;
        let absolute = drawings
            .iter()
            .find(|d| d.anchor.anchor_kind.as_deref() == Some("absolute"))
            .expect("absoluteAnchor drawing");
        assert_eq!(absolute.anchor.from_col_off_emu, 304800);
        assert_eq!(absolute.anchor.ext_emu_cx, Some(1524000));

        let hlink_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/shapes/shape-hyperlinks.xlsx"
        );
        let mut doc = xlcore_io::open(hlink_path).expect("open shape-hyperlinks fixture");
        let layout = crate::extract_doc(&mut doc).expect("extract");
        let drawings = &layout.sheets[0].drawings;
        let external = drawings
            .iter()
            .find(|d| {
                d.hyperlink.as_ref().and_then(|h| h.target.as_deref())
                    == Some("https://example.com/shape-external")
            })
            .expect("external shape hyperlink");
        assert_eq!(
            external
                .hyperlink
                .as_ref()
                .and_then(|h| h.tooltip.as_deref()),
            Some("External shape link")
        );
        let internal = drawings
            .iter()
            .find(|d| d.hyperlink.as_ref().and_then(|h| h.target.as_deref()) == Some("#Sheet1!B5"))
            .expect("internal shape hyperlink");
        assert!(internal.hyperlink.is_some());
        assert!(drawings
            .iter()
            .any(|d| d.hyperlink.is_none() && d.shape.is_some()));
    }

    #[test]
    fn normalize_drawing_hlink_target_prefixes_internal_sheet_refs() {
        assert_eq!(
            normalize_drawing_hlink_target("Sheet2!A1", Some("Sheet2!A1")),
            "#Sheet2!A1"
        );
        assert_eq!(
            normalize_drawing_hlink_target("Sheet1!B5", None),
            "#Sheet1!B5"
        );
        assert_eq!(
            normalize_drawing_hlink_target("https://example.com", None),
            "https://example.com"
        );
    }
}
