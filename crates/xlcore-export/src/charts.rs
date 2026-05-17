//! Extract drawings (charts) from a worksheet's DrawingsPart.
//!
//! v0 covers clustered/stacked column + bar charts. The data we need lives in
//! the chart part's `numCache`/`strCache` blocks, which Office writes any
//! time it saves the workbook -- so we can render charts without recalc.
//!
//! Pie/line/area/scatter/etc. are recognised but rendered as a placeholder
//! box with title for now.

use crate::chart_colors::*;
use crate::charts_helpers::*;

use crate::schema::*;
use base64::Engine;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_microsoft_com_office_drawing_2014_chartex as cx;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

/// chartEx graphicData URI (Office 2014+). Distinguishes a `cx:chartSpace`
/// payload from the legacy `c:chartSpace` (which uses the
/// `http://schemas.openxmlformats.org/drawingml/2006/chart` URI).
const CHARTEX_GRAPHIC_DATA_URI: &str =
    "http://schemas.microsoft.com/office/drawing/2014/chartex";

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
                let chart = extract_chart_ex(space);
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

fn extract_chart(space: &c::ChartSpace, theme: Option<&Theme>) -> Option<Chart> {
    let chart = space.c_chart.as_ref()?;
    let plot_area = &chart.plot_area;

    // Pre-scan plot_area_choice2 for value axes. We need to know which
    // axId belongs to the primary (left/bottom) vs secondary (right/top)
    // value axis so that, per chart-type group below, we can tag its
    // series with axis_group = primary/secondary. We also stash the
    // secondary numFmt to expose as Chart.value_format_secondary.
    let mut secondary_ax_ids: Vec<u32> = Vec::new();
    let mut primary_ax_ids: Vec<u32> = Vec::new();
    let mut primary_val_fmt: Option<String> = None;
    let mut secondary_val_fmt: Option<String> = None;
    let mut value_min: Option<f64> = None;
    let mut value_max: Option<f64> = None;
    let mut value_min_secondary: Option<f64> = None;
    let mut value_max_secondary: Option<f64> = None;
    // Axis titles. ECMA-376 §21.2.2.213 — every axis CT carries an
    // optional `<c:title>` (same `CT_Title` shape as the chart title).
    // We route by `axPos`: `b`/`t` → x-axis (catAx/dateAx), `l` →
    // y-axis, `r` → secondary y-axis.
    let mut x_axis_title: Option<String> = None;
    let mut y_axis_title: Option<String> = None;
    let mut y_axis_title_secondary: Option<String> = None;
    // `<c:majorGridlines>` toggle per value axis. ECMA-376 §21.2.2.85:
    // gridlines paint iff the element is present, and `<a:noFill/>` on
    // its line suppresses the stroke even when present. None ⇒ the
    // value axis is absent on this side; we collapse that to "don't
    // paint" at the renderer.
    let mut show_major_gridlines: Option<bool> = None;
    let mut show_major_gridlines_secondary: Option<bool> = None;
    // `<c:dispUnits>` per value axis. ECMA-376 §21.2.2.46:
    // tick labels on the axis are divided by `disp_units` before
    // formatting, and `disp_units_label` (if present) is painted near
    // the axis as a caption (e.g. "S$ mn" with `builtInUnit=thousands`).
    let mut disp_units: Option<f64> = None;
    let mut disp_units_label: Option<String> = None;
    let mut disp_units_secondary: Option<f64> = None;
    let mut disp_units_label_secondary: Option<String> = None;
    // `<c:majorUnit val="N"/>` per value axis (ECMA-376 §21.2.2.121).
    // When authored, the renderer steps ticks by exactly N source
    // units instead of niceTicks; lets workbooks pin cadences like
    // 9000 (NWC line chart, dispUnits=thousands → 0/9/18/27/36/45).
    let mut major_unit: Option<f64> = None;
    let mut major_unit_secondary: Option<f64> = None;
    // Route an axis's title to x / y / y-secondary by its `axPos`.
    // ECMA-376: `b`/`t` → horizontal, `l` → vertical, `r` → secondary
    // vertical. This is intentionally axis-*position*-driven (not
    // axis-*type*-driven) so horizontal bar charts — where the
    // catAx is at `l` and the valAx is at `b` — land their titles
    // on the correct edge.
    let route_title = |pos: Option<&c::AxisPositionValues>,
                       title: Option<&c::Title>,
                       x: &mut Option<String>,
                       y: &mut Option<String>,
                       y2: &mut Option<String>| {
        let Some(t) = extract_title(title) else {
            return;
        };
        match pos {
            Some(c::AxisPositionValues::Bottom) | Some(c::AxisPositionValues::Top) => {
                if x.is_none() {
                    *x = Some(t);
                }
            }
            Some(c::AxisPositionValues::Right) => {
                if y2.is_none() {
                    *y2 = Some(t);
                }
            }
            // Default / `Left` / unknown → primary y.
            _ => {
                if y.is_none() {
                    *y = Some(t);
                }
            }
        }
    };
    for choice in &plot_area.plot_area_choice2 {
        match choice {
            c::PlotAreaChoice2::CCatAx(ca) => route_title(
                ca.axis_position.as_ref().map(|p| &p.val),
                ca.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
            ),
            c::PlotAreaChoice2::CDateAx(da) => route_title(
                da.axis_position.as_ref().map(|p| &p.val),
                da.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
            ),
            _ => {}
        }
        if let c::PlotAreaChoice2::CValAx(va) = choice {
            let axid = va.axis_id.as_ref().map(|a| a.val).unwrap_or(0);
            let pos = va.axis_position.as_ref().map(|p| &p.val);
            let is_secondary = matches!(
                pos,
                Some(c::AxisPositionValues::Right) | Some(c::AxisPositionValues::Top)
            );
            // Scatter charts emit TWO `<c:valAx>` blocks — the numeric
            // x-axis at `axPos="b"` and the y-axis at `axPos="l"`. Only
            // the latter is the conceptual "primary value axis" whose
            // gridlines/format/scaling we want; the x-axis valAx should
            // be ignored for those concerns. (For bar/line/area charts
            // the catAx sits at b/t and there's only one valAx at l/r,
            // so this filter is a no-op.)
            let is_horizontal_value_axis =
                matches!(pos, Some(c::AxisPositionValues::Bottom)) && !is_secondary;
            // Major gridlines: present ∧ line not `<a:noFill/>`.
            // The MajorGridlines element only carries an optional
            // `<c:spPr>`; we look at its Debug repr for an `ANoFill`
            // token inside the line block (same pragma as the series
            // color resolver). When spPr is absent the default stroke
            // applies, so "present without noFill" ⇒ show. Element
            // entirely absent ⇒ don't paint (ECMA-376 §21.2.2.85).
            //
            // We deliberately always emit `Some(bool)` here instead of
            // mapping through `.map()` — the schema's `Option<bool>` is
            // for "no value axis exists at all" (pie/doughnut), not for
            // "value axis exists but its `<c:majorGridlines>` element is
            // absent". A None on the wire would let the renderer's
            // `!== false` back-compat fallback paint gridlines that
            // weren't authored.
            let gridlines_on = Some(va.major_gridlines.as_ref().is_some_and(|mg| {
                match mg.chart_shape_properties.as_deref() {
                    None => true,
                    Some(sp) => !line_has_no_fill(sp),
                }
            }));
            let fmt = va
                .numbering_format
                .as_ref()
                .map(|nf| nf.format_code.as_str().to_string());
            // `<c:scaling><c:min>` / `<c:max>`: explicit axis bounds.
            // Either may be absent (Excel auto-picks); both come
            // through as `f64` so just forward to schema.
            let scaling_min = va
                .scaling
                .as_ref()
                .and_then(|s| s.min_axis_value.as_ref())
                .map(|m| m.val);
            let scaling_max = va
                .scaling
                .as_ref()
                .and_then(|s| s.max_axis_value.as_ref())
                .map(|m| m.val);
            // `<c:majorUnit>` sits *outside* `<c:scaling>` directly on
            // the valAx (per ECMA-376 §21.2.2.120) — not nested like
            // min/max. Positive-finite values only; OOXML allows any
            // positive double but niceTicks already handles boundary
            // weirdness so we re-validate here for safety.
            let axis_major_unit = va
                .c_major_unit
                .as_ref()
                .map(|m| m.val)
                .filter(|v| v.is_finite() && *v > 0.0);
            if is_secondary {
                secondary_ax_ids.push(axid);
                if secondary_val_fmt.is_none() {
                    secondary_val_fmt = fmt;
                }
                if value_min_secondary.is_none() {
                    value_min_secondary = scaling_min;
                }
                if value_max_secondary.is_none() {
                    value_max_secondary = scaling_max;
                }
                if show_major_gridlines_secondary.is_none() {
                    show_major_gridlines_secondary = gridlines_on;
                }
                if disp_units_secondary.is_none() {
                    if let Some((f, lbl)) = extract_disp_units(va.c_disp_units.as_deref()) {
                        disp_units_secondary = Some(f);
                        disp_units_label_secondary = lbl;
                    }
                }
                if major_unit_secondary.is_none() {
                    major_unit_secondary = axis_major_unit;
                }
            } else if is_horizontal_value_axis {
                // Scatter x-axis: contributes its axId to primary_ax_ids
                // (so series axis-group resolution still works) but
                // not its gridlines / numFmt / scaling — those belong
                // to the y-axis.
                primary_ax_ids.push(axid);
            } else {
                primary_ax_ids.push(axid);
                if primary_val_fmt.is_none() {
                    primary_val_fmt = fmt;
                }
                if value_min.is_none() {
                    value_min = scaling_min;
                }
                if value_max.is_none() {
                    value_max = scaling_max;
                }
                if show_major_gridlines.is_none() {
                    show_major_gridlines = gridlines_on;
                }
                if disp_units.is_none() {
                    if let Some((f, lbl)) = extract_disp_units(va.c_disp_units.as_deref()) {
                        disp_units = Some(f);
                        disp_units_label = lbl;
                    }
                }
                if major_unit.is_none() {
                    major_unit = axis_major_unit;
                }
            }
            route_title(
                va.axis_position.as_ref().map(|p| &p.val),
                va.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
            );
        }
    }

    // Edge case: no axPos on either side. Treat first valAx encountered
    // as primary so single-axis charts still render.
    let secondary_axis = !secondary_ax_ids.is_empty() && !primary_ax_ids.is_empty();

    /// Resolve a chart-type group's axIds (a Vec<AxisId> with 2 entries:
    /// one cat-axis ref, one val-axis ref) to "primary" / "secondary".
    /// Defaults to primary when neither axId matches a known valAx —
    /// the safest fallback for malformed/legacy files.
    fn axis_group_for(ax_ids: &[c::AxisId], sec: &[u32]) -> Option<String> {
        if sec.is_empty() {
            return None;
        }
        for a in ax_ids {
            if sec.contains(&a.val) {
                return Some("secondary".to_string());
            }
        }
        Some("primary".to_string())
    }

    let mut bar_dir: Option<String> = None;
    let mut scatter_style: Option<String> = None;
    let mut radar_style: Option<String> = None;
    let mut bubble_scale: Option<u32> = None;
    let mut size_represents: Option<String> = None;
    let mut grouping: Option<String> = None;
    // ECMA-376 §21.2.2.75 / §21.2.2.108. Captured from the first
    // `<c:barChart>` group encountered (a chart can technically host
    // multiple bar groups but Excel writes one); combo charts get the
    // bar-side values, line/area groups don't carry these.
    let mut bar_gap_width: Option<u16> = None;
    let mut bar_overlap: Option<i8> = None;
    // Stock-chart decoration toggles. ECMA-376 §21.2.2.207 lets
    // `<c:stockChart>` host `<c:hiLowLines/>`, `<c:upDownBars/>`,
    // `<c:dropLines/>` as optional children.
    let mut stock_hi_low_lines = false;
    let mut stock_up_down_bars = false;
    let mut stock_drop_lines = false;
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut categories: Vec<String> = Vec::new();
    let mut _categories_ref: Option<String> = None;
    let mut categories_format: Option<String> = None;
    let mut value_format: Option<String> = None;
    let mut chart_data_labels: Option<DataLabels> = None;

    // Track every chart-type tag we encounter so we can emit `combo`
    // when more than one is present in the same plotArea.
    let mut group_types: Vec<&'static str> = Vec::new();

    // Helper macro: extract a chart-type group's series, tagging each
    // with `axis_group` (when the chart has a secondary axis) and
    // `chart_type` (per-series override, for combo rendering). The
    // chart-level categories/value_format are taken from the first
    // primary-axis group we see; combo charts otherwise inherit the
    // primary scale.
    macro_rules! extract_chartlike {
        ($coll:expr, $kind:expr, $ax_ids:expr, $is_primary_group:expr) => {{
            let ag = axis_group_for($ax_ids, &secondary_ax_ids);
            for ser in $coll {
                let mut row = common_series(
                    &ser.order,
                    ser.series_text.as_deref(),
                    ser.chart_shape_properties.as_deref(),
                    ser.c_val.as_deref(),
                    &ser.c_d_pt,
                    theme,
                );
                row.data_labels = extract_data_labels(ser.c_d_lbls.as_deref());
                row.axis_group = ag.clone();
                row.chart_type = Some($kind.to_string());
                series.push(row);
                if $is_primary_group && categories.is_empty() {
                    let (cs, r, fmt) = ax_data_values(ser.c_cat.as_deref());
                    categories = cs;
                    _categories_ref = r;
                    categories_format = fmt;
                }
                if $is_primary_group && value_format.is_none() {
                    value_format = values_format(ser.c_val.as_deref());
                }
            }
        }};
    }

    for choice in &plot_area.plot_area_choice1 {
        match choice {
            c::PlotAreaChoice::CBarChart(bc) => {
                let kind = match bc.bar_direction.val {
                    c::BarDirectionValues::Column => "column",
                    c::BarDirectionValues::Bar => "bar",
                };
                if bar_dir.is_none() {
                    bar_dir = Some(format!("{:?}", bc.bar_direction.val).to_ascii_lowercase());
                }
                if grouping.is_none() {
                    if let Some(g) = &bc.bar_grouping {
                        grouping = g
                            .val
                            .as_ref()
                            .map(|v| format!("{:?}", v).to_ascii_lowercase());
                    }
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                }
                if bar_gap_width.is_none() {
                    bar_gap_width = bc.c_gap_width.as_ref().and_then(|g| g.val);
                }
                if bar_overlap.is_none() {
                    bar_overlap = bc.c_overlap.as_ref().and_then(|o| o.val);
                }
                let ag = axis_group_for(&bc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&bc.c_ser, kind, &bc.c_ax_id, is_primary);
                group_types.push(kind);
            }
            c::PlotAreaChoice::CLineChart(lc) => {
                if grouping.is_none() {
                    grouping = lc
                        .grouping
                        .val
                        .as_ref()
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(lc.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&lc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&lc.c_ser, "line", &lc.c_ax_id, is_primary);
                // Propagate per-series `<c:marker><c:symbol val="..."/>`
                // (LineChartSeries is the only series shape with a top-
                // level `marker` field, so this can't go in the macro).
                for (offset, ser) in lc.c_ser.iter().enumerate() {
                    let sym = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.marker_symbol = sym;
                    }
                }
                group_types.push("line");
            }
            c::PlotAreaChoice::CAreaChart(ac) => {
                if grouping.is_none() {
                    grouping = ac
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(ac.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&ac.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&ac.c_ser, "area", &ac.c_ax_id, is_primary);
                group_types.push("area");
            }
            c::PlotAreaChoice::CPieChart(pc) => {
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                // Pie has no axes in OOXML; pass an empty slice.
                extract_chartlike!(&pc.c_ser, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::CDoughnutChart(dc) => {
                chart_data_labels = extract_data_labels(dc.c_d_lbls.as_deref());
                extract_chartlike!(&dc.c_ser, "doughnut", &[] as &[c::AxisId], true);
                group_types.push("doughnut");
                break;
            }
            c::PlotAreaChoice::CScatterChart(sc) => {
                chart_data_labels = extract_data_labels(sc.c_d_lbls.as_deref());
                // ECMA-376 §21.2.2.193: ScatterStyle val is required
                // (default `line`). Excel's *UI* default for new scatter
                // charts is `marker`, but a workbook that explicitly
                // wrote `<c:scatterStyle val="lineMarker"/>` etc. should
                // round-trip the requested style.
                scatter_style = sc.scatter_style.val.as_ref().map(|v| {
                    match v {
                        c::ScatterStyleValues::Line => "line",
                        c::ScatterStyleValues::LineMarker => "lineMarker",
                        c::ScatterStyleValues::Marker => "marker",
                        c::ScatterStyleValues::Smooth => "smooth",
                        c::ScatterStyleValues::SmoothMarker => "smoothMarker",
                    }
                    .to_string()
                });
                for ser in &sc.c_ser {
                    let mut row = common_series_scatter(
                        &ser.order,
                        ser.series_text.as_deref(),
                        ser.chart_shape_properties.as_deref(),
                        ser.c_y_val.as_deref(),
                        &ser.c_d_pt,
                        theme,
                    );
                    let (xs, xref) = scatter_x_values(ser.c_x_val.as_deref());
                    row.x_values = xs;
                    row.x_values_ref = xref;
                    row.data_labels = extract_data_labels(ser.c_d_lbls.as_deref());
                    row.axis_group = axis_group_for(&sc.c_ax_id, &secondary_ax_ids);
                    row.chart_type = Some("scatter".to_string());
                    row.marker_symbol = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    series.push(row);
                    if categories.is_empty() {
                        // Stash the x-axis ref for axis labels (numeric).
                        let (cs, r, fmt) = x_axis_values(ser.c_x_val.as_deref());
                        categories = cs;
                        _categories_ref = r;
                        categories_format = fmt;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.c_y_val.as_deref());
                    }
                }
                group_types.push("scatter");
                break;
            }
            c::PlotAreaChoice::CBar3DChart(bc) => {
                // Legacy 3D variant: dispatch to the 2D bar painter.
                // 3D-only flourishes (gap_depth, shape, perspective)
                // are intentionally dropped — Excel's 3D chart visuals
                // are out of scope for v0; the data + 2D layout match
                // ECMA-376 well enough for HITL preview.
                let kind = match bc.bar_direction.val {
                    c::BarDirectionValues::Column => "column",
                    c::BarDirectionValues::Bar => "bar",
                };
                if bar_dir.is_none() {
                    bar_dir = Some(format!("{:?}", bc.bar_direction.val).to_ascii_lowercase());
                }
                if grouping.is_none() {
                    if let Some(g) = &bc.bar_grouping {
                        grouping = g
                            .val
                            .as_ref()
                            .map(|v| format!("{:?}", v).to_ascii_lowercase());
                    }
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                }
                if bar_gap_width.is_none() {
                    bar_gap_width = bc.c_gap_width.as_ref().and_then(|g| g.val);
                }
                let ag = axis_group_for(&bc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&bc.c_ser, kind, &bc.c_ax_id, is_primary);
                group_types.push(kind);
            }
            c::PlotAreaChoice::CLine3DChart(lc) => {
                if grouping.is_none() {
                    grouping = lc
                        .grouping
                        .val
                        .as_ref()
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(lc.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&lc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&lc.c_ser, "line", &lc.c_ax_id, is_primary);
                for (offset, ser) in lc.c_ser.iter().enumerate() {
                    let sym = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.marker_symbol = sym;
                    }
                }
                group_types.push("line");
            }
            c::PlotAreaChoice::CArea3DChart(ac) => {
                if grouping.is_none() {
                    grouping = ac
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(ac.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&ac.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&ac.c_ser, "area", &ac.c_ax_id, is_primary);
                group_types.push("area");
            }
            c::PlotAreaChoice::CPie3DChart(pc) => {
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                extract_chartlike!(&pc.c_ser, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::CRadarChart(rc) => {
                // ECMA-376 §21.2.2.155 / §21.2.2.176. Radar charts are
                // category-axis + value-axis the same way line charts
                // are, so we reuse the line series shape: idx/order/tx/
                // spPr/cat/val/dPt/dLbls. The renderer wraps the
                // category axis into a polar layout. `radarStyle`:
                //   - `standard` — line only
                //   - `marker`   — line + markers (Excel UI default)
                //   - `filled`   — filled polygon (semi-transparent)
                if radar_style.is_none() {
                    radar_style = Some(
                        match rc.radar_style.val {
                            c::RadarStyleValues::Standard => "standard",
                            c::RadarStyleValues::Marker => "marker",
                            c::RadarStyleValues::Filled => "filled",
                        }
                        .to_string(),
                    );
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(rc.c_d_lbls.as_deref());
                }
                let series_before = series.len();
                extract_chartlike!(&rc.c_ser, "radar", &rc.c_ax_id, true);
                // RadarChartSeries also carries a top-level `marker`
                // node (same shape as LineChartSeries); propagate it
                // so per-series marker symbol overrides survive.
                for (offset, ser) in rc.c_ser.iter().enumerate() {
                    let sym = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.marker_symbol = sym;
                    }
                }
                group_types.push("radar");
                break;
            }
            c::PlotAreaChoice::CStockChart(sc) => {
                // ECMA-376 §21.2.2.207. Stock charts are line-shaped
                // series (LineChartSeries) with optional hiLowLines /
                // upDownBars / dropLines decoration. The series count
                // implies subtype:
                //   3  → High-Low-Close (HLC)
                //   4  → Open-High-Low-Close (OHLC)
                //   4  → Volume-High-Low-Close (VHLC, if first series
                //          is on a secondary axis as a column group;
                //          xlsxwriter emits this differently, with a
                //          parallel `<c:barChart>` for volume)
                //   5  → Volume-Open-High-Low-Close (VOHLC)
                // We just expose series + decoration flags; the
                // renderer infers subtype from `series.length`.
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(sc.c_d_lbls.as_deref());
                }
                stock_hi_low_lines = stock_hi_low_lines || sc.c_hi_low_lines.is_some();
                stock_up_down_bars = stock_up_down_bars || sc.c_up_down_bars.is_some();
                stock_drop_lines = stock_drop_lines || sc.c_drop_lines.is_some();
                let series_before = series.len();
                extract_chartlike!(&sc.c_ser, "stock", &sc.c_ax_id, true);
                // StockChart shares LineChartSeries; propagate the
                // per-series marker symbol (same as line). xlsxwriter
                // emits `<c:marker><c:symbol val="none"/></c:marker>`
                // on high/low and `<c:symbol val="dot"/>` on close —
                // we honor that so only close paints a marker.
                for (offset, ser) in sc.c_ser.iter().enumerate() {
                    let sym = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.marker_symbol = sym;
                    }
                }
                group_types.push("stock");
                break;
            }
            c::PlotAreaChoice::COfPieChart(pc) => {
                // ECMA-376 §21.2.2.124. `ofPieType` (`pie` | `bar`)
                // would split the second plot into either a satellite
                // pie or bar of grouped slices; we approximate as a
                // plain pie until the satellite layout lands.
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                extract_chartlike!(&pc.c_ser, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::CBubbleChart(bc) => {
                // ECMA-376 §21.2.2.30 / .197: bubbleScale (0..=300,
                // default 100), sizeRepresents (`area` default or `w`).
                bubble_scale = bc.c_bubble_scale.as_ref().and_then(|s| s.val);
                size_represents = bc
                    .c_size_represents
                    .as_ref()
                    .and_then(|s| s.val.as_ref())
                    .map(|v| {
                        match v {
                            c::SizeRepresentsValues::Area => "area",
                            c::SizeRepresentsValues::Width => "w",
                        }
                        .to_string()
                    });
                chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                for ser in &bc.c_ser {
                    let mut row = common_series_scatter(
                        &ser.order,
                        ser.series_text.as_deref(),
                        ser.chart_shape_properties.as_deref(),
                        ser.c_y_val.as_deref(),
                        &ser.c_d_pt,
                        theme,
                    );
                    let (xs, xref) = scatter_x_values(ser.c_x_val.as_deref());
                    row.x_values = xs;
                    row.x_values_ref = xref;
                    // BubbleSize shares the `CT_NumDataSource` shape
                    // with YValues / Values. Reuse the YValues parser
                    // by transmuting through a shared accessor.
                    let (sizes, sref) = bubble_size_values(ser.c_bubble_size.as_deref());
                    row.bubble_sizes = sizes;
                    row.bubble_sizes_ref = sref;
                    row.data_labels = extract_data_labels(ser.c_d_lbls.as_deref());
                    row.axis_group = axis_group_for(&bc.c_ax_id, &secondary_ax_ids);
                    row.chart_type = Some("bubble".to_string());
                    series.push(row);
                    if categories.is_empty() {
                        let (cs, r, fmt) = x_axis_values(ser.c_x_val.as_deref());
                        categories = cs;
                        _categories_ref = r;
                        categories_format = fmt;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.c_y_val.as_deref());
                    }
                }
                group_types.push("bubble");
                break;
            }
            _ => {}
        }
    }

    // Single-type charts: collapse to the historical scalar type.
    // Multi-type plotArea ⇒ "combo"; renderer dispatches per-series.
    let unique_types: std::collections::BTreeSet<&&str> = group_types.iter().collect();
    let chart_type = match unique_types.len() {
        0 => "unknown".to_string(),
        1 => (*group_types.first().unwrap()).to_string(),
        _ => "combo".to_string(),
    };
    // When the chart isn't a combo, clear per-series chart_type so we
    // don't bloat the JSON output for the common case.
    if chart_type != "combo" {
        for s in &mut series {
            s.chart_type = None;
        }
    }
    // Same for axis_group when no secondary axis exists.
    if !secondary_axis {
        for s in &mut series {
            s.axis_group = None;
        }
    }
    let value_format = value_format.or(primary_val_fmt);

    // Title resolution (ECMA-376 §21.2.2.211 + §21.2.2.4):
    //   - `<c:title><c:tx>...` explicit text wins.
    //   - `<c:title>` present *without* `<c:tx>` AND `<c:autoTitleDeleted
    //     val="0"/>` (or element absent, which defaults to false) AND
    //     the chart has exactly one series → Excel auto-generates the
    //     title from that series's name. This is how the AGS NWC line
    //     chart picks up its "NWC" title even though chart15.xml's
    //     `<c:title>` carries only `<c:spPr>`/`<c:txPr>` (formatting,
    //     no text node).
    //   - `<c:autoTitleDeleted val="1"/>` → user explicitly cleared
    //     the auto title; we honor that and emit no title.
    let auto_title_deleted = chart
        .auto_title_deleted
        .as_ref()
        .and_then(|a| a.val)
        .unwrap_or(false);
    let title = extract_title(chart.title.as_deref()).or_else(|| {
        if chart.title.is_some() && !auto_title_deleted && series.len() == 1 {
            let n = series[0].name.clone();
            if n.is_empty() { None } else { Some(n) }
        } else {
            None
        }
    });
    // Legend presence + position. Critical distinction:
    //   - `<c:legend>` absent             → legend_pos = None        (don't paint)
    //   - `<c:legend>` present, no <c:legendPos>  → legend_pos = Some("r")  (Excel default)
    //   - `<c:legend>` present with <c:legendPos>  → legend_pos = Some(<that>)
    // The renderer treats `None` as "no legend". This matters because
    // many AGS workbook charts have no `<c:legend>` element at all
    // (e.g. the per-data-point waterfall on `Charts_Chart_2.xlsx` —
    // see parity-charts.md Bug #3) and Excel desktop / hsx correctly
    // omit the legend; pre-fix we were defaulting to "b" whenever
    // `legend_pos` was `None`, fabricating a legend for every chart.
    let legend_pos = chart.legend.as_ref().map(|l| {
        l.legend_position
            .as_ref()
            .and_then(|lp| lp.val.as_ref())
            .map(|v| format!("{:?}", v).to_ascii_lowercase())
            .map(|s| match s.as_str() {
                x if x.contains("bottom") => "b".to_string(),
                x if x.contains("top") && x.contains("right") => "tr".to_string(),
                x if x.contains("top") => "t".to_string(),
                x if x.contains("left") => "l".to_string(),
                x if x.contains("right") => "r".to_string(),
                _ => "r".to_string(),
            })
            .unwrap_or_else(|| "r".to_string())
    });

    Some(Chart {
        chart_type,
        title,
        series,
        categories,
        categories_ref: _categories_ref,
        categories_format,
        legend_pos,
        value_format,
        grouping,
        bar_dir,
        scatter_style,
        radar_style,
        data_labels: chart_data_labels,
        secondary_axis,
        value_format_secondary: secondary_val_fmt,
        value_min,
        value_max,
        value_min_secondary,
        value_max_secondary,
        major_unit,
        major_unit_secondary,
        bar_gap_width,
        bar_overlap,
        x_axis_title,
        y_axis_title,
        y_axis_title_secondary,
        show_major_gridlines,
        show_major_gridlines_secondary,
        disp_units,
        disp_units_label,
        disp_units_secondary,
        disp_units_label_secondary,
        bubble_scale,
        size_represents,
        stock_hi_low_lines,
        stock_up_down_bars,
        stock_drop_lines,
        cx_layout: None,
        cx_subtotal_indices: Vec::new(),
        cx_category_levels: Vec::new(),
        cx_waterfall_increment_color: None,
        cx_waterfall_decrement_color: None,
        cx_waterfall_subtotal_color: None,
    })
}

/// Map a chartEx `SeriesLayout` to the schema's `cx_layout` string.
fn cx_layout_name(l: &cx::SeriesLayout) -> &'static str {
    match l {
        cx::SeriesLayout::Waterfall => "waterfall",
        cx::SeriesLayout::Funnel => "funnel",
        cx::SeriesLayout::Treemap => "treemap",
        cx::SeriesLayout::Sunburst => "sunburst",
        cx::SeriesLayout::BoxWhisker => "boxWhisker",
        cx::SeriesLayout::ParetoLine => "paretoLine",
        cx::SeriesLayout::RegionMap => "regionMap",
        cx::SeriesLayout::ClusteredColumn => "clusteredColumn",
    }
}

/// Extract title text from a chartEx `<cx:title>` element. Mirrors
/// `extract_title` for legacy charts but walks the chartEx-namespaced
/// `Text` / `RichTextBody` shape.
fn extract_chart_ex_title(t: Option<&cx::ChartTitle>) -> Option<String> {
    let t = t?;
    let text = t.text.as_deref()?;
    let choice = text.text_choice.as_ref()?;
    match choice {
        cx::TextChoice::CxTxData(td) => extract_text_data_v(td),
        cx::TextChoice::CxRich(rich) => {
            // Concatenate `<a:t>` text across each paragraph's runs.
            // chartEx rich text reuses the regular drawingml namespace
            // for paragraphs / runs, so we walk the `a:` types from
            // `ooxmlsdk::schemas::...drawingml_2006_main`.
            use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
            let mut out = String::new();
            for p in &rich.a_p {
                for ch in &p.paragraph_choice {
                    if let a::ParagraphChoice::AR(run) = ch {
                        out.push_str(run.text.as_str());
                    }
                }
            }
            if out.is_empty() { None } else { Some(out) }
        }
    }
}

/// Pull the inline `<cx:v>` text from a `<cx:txData>` block, whether
/// the schema picked the bare `CxV` variant or the multi-child
/// `Sequence` variant.
fn extract_text_data_v(td: &cx::TextData) -> Option<String> {
    match td.text_data_choice.as_ref()? {
        cx::TextDataChoice::CxV(s) => Some(s.clone()),
        cx::TextDataChoice::Sequence { v_xsdstring, .. } => v_xsdstring.clone(),
    }
}

/// One series' parsed data — produced by `parse_series_data`. Captures
/// the categories from the series's data block (only relevant for the
/// first/primary series in multi-series chartEx — subsequent series'
/// own categories are ignored), plus that series's numeric values and
/// the formula reference used to fill them in via `refs.rs`.
struct ParsedSeriesData {
    categories: Vec<String>,
    categories_ref: Option<String>,
    values: Vec<f64>,
    values_ref: Option<String>,
    value_format: Option<String>,
}

/// Resolve a chartEx series's `<cx:dataId>` to its `<cx:data>` block
/// under `<cx:chartData>`, then walk the inner dimensions to extract
/// categories + numeric values. Returns `None` when the chartData
/// block is missing entirely.
fn parse_series_data(
    space: &cx::ChartSpace,
    series: &cx::Series,
) -> Option<ParsedSeriesData> {
    let data_id = series.cx_data_id.as_ref().map(|d| d.val).unwrap_or(0);
    let data_block = space
        .chart_data
        .as_deref()
        .and_then(|cd| cd.cx_data.iter().find(|d| d.id == data_id))
        .or_else(|| space.chart_data.as_deref().and_then(|cd| cd.cx_data.first()))?;

    let mut categories: Vec<String> = Vec::new();
    let mut categories_ref: Option<String> = None;
    let mut values: Vec<f64> = Vec::new();
    let mut values_ref: Option<String> = None;
    let mut value_format: Option<String> = None;

    for choice in &data_block.data_choice {
        match choice {
            cx::DataChoice::CxStrDim(sd) => {
                if !matches!(sd.r#type, cx::StringDimensionType::Cat) {
                    continue;
                }
                let levels: Vec<&cx::StringLevel> = match sd.string_dimension_choice.as_ref() {
                    Some(cx::StringDimensionChoice::Sequence(seq)) => {
                        if let Some(s) = seq.formula.xml_content.as_ref() {
                            categories_ref = Some(s.clone());
                        }
                        seq.string_level.iter().collect()
                    }
                    Some(cx::StringDimensionChoice::CxLvl(lvl)) => vec![lvl.as_ref()],
                    None => Vec::new(),
                };
                if let Some(lvl) = levels.first() {
                    let n = lvl.pt_count as usize;
                    categories = vec![String::new(); n];
                    for pt in &lvl.cx_pt {
                        let i = pt.index as usize;
                        if i < n {
                            categories[i] = pt.xml_content.clone().unwrap_or_default();
                        }
                    }
                }
            }
            cx::DataChoice::CxNumDim(nd) => {
                // Funnel / waterfall / pareto use `type="val"`; treemap /
                // sunburst / histogram use `type="size"` (the dimension
                // encodes rectangle/ring area or histogram-bin count
                // rather than a y-axis value). Both map to the same
                // `values` vector — the per-layout painter knows what
                // the numbers mean.
                if !matches!(
                    nd.r#type,
                    cx::NumericDimensionType::Val | cx::NumericDimensionType::Size
                ) {
                    continue;
                }
                let levels: Vec<&cx::NumericLevel> = match nd.numeric_dimension_choice.as_ref() {
                    Some(cx::NumericDimensionChoice::Sequence(seq)) => {
                        if let Some(s) = seq.formula.xml_content.as_ref() {
                            values_ref = Some(s.clone());
                        }
                        seq.numeric_level.iter().collect()
                    }
                    Some(cx::NumericDimensionChoice::CxLvl(lvl)) => vec![lvl.as_ref()],
                    None => Vec::new(),
                };
                if let Some(lvl) = levels.first() {
                    let n = lvl.pt_count as usize;
                    values = vec![0.0; n];
                    for pt in &lvl.cx_pt {
                        let i = pt.idx as usize;
                        if i < n {
                            values[i] = pt.xml_content.unwrap_or(0.0);
                        }
                    }
                    if let Some(fc) = &lvl.format_code {
                        value_format = Some(fc.clone());
                    }
                }
            }
        }
    }

    Some(ParsedSeriesData {
        categories,
        categories_ref,
        values,
        values_ref,
        value_format,
    })
}

/// Extract a series's display name from its `<cx:tx><cx:txData><cx:v>`.
fn parse_series_name(series: &cx::Series) -> String {
    series
        .text
        .as_deref()
        .and_then(|t| t.text_choice.as_ref())
        .and_then(|c| match c {
            cx::TextChoice::CxTxData(td) => extract_text_data_v(td),
            cx::TextChoice::CxRich(_) => None,
        })
        .unwrap_or_default()
}

/// Build a bare `ChartSeries` carrying just name + values. chartEx
/// series don't currently surface per-series colors / data labels /
/// axis-group toggles — those slots stay at defaults for the renderer
/// to fill in (e.g. boxWhisker accents come from the theme).
fn make_chart_series(
    name: String,
    values: Vec<f64>,
    values_ref: Option<String>,
) -> ChartSeries {
    ChartSeries {
        name,
        name_ref: None,
        color: None,
        values,
        values_ref,
        x_values: Vec::new(),
        x_values_ref: None,
        bubble_sizes: Vec::new(),
        bubble_sizes_ref: None,
        point_colors: Vec::new(),
        data_labels: None,
        axis_group: None,
        chart_type: None,
        marker_symbol: None,
    }
}

/// True when this series's layoutPr carries a `<cx:binning>` element —
/// the marker for a clusteredColumn that should render as a histogram
/// (auto- or explicit-binned columns over a continuous value axis)
/// rather than as a plain categorical column chart.
fn series_has_binning(series: &cx::Series) -> bool {
    series
        .cx_layout_pr
        .as_deref()
        .and_then(|lp| lp.series_layout_properties_choice.as_ref())
        .is_some_and(|c| matches!(c, cx::SeriesLayoutPropertiesChoice::CxBinning(_)))
}

/// chartEx (cx:) extractor. Surfaces all series with their values +
/// `cx_layout` set to a renderer-friendly tag:
///
///   - `"waterfall"` / `"funnel"` / `"treemap"` / `"sunburst"` /
///     `"regionMap"` — single-series layouts (existing v1 scope).
///   - `"histogram"` — single clusteredColumn series whose layoutPr
///     carries `<cx:binning>`. The renderer auto-bins the raw values.
///   - `"pareto"` — two series: a primary clusteredColumn plus a
///     secondary paretoLine that shares the primary's data (the
///     cumulative-% line is computed at draw time).
///   - `"boxWhisker"` — N parallel boxWhisker series; each carries
///     a column of raw observations. Quartiles / whiskers are
///     computed at draw time per the layoutPr `quartileMethod`.
///
/// Returns `Some(Chart)` with `chart_type = "chartex"`.
fn extract_chart_ex(space: &cx::ChartSpace) -> Option<Chart> {
    let chart = space.chart.as_ref();
    let plot_area = chart.plot_area.as_ref();
    let region = plot_area.plot_area_region.as_ref();
    let series_list = &region.cx_series;
    let first = series_list.first()?;

    // Detect the layout family. Most legacy chartEx layouts are
    // single-series and map straight from the primary series's
    // `layoutId`; histogram / pareto / boxWhisker compose multiple
    // series or signal via layoutPr.
    let has_pareto_line = series_list
        .iter()
        .any(|s| matches!(s.layout_id, cx::SeriesLayout::ParetoLine));
    let all_box_whisker = !series_list.is_empty()
        && series_list
            .iter()
            .all(|s| matches!(s.layout_id, cx::SeriesLayout::BoxWhisker));
    let single_histogram = series_list.len() == 1
        && matches!(first.layout_id, cx::SeriesLayout::ClusteredColumn)
        && series_has_binning(first);
    let layout = if has_pareto_line {
        "pareto".to_string()
    } else if all_box_whisker {
        "boxWhisker".to_string()
    } else if single_histogram {
        "histogram".to_string()
    } else {
        cx_layout_name(&first.layout_id).to_string()
    };

    // Primary series's parsed data also supplies the chart-level
    // categories + value format (consumed by axis-tick rendering even
    // for multi-series layouts).
    let primary_data = parse_series_data(space, first)?;

    // Build the schema's `series` vector. Pareto + boxWhisker carry
    // multiple series; everything else surfaces just the primary so
    // the existing single-series consumers stay backwards compatible.
    let series: Vec<ChartSeries> = if layout == "boxWhisker" {
        series_list
            .iter()
            .filter_map(|s| {
                let parsed = parse_series_data(space, s)?;
                Some(make_chart_series(
                    parse_series_name(s),
                    parsed.values,
                    parsed.values_ref,
                ))
            })
            .collect()
    } else if layout == "pareto" {
        // Walk in source order so legend / series indexing stays
        // predictable. The paretoLine companion shares the primary's
        // data block (no own `<cx:dataId>`); its values are filled in
        // at render time as a cumulative percentage.
        let mut out: Vec<ChartSeries> = Vec::with_capacity(series_list.len());
        for s in series_list {
            let name = parse_series_name(s);
            match s.layout_id {
                cx::SeriesLayout::ParetoLine => {
                    let display = if name.is_empty() {
                        "Cumulative %".to_string()
                    } else {
                        name
                    };
                    out.push(make_chart_series(display, Vec::new(), None));
                }
                _ => {
                    let parsed = parse_series_data(space, s)?;
                    out.push(make_chart_series(name, parsed.values, parsed.values_ref));
                }
            }
        }
        out
    } else {
        vec![make_chart_series(
            parse_series_name(first),
            primary_data.values.clone(),
            primary_data.values_ref.clone(),
        )]
    };

    // Subtotal indices (`<cx:layoutPr><cx:subtotals><cx:idx val="N"/>`).
    let subtotal_indices: Vec<u32> = first
        .cx_layout_pr
        .as_deref()
        .and_then(|lp| lp.cx_subtotals.as_ref())
        .map(|sub| sub.cx_idx.iter().map(|i| i.val).collect())
        .unwrap_or_default();

    let title = extract_chart_ex_title(chart.chart_title.as_deref());

    // Legend presence: chartEx legends are uncommon for waterfall;
    // honour the same "absent => no paint" rule used for legacy charts.
    let legend_pos = chart.legend.as_ref().map(|l| {
        match l.pos.as_ref() {
            Some(cx::SidePos::B) => "b",
            Some(cx::SidePos::T) => "t",
            Some(cx::SidePos::L) => "l",
            Some(cx::SidePos::R) => "r",
            None => "r",
        }
        .to_string()
    });

    Some(Chart {
        chart_type: "chartex".to_string(),
        title,
        series,
        categories: primary_data.categories,
        categories_ref: primary_data.categories_ref,
        categories_format: None,
        legend_pos,
        value_format: primary_data.value_format,
        grouping: None,
        bar_dir: None,
        scatter_style: None,
        radar_style: None,
        data_labels: None,
        secondary_axis: false,
        value_format_secondary: None,
        value_min: None,
        value_max: None,
        value_min_secondary: None,
        value_max_secondary: None,
        major_unit: None,
        major_unit_secondary: None,
        bar_gap_width: None,
        bar_overlap: None,
        x_axis_title: None,
        y_axis_title: None,
        y_axis_title_secondary: None,
        show_major_gridlines: None,
        show_major_gridlines_secondary: None,
        disp_units: None,
        disp_units_label: None,
        disp_units_secondary: None,
        disp_units_label_secondary: None,
        bubble_scale: None,
        size_represents: None,
        stock_hi_low_lines: false,
        stock_up_down_bars: false,
        stock_drop_lines: false,
        cx_layout: Some(layout),
        cx_subtotal_indices: subtotal_indices,
        cx_category_levels: Vec::new(),
        cx_waterfall_increment_color: None,
        cx_waterfall_decrement_color: None,
        cx_waterfall_subtotal_color: None,
    })
}
