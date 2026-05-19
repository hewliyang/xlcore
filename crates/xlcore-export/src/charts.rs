//! Extract drawings (charts) from a worksheet's DrawingsPart.
//!
//! v0 covers clustered/stacked column + bar charts. The data we need lives in
//! the chart part's `numCache`/`strCache` blocks, which Office writes any
//! time it saves the workbook -- so we can render charts without recalc.
//!
//! Pie/line/area/scatter/etc. are recognised but rendered as a placeholder
//! box with title for now.

use crate::charts_ex::extract_chart_ex;
use crate::charts_legacy::extract_chart;
use crate::schema::*;
use base64::Engine;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

/// chartEx graphicData URI (Office 2014+). Distinguishes a `cx:chartSpace`
/// payload from the legacy `c:chartSpace` (which uses the
/// `http://schemas.openxmlformats.org/drawingml/2006/chart` URI).
const CHARTEX_GRAPHIC_DATA_URI: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";

/// What kind of drawing the anchor points at, plus the rId we need to
/// resolve through the drawing-part's relationships.
enum AnchorTarget {
    Chart(String),
    /// chartEx (cx:chartSpace) — Microsoft 2014+ part. Resolved via
    /// `drawings_part.extended_chart_parts()` (different rel type from
    /// legacy charts) and rendered by `extract_chart_ex`.
    ChartEx(String),
    Image(String),
    /// `<xdr:sp>` autoshape or `<xdr:grpSp>` group shape tree.
    /// Walked by `crate::shapes::extract_shape_tree` into a flattened
    /// list of fractional-bbox leaves the renderer can paint.
    Shape(ShapeRoot),
}

/// In-memory handle for the shape branch of an anchor's choice slot.
enum ShapeRoot {
    Sp(std::boxed::Box<xdr::Shape>),
    GrpSp(std::boxed::Box<xdr::GroupShape>),
    CxnSp(std::boxed::Box<xdr::ConnectionShape>),
}

