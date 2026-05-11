//! Extract drawings (charts) from a worksheet's DrawingsPart.
//!
//! v0 covers clustered/stacked column + bar charts. The data we need lives in
//! the chart part's `numCache`/`strCache` blocks, which Office writes any
//! time it saves the workbook -- so we can render charts without recalc.
//!
//! Pie/line/area/scatter/etc. are recognised but rendered as a placeholder
//! box with title for now.

use crate::schema::*;
use base64::Engine;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;

/// What kind of drawing the anchor points at, plus the rId we need to
/// resolve through the drawing-part's relationships.
enum AnchorTarget {
    Chart(String),
    Image(String),
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
                };
                let target = match a.two_cell_anchor_choice.as_ref()? {
                    xdr::TwoCellAnchorChoice::XdrGraphicFrame(gf) => {
                        // The chart `r:id` is in the graphicData's untyped
                        // children: `<c:chart r:id="rId1"/>`. Regex it out.
                        AnchorTarget::Chart(find_relationship_id(
                            &gf.graphic.graphic_data.xml_children,
                        )?)
                    }
                    xdr::TwoCellAnchorChoice::XdrPic(pic) => {
                        let blip = pic.blip_fill.blip.as_ref()?;
                        let embed = blip.embed.as_ref()?;
                        AnchorTarget::Image(embed.as_str().to_string())
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
    let image_by_rid: Vec<(String, ooxmlsdk::parts::image_part::ImagePart)> = image_parts
        .into_iter()
        .filter_map(|p| Some((part_relationship_id_dbg(&p)?, p)))
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

    let mut chart_type = "unknown".to_string();
    let mut bar_dir: Option<String> = None;
    let mut scatter_style: Option<String> = None;
    let mut grouping: Option<String> = None;
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut categories: Vec<String> = Vec::new();
    let mut _categories_ref: Option<String> = None;
    let mut value_format: Option<String> = None;

    // Helper macro: extract the (categories, value_format) for the first
    // series of a chart that uses c_cat / c_val (i.e. everything except
    // scatter). Also pushes per-series ChartSeries rows.
    let mut chart_data_labels: Option<DataLabels> = None;

    macro_rules! extract_chartlike {
        ($coll:expr) => {{
            let mut cats_ref: Option<String> = None;
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
                series.push(row);
                if categories.is_empty() {
                    let (cs, r) = ax_data_values(ser.c_cat.as_deref());
                    categories = cs;
                    cats_ref = r;
                }
                if value_format.is_none() {
                    value_format = values_format(ser.c_val.as_deref());
                }
            }
            _categories_ref = cats_ref;
        }};
    }

    for choice in &plot_area.plot_area_choice1 {
        match choice {
            c::PlotAreaChoice::CBarChart(bc) => {
                chart_type = "bar".to_string();
                bar_dir = Some(format!("{:?}", bc.bar_direction.val).to_ascii_lowercase());
                if let Some(g) = &bc.bar_grouping {
                    grouping = g
                        .val
                        .as_ref()
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                extract_chartlike!(&bc.c_ser);
                break;
            }
            c::PlotAreaChoice::CLineChart(lc) => {
                chart_type = "line".into();
                grouping = lc
                    .grouping
                    .val
                    .as_ref()
                    .map(|v| format!("{:?}", v).to_ascii_lowercase());
                chart_data_labels = extract_data_labels(lc.c_d_lbls.as_deref());
                extract_chartlike!(&lc.c_ser);
                break;
            }
            c::PlotAreaChoice::CAreaChart(ac) => {
                chart_type = "area".into();
                grouping = ac
                    .grouping
                    .as_ref()
                    .and_then(|g| g.val.as_ref())
                    .map(|v| format!("{:?}", v).to_ascii_lowercase());
                chart_data_labels = extract_data_labels(ac.c_d_lbls.as_deref());
                extract_chartlike!(&ac.c_ser);
                break;
            }
            c::PlotAreaChoice::CPieChart(pc) => {
                chart_type = "pie".into();
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                extract_chartlike!(&pc.c_ser);
                break;
            }
            c::PlotAreaChoice::CDoughnutChart(dc) => {
                chart_type = "doughnut".into();
                chart_data_labels = extract_data_labels(dc.c_d_lbls.as_deref());
                extract_chartlike!(&dc.c_ser);
                break;
            }
            c::PlotAreaChoice::CScatterChart(sc) => {
                chart_type = "scatter".into();
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
                let mut cats_ref: Option<String> = None;
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
                    series.push(row);
                    if categories.is_empty() {
                        // Stash the x-axis ref for axis labels (numeric).
                        let (cs, r) = x_axis_values(ser.c_x_val.as_deref());
                        categories = cs;
                        cats_ref = r;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.c_y_val.as_deref());
                    }
                }
                _categories_ref = cats_ref;
                break;
            }
            c::PlotAreaChoice::CBar3DChart(_) => {
                chart_type = "bar".into();
                break;
            }
            _ => {}
        }
    }

    if chart_type == "bar" && matches!(bar_dir.as_deref(), Some("col") | Some("column")) {
        chart_type = "column".to_string();
    }

    let title = extract_title(chart.title.as_deref());
    let legend_pos = chart
        .legend
        .as_ref()
        .and_then(|l| l.legend_position.as_ref())
        .and_then(|lp| lp.val.as_ref())
        .map(|v| format!("{:?}", v).to_ascii_lowercase())
        .map(|s| match s.as_str() {
            x if x.contains("bottom") => "b".to_string(),
            x if x.contains("top") && x.contains("right") => "tr".to_string(),
            x if x.contains("top") => "t".to_string(),
            x if x.contains("left") => "l".to_string(),
            x if x.contains("right") => "r".to_string(),
            _ => "b".to_string(),
        });

    Some(Chart {
        chart_type,
        title,
        series,
        categories,
        categories_ref: _categories_ref,
        legend_pos,
        value_format,
        grouping,
        bar_dir,
        scatter_style,
        data_labels: chart_data_labels,
    })
}

