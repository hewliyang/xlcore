use ooxmlsdk::parts::chart_part::ChartPart;
use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, ChartAnchor, ChartInfo, ChartKind, ChartLegendPosition, ChartPatch,
    ChartSeriesInfo, ChartSeriesPatch,
};

use crate::errors::sdk_err_to_api;
use crate::refs::quote_sheet_name;
use crate::{Result, Workbook};

const CHART_GRAPHIC_DATA_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";

impl Workbook {
    pub fn charts(&mut self, sheet: Option<&str>) -> Result<Vec<ChartInfo>> {
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
            let chart_parts: Vec<ChartPart> =
                drawings_part.chart_parts(&self.doc).collect();
            let chart_by_rid: std::collections::HashMap<String, ChartPart> = chart_parts
                .into_iter()
                .filter_map(|p| p.relationship_id().map(|r| (r.to_string(), p.clone())))
                .collect();

            let drawing_root = drawings_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?
                .clone();

            for (anchor_idx, choice) in drawing_root.worksheet_drawing_choice.iter().enumerate() {
                let xdr::WorksheetDrawingChoice::TwoCellAnchor(anchor) = choice else {
                    continue;
                };
                let Some(rid) = anchor_chart_rid(anchor) else {
                    continue;
                };
                let name = anchor_chart_name(anchor)
                    .unwrap_or_else(|| format!("Chart {}", anchor_idx + 1));
                let Some(chart_part) = chart_by_rid.get(&rid).cloned() else {
                    continue;
                };
                let space_xml = {
                    let space = chart_part
                        .root_element(&mut self.doc)
                        .map_err(sdk_err_to_api)?;
                    space.to_xml().map_err(sdk_err_to_api)?
                };
                let parsed = parse_chart_space_xml(&space_xml);
                out.push(ChartInfo {
                    sheet: sheet_name.clone(),
                    id: rid,
                    name,
                    kind: parsed.kind,
                    title: parsed.title,
                    legend_position: parsed.legend,
                    categories_ref: parsed.categories_ref,
                    series: parsed.series,
                    anchor: anchor_to_chart_anchor(anchor),
                });
            }
        }
        Ok(out)
    }

    pub fn set_chart(&mut self, patch: ChartPatch) -> Result<ChartInfo> {
        if patch.series.is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                "chart must have at least one series",
            )
            .with_sheet(&patch.sheet));
        }
        for s in &patch.series {
            if s.values_ref.trim().is_empty() {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidChart,
                    "chart series values_ref must not be empty",
                )
                .with_sheet(&patch.sheet));
            }
        }

        if !self.sheet_exists(&patch.sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {}", patch.sheet),
            )
            .with_sheet(&patch.sheet));
        }

        let ws_part = self.worksheet_part_for_sheet(&patch.sheet)?;

        let (drawings_part, fresh_drawings) = match ws_part.drawings_part(&self.doc) {
            Some(p) => (p.clone(), false),
            None => {
                let p: DrawingsPart = ws_part
                    .add_new_part_auto_id(&mut self.doc)
                    .map_err(sdk_err_to_api)?;
                let empty = xdr::WorksheetDrawing::default();
                p.set_root_element(&mut self.doc, empty)
                    .map_err(sdk_err_to_api)?;
                (p, true)
            }
        };

        if fresh_drawings {
            let rid = drawings_part
                .relationship_id()
                .ok_or_else(|| {
                    ApiError::new(ApiErrorCode::Other, "new drawings part missing rid")
                })?
                .to_string();
            let ws = ws_part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            ws.drawing = Some(x::Drawing {
                xml_other_attrs: Vec::new(),
                id: rid,
            });
        }

        let chart_part: ChartPart = drawings_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let chart_rid = chart_part
            .relationship_id()
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new chart part missing rid"))?
            .to_string();

        let chart_xml = build_chart_space_xml(&patch);
        let chart_space = c::ChartSpace::from_bytes(chart_xml.as_bytes())
            .map_err(sdk_err_to_api)?;
        chart_part
            .set_root_element(&mut self.doc, chart_space)
            .map_err(sdk_err_to_api)?;

        let chart_index = drawings_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?
            .worksheet_drawing_choice
            .len()
            + 1;
        let chart_name = patch
            .name
            .clone()
            .unwrap_or_else(|| format!("Chart {chart_index}"));

        let anchor_xml = build_two_cell_anchor_xml(&patch.anchor, &chart_name, chart_index, &chart_rid);
        let new_anchor = xdr::TwoCellAnchor::from_bytes(anchor_xml.as_bytes())
            .map_err(sdk_err_to_api)?;

        let drawing_mut = drawings_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        drawing_mut
            .worksheet_drawing_choice
            .push(xdr::WorksheetDrawingChoice::TwoCellAnchor(Box::new(
                new_anchor,
            )));

        Ok(ChartInfo {
            sheet: patch.sheet.clone(),
            id: chart_rid,
            name: chart_name,
            kind: patch.kind,
            title: patch.title.clone(),
            legend_position: patch.legend_position,
            categories_ref: patch.categories_ref.clone(),
            series: patch
                .series
                .iter()
                .map(|s| ChartSeriesInfo {
                    name: s.name.clone(),
                    name_ref: s.name_ref.clone(),
                    values_ref: s.values_ref.clone(),
                })
                .collect(),
            anchor: patch.anchor,
        })
    }

    pub fn remove_chart(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
    ) -> Result<Option<ChartInfo>> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let all = self.charts(Some(&sheet))?;
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
        drawing_mut
            .worksheet_drawing_choice
            .retain(|choice| match choice {
                xdr::WorksheetDrawingChoice::TwoCellAnchor(a) => {
                    anchor_chart_rid(a.as_ref()).as_deref() != Some(id.as_str())
                }
                _ => true,
            });

        let _ = drawings_part
            .delete_part_by_id(&mut self.doc, id.as_str())
            .map_err(sdk_err_to_api)?;

        Ok(Some(info))
    }
}