/// Extract drawings (charts + images) from this worksheet's drawingsPart.
///
/// `theme` is the workbook's parsed theme (or `None` to use Office
/// 2007+ defaults). It's needed because chart series colors can be
/// stored as scheme refs (`<a:schemeClr val="accent1"/>`) and Office's
/// auto-cycling defaults map series order to accent1..accent6 — in
/// both cases we want the workbook's actual theme palette.
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
    let anchors_xml: Vec<(DrawingAnchor, AnchorTarget)> = drawing_root
        .worksheet_drawing_choice
        .iter()
        .filter_map(|choice| match choice {
            xdr::WorksheetDrawingChoice::XdrTwoCellAnchor(a) => {
                let from = a.from_marker.as_ref()?;
                let to = a.to_marker.as_ref()?;
                let anchor = DrawingAnchor {
                    from_col: from.column_id as u32,
                    from_col_off_emu: from.column_offset,
                    from_row: from.row_id as u32,
                    from_row_off_emu: from.row_offset,
                    to_col: to.column_id as u32,
                    to_col_off_emu: to.column_offset,
                    to_row: to.row_id as u32,
                    to_row_off_emu: to.row_offset,
                    ext_emu_cx: None,
                    ext_emu_cy: None,
                };
                let target = match a.two_cell_anchor_choice.as_ref()? {
                    xdr::TwoCellAnchorChoice::XdrGraphicFrame(gf) => {
                        let rid = find_relationship_id(&gf.graphic.graphic_data.xml_children)?;
                        if gf.graphic.graphic_data.uri.as_str() == CHARTEX_GRAPHIC_DATA_URI {
                            AnchorTarget::ChartEx(rid)
                        } else {
                            AnchorTarget::Chart(rid)
                        }
                    }
                    xdr::TwoCellAnchorChoice::XdrPic(pic) => {
                        let blip = pic.blip_fill.blip.as_ref()?;
                        let embed = blip.embed.as_ref()?;
                        AnchorTarget::Image(embed.as_str().to_string())
                    }
                    xdr::TwoCellAnchorChoice::XdrSp(sp) => {
                        AnchorTarget::Shape(ShapeRoot::Sp(sp.clone()))
                    }
                    xdr::TwoCellAnchorChoice::XdrGrpSp(g) => {
                        AnchorTarget::Shape(ShapeRoot::GrpSp(g.clone()))
                    }
                    xdr::TwoCellAnchorChoice::XdrCxnSp(c) => {
                        AnchorTarget::Shape(ShapeRoot::CxnSp(c.clone()))
                    }
                    _ => return None,
                };
                Some((anchor, target))
            }
            xdr::WorksheetDrawingChoice::XdrOneCellAnchor(a) => {
                // `oneCellAnchor` pins the upper-left to a cell + offset
                // and sizes the drawing via a fixed EMU extent. We keep
                // the exact extent in `ext_emu_*` for pixel-accurate
                // rendering; the `to_*` fields are filled with a coarse
                // cell-count approximation (default col 64px / row 20px)
                // so grid expansion (minCols/minRows) still reserves
                // roughly enough space for the chart.
                let from = a.from_marker.as_ref()?;
                let ext = a.extent.as_ref()?;
                const EMU_PER_DEFAULT_COL: i64 = 64 * 9525;
                const EMU_PER_DEFAULT_ROW: i64 = 20 * 9525;
                let col_span = ((ext.cx + EMU_PER_DEFAULT_COL - 1) / EMU_PER_DEFAULT_COL).max(1);
                let row_span = ((ext.cy + EMU_PER_DEFAULT_ROW - 1) / EMU_PER_DEFAULT_ROW).max(1);
                let anchor = DrawingAnchor {
                    from_col: from.column_id as u32,
                    from_col_off_emu: from.column_offset,
                    from_row: from.row_id as u32,
                    from_row_off_emu: from.row_offset,
                    to_col: from.column_id as u32 + col_span as u32,
                    to_col_off_emu: 0,
                    to_row: from.row_id as u32 + row_span as u32,
                    to_row_off_emu: 0,
                    ext_emu_cx: Some(ext.cx),
                    ext_emu_cy: Some(ext.cy),
                };
                let target = match a.one_cell_anchor_choice.as_ref()? {
                    xdr::OneCellAnchorChoice::XdrGraphicFrame(gf) => {
                        let rid = find_relationship_id(&gf.graphic.graphic_data.xml_children)?;
                        if gf.graphic.graphic_data.uri.as_str() == CHARTEX_GRAPHIC_DATA_URI {
                            AnchorTarget::ChartEx(rid)
                        } else {
                            AnchorTarget::Chart(rid)
                        }
                    }
                    xdr::OneCellAnchorChoice::XdrPic(pic) => {
                        let blip = pic.blip_fill.blip.as_ref()?;
                        let embed = blip.embed.as_ref()?;
                        AnchorTarget::Image(embed.as_str().to_string())
                    }
                    xdr::OneCellAnchorChoice::XdrSp(sp) => {
                        AnchorTarget::Shape(ShapeRoot::Sp(sp.clone()))
                    }
                    xdr::OneCellAnchorChoice::XdrGrpSp(g) => {
                        AnchorTarget::Shape(ShapeRoot::GrpSp(g.clone()))
                    }
                    xdr::OneCellAnchorChoice::XdrCxnSp(c) => {
                        AnchorTarget::Shape(ShapeRoot::CxnSp(c.clone()))
                    }
                    _ => return None,
                };
                Some((anchor, target))
            }
            _ => None,
        })
        .collect();

    // Build (rid -> Part) maps by scraping each part's Debug repr. The
    // relationship_id field is pub(crate) on these structs so the string
    // scan is the pragmatic path; cheap, no maintenance liability.
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

    // Pre-encode every image part as a `data:` URI keyed by its
    // relationship id. We hand this to the shape extractor so it can
    // surface `<xdr:pic>` nodes nested inside groups (e.g. the
    // screenshot thumbnails inside the Map Chart template's grouped
    // callouts). Top-level pictures still route through
    // `AnchorTarget::Image` and use the per-anchor branch below.
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
    for (anchor, target) in anchors_xml {
        match target {
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
                    anchor,
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
                    anchor,
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
                    anchor,
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
                // ImagePart's CONTENT_TYPE is empty; sniff from the bytes.
                let mime = sniff_image_mime(&bytes).unwrap_or("image/png");
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let data_uri = format!("data:{};base64,{}", mime, b64);
                out.push(Drawing {
                    kind: "image".to_string(),
                    anchor,
                    chart: None,
                    image: Some(Image { data_uri }),
                    shape: None,
                });
            }
        }
    }
    out
}

/// Cheap magic-byte sniff: PNG / JPEG / GIF / BMP / WebP / SVG. Falls back
/// to None and lets the caller default.
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

fn find_relationship_id(children: &[Box<str>]) -> Option<String> {
    for child in children {
        let s: &str = child;
        if let Some(idx) = s.find("r:id=\"") {
            let rest = &s[idx + 6..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}