/// Convert an OOXML `<c:dLbls>` block into our flat `DataLabels` shape.
/// Returns `None` when the block is fully absent or carries `<c:delete
/// val="1"/>` (Excel's "labels suppressed" marker), or when no show*
/// flag is enabled — there's nothing to render in that case.
fn extract_data_labels(dl: Option<&c::DataLabels>) -> Option<DataLabels> {
    let dl = dl?;
    let seq = match dl.data_labels_choice.as_ref()? {
        c::DataLabelsChoice::CDelete(_) => return None,
        c::DataLabelsChoice::Sequence(s) => s,
    };
    // OOXML CT_Boolean: element absent ⇒ false; element present with no
    // val attr ⇒ true (per ECMA-376 part 1, §21.2.2.4 default); element
    // present with val="0"/"false" ⇒ false; val="1"/"true" ⇒ true.
    let show_value = seq
        .show_value
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    let show_category = seq
        .show_category_name
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    let show_series_name = seq
        .show_series_name
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    let show_percent = seq
        .show_percent
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    if !show_value && !show_category && !show_series_name && !show_percent {
        return None;
    }
    let position = seq.data_label_position.as_ref().map(|p| {
        match p.val {
            c::DataLabelPositionValues::BestFit => "bestFit",
            c::DataLabelPositionValues::Bottom => "b",
            c::DataLabelPositionValues::Center => "ctr",
            c::DataLabelPositionValues::InsideBase => "inBase",
            c::DataLabelPositionValues::InsideEnd => "inEnd",
            c::DataLabelPositionValues::Left => "l",
            c::DataLabelPositionValues::OutsideEnd => "outEnd",
            c::DataLabelPositionValues::Right => "r",
            c::DataLabelPositionValues::Top => "t",
        }
        .to_string()
    });
    let separator = seq.separator.as_ref().map(|s| s.as_str().to_string());
    let num_fmt = seq
        .numbering_format
        .as_ref()
        .map(|nf| nf.format_code.as_str().to_string());
    Some(DataLabels {
        show_value,
        show_category,
        show_series_name,
        show_percent,
        position,
        separator,
        num_fmt,
    })
}