fn anchor_chart_rid(anchor: &xdr::TwoCellAnchor) -> Option<String> {
    let Some(xdr::TwoCellAnchorChoice::GraphicFrame(gf)) = anchor.two_cell_anchor_choice.as_ref()
    else {
        return None;
    };
    if gf.graphic.graphic_data.uri.as_str() != CHART_GRAPHIC_DATA_URI {
        return None;
    }
    for choice in &gf.graphic.graphic_data.graphic_data_choice {
        if let a::GraphicDataChoice::ChartReference(ch) = choice {
            return Some(ch.id.as_str().to_string());
        }
    }
    None
}

fn anchor_chart_name(anchor: &xdr::TwoCellAnchor) -> Option<String> {
    let xdr::TwoCellAnchorChoice::GraphicFrame(gf) = anchor.two_cell_anchor_choice.as_ref()?
    else {
        return None;
    };
    let cnv = &gf.non_visual_graphic_frame_properties.non_visual_drawing_properties;
    let n = cnv.name.as_str();
    if n.is_empty() {
        None
    } else {
        Some(n.to_string())
    }
}

fn anchor_to_chart_anchor(anchor: &xdr::TwoCellAnchor) -> ChartAnchor {
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

struct ParsedChart {
    kind: ChartKind,
    title: Option<String>,
    legend: Option<ChartLegendPosition>,
    categories_ref: Option<String>,
    series: Vec<ChartSeriesInfo>,
}

fn parse_chart_space_xml(xml: &str) -> ParsedChart {
    let kind = if xml.contains("<c:barChart>") {
        if let Some(pos) = xml.find("<c:barDir") {
            let slice = &xml[pos..];
            if slice.contains("val=\"bar\"") {
                ChartKind::Bar
            } else {
                ChartKind::Column
            }
        } else {
            ChartKind::Column
        }
    } else if xml.contains("<c:lineChart>") {
        ChartKind::Line
    } else if xml.contains("<c:pieChart>") {
        ChartKind::Pie
    } else if xml.contains("<c:areaChart>") {
        ChartKind::Area
    } else {
        ChartKind::Column
    };

    let title = extract_first_between(xml, "<c:title>", "</c:title>")
        .and_then(|inner| extract_first_between(inner, "<a:t>", "</a:t>").map(|s| s.to_string()));

    let legend = extract_first_attr(xml, "<c:legendPos", "val=\"").and_then(|v| match v {
        "r" => Some(ChartLegendPosition::Right),
        "l" => Some(ChartLegendPosition::Left),
        "t" => Some(ChartLegendPosition::Top),
        "b" => Some(ChartLegendPosition::Bottom),
        "tr" => Some(ChartLegendPosition::TopRight),
        _ => None,
    });

    let categories_ref = extract_first_between(xml, "<c:cat>", "</c:cat>")
        .and_then(|inner| extract_first_between(inner, "<c:f>", "</c:f>").map(|s| s.to_string()));

    let mut series = Vec::new();
    let mut cursor = xml;
    while let Some(start) = cursor.find("<c:ser>") {
        let after = &cursor[start + "<c:ser>".len()..];
        let Some(end) = after.find("</c:ser>") else {
            break;
        };
        let body = &after[..end];

        let tx = extract_first_between(body, "<c:tx>", "</c:tx>");
        let name_literal = tx.and_then(|t| extract_first_between(t, "<c:v>", "</c:v>"));
        let name_ref = tx.and_then(|t| extract_first_between(t, "<c:f>", "</c:f>"));

        let values_ref = extract_first_between(body, "<c:val>", "</c:val>")
            .and_then(|inner| extract_first_between(inner, "<c:f>", "</c:f>"))
            .map(|s| s.to_string())
            .unwrap_or_default();

        series.push(ChartSeriesInfo {
            name: name_literal.map(|s| s.to_string()),
            name_ref: name_ref.map(|s| s.to_string()),
            values_ref,
        });

        cursor = &after[end + "</c:ser>".len()..];
    }

    ParsedChart {
        kind,
        title,
        legend,
        categories_ref,
        series,
    }
}

fn extract_first_between<'a>(haystack: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = haystack.find(open)? + open.len();
    let rest = &haystack[start..];
    let end = rest.find(close)?;
    Some(&rest[..end])
}