/// Common per-series extraction shared by bar/line/area/pie. Reads name,
/// color, and y-values from the standard `c:tx` / `c:spPr` / `c:val` slots.
fn common_series(
    order: &c::Order,
    tx: Option<&c::SeriesText>,
    sp_pr: Option<&c::ChartShapeProperties>,
    val: Option<&c::Values>,
    d_pts: &[c::DataPoint],
    theme: Option<&Theme>,
) -> ChartSeries {
    let (name, name_ref) = series_text_or_ref(tx);
    let (values, values_ref) = number_reference_values(val);
    let color = series_color_via_debug(sp_pr, theme).or_else(|| {
        let n = order.val % 6 + 1;
        Some(theme_accent_color(n, theme))
    });
    let point_colors = extract_point_colors(d_pts, values.len(), theme);
    ChartSeries {
        name: name.unwrap_or_default(),
        name_ref,
        color,
        values,
        values_ref,
        x_values: Vec::new(),
        x_values_ref: None,
        point_colors,
        data_labels: None,
    }
}

/// Same as `common_series` but takes the scatter-only `YValues` shape.
fn common_series_scatter(
    order: &c::Order,
    tx: Option<&c::SeriesText>,
    sp_pr: Option<&c::ChartShapeProperties>,
    y_val: Option<&c::YValues>,
    d_pts: &[c::DataPoint],
    theme: Option<&Theme>,
) -> ChartSeries {
    let (name, name_ref) = series_text_or_ref(tx);
    let (values, values_ref) = y_values_values(y_val);
    let color = series_color_via_debug(sp_pr, theme).or_else(|| {
        let n = order.val % 6 + 1;
        Some(theme_accent_color(n, theme))
    });
    let point_colors = extract_point_colors(d_pts, values.len(), theme);
    ChartSeries {
        name: name.unwrap_or_default(),
        name_ref,
        color,
        values,
        values_ref,
        x_values: Vec::new(),
        x_values_ref: None,
        point_colors,
        data_labels: None,
    }
}

/// Build a `point_colors` Vec from `<c:dPt>` children. Returns an empty
/// Vec when no data point carries an explicit fill (the common case),
/// so the renderer can cheaply fall back to its per-slice palette /
/// series color. When at least one `<c:dPt>` does have a fill, the
/// returned Vec is sized to `max(values_len, max_dpt_idx + 1)` and
/// indexed by the data point's `c:idx` value; missing entries are
/// empty strings. We can't always trust `values_len` because pie/line
/// series load their numbers from `<c:numRef>` formulas that are only
/// resolved post-sheet-extract — at chart-extract time `values` is
/// commonly empty even though the dPts are present.
fn extract_point_colors(
    d_pts: &[c::DataPoint],
    values_len: usize,
    theme: Option<&Theme>,
) -> Vec<String> {
    if d_pts.is_empty() {
        return Vec::new();
    }
    let max_idx = d_pts
        .iter()
        .map(|dp| dp.index.val as usize)
        .max()
        .unwrap_or(0);
    let len = values_len.max(max_idx + 1);
    let mut out = vec![String::new(); len];
    let mut any = false;
    for dp in d_pts {
        let idx = dp.index.val as usize;
        if idx >= len {
            continue;
        }
        if let Some(c) = series_color_via_debug(dp.chart_shape_properties.as_deref(), theme) {
            out[idx] = c;
            any = true;
        }
    }
    if any {
        out
    } else {
        Vec::new()
    }
}

/// Read string/number values out of a CategoryAxisData slot (used by both
/// `c:cat` on bar/line/area/pie and `c:xVal` on scatter).
fn ax_data_values(cat: Option<&c::CategoryAxisData>) -> (Vec<String>, Option<String>) {
    let Some(cat) = cat else {
        return (Vec::new(), None);
    };
    let Some(choice) = cat.category_axis_data_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::CategoryAxisDataChoice::CStrRef(sr) => (
            string_cache_values(&sr.string_cache),
            Some(sr.formula.as_str().to_string()),
        ),
        c::CategoryAxisDataChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    nc.c_pt
                        .iter()
                        .map(|p| p.numeric_value.as_str().to_string())
                        .collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::CategoryAxisDataChoice::CStrLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
        ),
        c::CategoryAxisDataChoice::CNumLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
        ),
        _ => (Vec::new(), None),
    }
}

fn values_format(v: Option<&c::Values>) -> Option<String> {
    let v = v?;
    match v.values_choice.as_ref()? {
        c::ValuesChoice::CNumRef(nr) => nr
            .numbering_cache
            .as_ref()
            .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
        c::ValuesChoice::CNumLit(lit) => lit.format_code.as_ref().map(|s| s.as_str().to_string()),
    }
}

fn y_values_values(v: Option<&c::YValues>) -> (Vec<f64>, Option<String>) {
    let Some(v) = v else {
        return (Vec::new(), None);
    };
    let Some(choice) = v.y_values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::YValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::YValuesChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
    }
}

fn y_values_format(v: Option<&c::YValues>) -> Option<String> {
    let v = v?;
    match v.y_values_choice.as_ref()? {
        c::YValuesChoice::CNumRef(nr) => nr
            .numbering_cache
            .as_ref()
            .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
        c::YValuesChoice::CNumLit(lit) => lit.format_code.as_ref().map(|s| s.as_str().to_string()),
    }
}

fn x_axis_values(x: Option<&c::XValues>) -> (Vec<String>, Option<String>) {
    let Some(x) = x else {
        return (Vec::new(), None);
    };
    let Some(choice) = x.x_values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::XValuesChoice::CStrRef(sr) => (
            string_cache_values(&sr.string_cache),
            Some(sr.formula.as_str().to_string()),
        ),
        c::XValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    nc.c_pt
                        .iter()
                        .map(|p| p.numeric_value.as_str().to_string())
                        .collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::XValuesChoice::CStrLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
        ),
        c::XValuesChoice::CNumLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
        ),
        _ => (Vec::new(), None),
    }
}

/// Numeric x-values for a scatter series. Returns parsed f64s when
/// available, plus the underlying formula ref.
fn scatter_x_values(x: Option<&c::XValues>) -> (Vec<f64>, Option<String>) {
    let Some(x) = x else {
        return (Vec::new(), None);
    };
    let Some(choice) = x.x_values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::XValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::XValuesChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
        _ => (Vec::new(), None),
    }
}

fn series_text_or_ref(t: Option<&c::SeriesText>) -> (Option<String>, Option<String>) {
    let Some(t) = t else {
        return (None, None);
    };
    let Some(choice) = t.series_text_choice.as_ref() else {
        return (None, None);
    };
    match choice {
        c::SeriesTextChoice::CStrRef(sr) => {
            let cached = sr.string_cache.as_ref().and_then(|sc| {
                sc.c_pt
                    .first()
                    .map(|p| p.numeric_value.as_str().to_string())
            });
            (cached, Some(sr.formula.as_str().to_string()))
        }
        c::SeriesTextChoice::CV(v) => (Some(v.as_str().to_string()), None),
    }
}

fn number_reference_values(v: Option<&c::Values>) -> (Vec<f64>, Option<String>) {
    let Some(v) = v else {
        return (Vec::new(), None);
    };
    let Some(choice) = v.values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::ValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::ValuesChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
    }
}

fn string_cache_values(sc: &Option<Box<c::StringCache>>) -> Vec<String> {
    let Some(sc) = sc else {
        return Vec::new();
    };
    let mut indexed: Vec<(u32, String)> = sc
        .c_pt
        .iter()
        .map(|p| (p.index, p.numeric_value.as_str().to_string()))
        .collect();
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, v)| v).collect()
}