fn extract_first_attr<'a>(haystack: &'a str, tag: &str, attr_prefix: &str) -> Option<&'a str> {
    let pos = haystack.find(tag)?;
    let after = &haystack[pos..];
    let close = after.find('>')?;
    let inside = &after[..close];
    let attr_start = inside.find(attr_prefix)? + attr_prefix.len();
    let after_attr = &inside[attr_start..];
    let end = after_attr.find('"')?;
    Some(&after_attr[..end])
}

fn build_two_cell_anchor_xml(
    anchor: &ChartAnchor,
    chart_name: &str,
    chart_index: usize,
    chart_rid: &str,
) -> String {
    let from_col_off = anchor.from_column_offset_emu.unwrap_or(0);
    let from_row_off = anchor.from_row_offset_emu.unwrap_or(0);
    let to_col_off = anchor.to_column_offset_emu.unwrap_or(0);
    let to_row_off = anchor.to_row_offset_emu.unwrap_or(0);
    let cnv_id = chart_index as u32 + 1;
    let chart_name_esc = escape_xml(chart_name);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<xdr:twoCellAnchor \
xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<xdr:from><xdr:col>{from_col}</xdr:col><xdr:colOff>{from_col_off}</xdr:colOff>\
<xdr:row>{from_row}</xdr:row><xdr:rowOff>{from_row_off}</xdr:rowOff></xdr:from>\
<xdr:to><xdr:col>{to_col}</xdr:col><xdr:colOff>{to_col_off}</xdr:colOff>\
<xdr:row>{to_row}</xdr:row><xdr:rowOff>{to_row_off}</xdr:rowOff></xdr:to>\
<xdr:graphicFrame macro=\"\">\
<xdr:nvGraphicFramePr>\
<xdr:cNvPr id=\"{cnv_id}\" name=\"{chart_name_esc}\"/>\
<xdr:cNvGraphicFramePr/>\
</xdr:nvGraphicFramePr>\
<xdr:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/></xdr:xfrm>\
<a:graphic><a:graphicData uri=\"{uri}\">\
<c:chart xmlns:c=\"{uri}\" r:id=\"{rid}\"/>\
</a:graphicData></a:graphic>\
</xdr:graphicFrame>\
<xdr:clientData/>\
</xdr:twoCellAnchor>",
        from_col = anchor.from_column,
        from_row = anchor.from_row,
        to_col = anchor.to_column,
        to_row = anchor.to_row,
        uri = CHART_GRAPHIC_DATA_URI,
        rid = chart_rid,
    )
}

fn build_chart_space_xml(patch: &ChartPatch) -> String {
    let title_xml = match patch.title.as_deref() {
        Some(text) if !text.is_empty() => format!(
            "<c:title><c:tx><c:rich>\
<a:bodyPr rot=\"0\" spcFirstLastPara=\"1\" vertOverflow=\"ellipsis\" wrap=\"square\" anchor=\"ctr\" anchorCtr=\"1\"/>\
<a:lstStyle/>\
<a:p><a:r><a:rPr lang=\"en-US\"/><a:t>{}</a:t></a:r></a:p>\
</c:rich></c:tx><c:overlay val=\"0\"/></c:title>\
<c:autoTitleDeleted val=\"0\"/>",
            escape_xml(text)
        ),
        _ => "<c:autoTitleDeleted val=\"1\"/>".to_string(),
    };

    let legend_xml = match patch.legend_position {
        Some(ChartLegendPosition::None) => String::new(),
        Some(pos) => {
            let val = legend_pos_val(pos);
            format!(
                "<c:legend><c:legendPos val=\"{val}\"/><c:overlay val=\"0\"/></c:legend>"
            )
        }
        None => {
            "<c:legend><c:legendPos val=\"r\"/><c:overlay val=\"0\"/></c:legend>".to_string()
        }
    };

    let plot_inner = match patch.kind {
        ChartKind::Pie => build_pie_chart_xml(patch),
        ChartKind::Line => build_line_chart_xml(patch),
        ChartKind::Area => build_area_chart_xml(patch),
        ChartKind::Column | ChartKind::Bar => build_bar_chart_xml(patch),
    };

    let axes_xml = match patch.kind {
        ChartKind::Pie => String::new(),
        _ => build_axes_xml(),
    };

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<c:chartSpace \
xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" \
xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<c:chart>{title_xml}<c:plotArea><c:layout/>{plot_inner}{axes_xml}</c:plotArea>{legend_xml}\
<c:plotVisOnly val=\"1\"/><c:dispBlanksAs val=\"gap\"/></c:chart></c:chartSpace>"
    )
}