// Pull explicit fill colors out of a `<c:spPr>` block via the struct's
// Debug repr. ooxmlsdk's choice enums are a moving target across
// versions, so a string scan is pragmatic. The Debug repr uses Rust
// type names (`RgbColorModelHex`, `SchemeColor`, `ASolidFill`) rather
// than the XML qnames (`srgbClr`, `schemeClr`, `solidFill`) — so we
// anchor on `ASolidFill(SolidFill {` to lock onto the fill (and skip
// e.g. line-color `<a:ln>` blocks), then look for the first
// `RgbColorModelHex { ... val: "<6 hex>"` or `SchemeColor { val:
// AccentN` underneath it.
fn series_color_via_debug(
    props: Option<&c::ChartShapeProperties>,
    theme: Option<&Theme>,
) -> Option<String> {
    let props = props?;
    let dbg = format!("{:?}", props);
    // Scope to the *shape's* solid fill (chart_shape_properties_choice2).
    // Important: we deliberately don't look inside `a_ln: Some(Outline {
    // ... ASolidFill(...) })` — that's the outline color, not the fill.
    // Series-level spPr commonly has only an outline, no shape fill, in
    // which case the function correctly returns None and the caller
    // falls back to the theme accent.
    let fill_pos = dbg.find("chart_shape_properties_choice2: Some(ASolidFill(SolidFill {")?;
    let fill_block = &dbg[fill_pos..];
    // Cap the scan at the close of the SolidFill struct (best-effort:
    // first `}))` after the open brace covers the typical shape
    // `ASolidFill(SolidFill { ... Some(ASrgbClr(... { ... }))` with the
    // outer `}))` ending the SolidFill).
    let end = fill_block
        .find("})),")
        .or_else(|| fill_block.find("}))"))
        .unwrap_or(fill_block.len());
    let fill_block = &fill_block[..end];

    if let Some(p) = fill_block.find("RgbColorModelHex {") {
        let rest = &fill_block[p..];
        if let Some(v) = rest.find("val: \"") {
            let rest = &rest[v + 6..];
            if let Some(e) = rest.find('"') {
                let hex = &rest[..e];
                if hex.len() == 6 {
                    return Some(format!("#{}", hex));
                }
            }
        }
    }
    if fill_block.contains("SchemeColor {") {
        for n in 1..=6u32 {
            let needle = format!("Accent{n}");
            if fill_block.contains(&needle) {
                return Some(theme_accent_color(n, theme));
            }
        }
    }
    None
}

/// Resolve `accent{n}` against the workbook theme (slots 4..9 in our
/// spreadsheet-indexed `theme.colors`), falling back to the Office
/// 2007+ defaults when the theme didn't ship one.
fn theme_accent_color(n: u32, theme: Option<&Theme>) -> String {
    if let Some(t) = theme {
        let slot = 3 + n as usize; // accent1 -> theme.colors[4]
        if let Some(hex) = t.colors.get(slot) {
            if hex.len() == 6 {
                return format!("#{}", hex);
            }
        }
    }
    office_accent_color_default(n)
}

fn office_accent_color_default(n: u32) -> String {
    match n {
        1 => "#4472C4",
        2 => "#ED7D31",
        3 => "#A5A5A5",
        4 => "#FFC000",
        5 => "#5B9BD5",
        6 => "#70AD47",
        _ => "#4472C4",
    }
    .to_string()
}

fn extract_title(t: Option<&c::Title>) -> Option<String> {
    let t = t?;
    let txt = t.chart_text.as_ref()?;
    match txt.chart_text_choice.as_ref()? {
        c::ChartTextChoice::CStrRef(sr) => sr.string_cache.as_ref().and_then(|sc| {
            sc.c_pt
                .first()
                .map(|p| p.numeric_value.as_str().to_string())
        }),
        c::ChartTextChoice::CRich(rich) => {
            let mut s = String::new();
            for p in &rich.a_p {
                for ch in &p.paragraph_choice {
                    if let ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::ParagraphChoice::AR(run) = ch {
                        s.push_str(run.text.as_str());
                    }
                }
            }
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        c::ChartTextChoice::CStrLit(lit) => lit
            .c_pt
            .first()
            .map(|p| p.numeric_value.as_str().to_string()),
    }
}