fn legend_pos_val(pos: ChartLegendPosition) -> &'static str {
    match pos {
        ChartLegendPosition::Right => "r",
        ChartLegendPosition::Left => "l",
        ChartLegendPosition::Top => "t",
        ChartLegendPosition::Bottom => "b",
        ChartLegendPosition::TopRight => "tr",
        ChartLegendPosition::None => "r",
    }
}

fn build_bar_chart_xml(patch: &ChartPatch) -> String {
    let bar_dir = if matches!(patch.kind, ChartKind::Bar) {
        "bar"
    } else {
        "col"
    };
    let series = patch
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| build_series_xml(i, s, patch.categories_ref.as_deref()))
        .collect::<String>();
    format!(
        "<c:barChart><c:barDir val=\"{bar_dir}\"/><c:grouping val=\"clustered\"/>\
<c:varyColors val=\"0\"/>{series}<c:axId val=\"111111111\"/><c:axId val=\"222222222\"/></c:barChart>"
    )
}

fn build_line_chart_xml(patch: &ChartPatch) -> String {
    let series = patch
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| build_series_xml(i, s, patch.categories_ref.as_deref()))
        .collect::<String>();
    format!(
        "<c:lineChart><c:grouping val=\"standard\"/><c:varyColors val=\"0\"/>{series}\
<c:marker val=\"1\"/><c:axId val=\"111111111\"/><c:axId val=\"222222222\"/></c:lineChart>"
    )
}

fn build_area_chart_xml(patch: &ChartPatch) -> String {
    let series = patch
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| build_series_xml(i, s, patch.categories_ref.as_deref()))
        .collect::<String>();
    format!(
        "<c:areaChart><c:grouping val=\"standard\"/><c:varyColors val=\"0\"/>{series}\
<c:axId val=\"111111111\"/><c:axId val=\"222222222\"/></c:areaChart>"
    )
}

fn build_pie_chart_xml(patch: &ChartPatch) -> String {
    let series = patch
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| build_series_xml(i, s, patch.categories_ref.as_deref()))
        .collect::<String>();
    format!("<c:pieChart><c:varyColors val=\"1\"/>{series}</c:pieChart>")
}

fn build_series_xml(idx: usize, series: &ChartSeriesPatch, categories_ref: Option<&str>) -> String {
    let tx = if let Some(r) = series.name_ref.as_deref() {
        format!(
            "<c:tx><c:strRef><c:f>{}</c:f></c:strRef></c:tx>",
            escape_xml(r)
        )
    } else if let Some(name) = series.name.as_deref() {
        format!("<c:tx><c:v>{}</c:v></c:tx>", escape_xml(name))
    } else {
        String::new()
    };
    let cat = match categories_ref {
        Some(r) if !r.is_empty() => format!(
            "<c:cat><c:strRef><c:f>{}</c:f></c:strRef></c:cat>",
            escape_xml(r)
        ),
        _ => String::new(),
    };
    format!(
        "<c:ser><c:idx val=\"{idx}\"/><c:order val=\"{idx}\"/>{tx}{cat}\
<c:val><c:numRef><c:f>{val}</c:f></c:numRef></c:val></c:ser>",
        val = escape_xml(&series.values_ref)
    )
}

fn build_axes_xml() -> String {
    "<c:catAx><c:axId val=\"111111111\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling>\
<c:delete val=\"0\"/><c:axPos val=\"b\"/><c:crossAx val=\"222222222\"/></c:catAx>\
<c:valAx><c:axId val=\"222222222\"/><c:scaling><c:orientation val=\"minMax\"/></c:scaling>\
<c:delete val=\"0\"/><c:axPos val=\"l\"/><c:crossAx val=\"111111111\"/></c:valAx>"
        .to_string()
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[allow(dead_code)]
fn quote_sheet(name: &str) -> String {
    quote_sheet_name(name)
}
