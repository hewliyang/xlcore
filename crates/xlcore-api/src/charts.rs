use ooxmlsdk::parts::chart_part::ChartPart;
use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use ooxmlsdk::simple_type::{BooleanValue, CoordinateValue};
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, ApiWarning, ChartAnchor, ChartDataLabelPosition, ChartDataLabels,
    ChartInfo, ChartKind, ChartLegendPosition, ChartPatch, ChartSeriesInfo, ChartSeriesPatch,
    ChartStacking,
};

use crate::errors::sdk_err_to_api;
use crate::refs::quote_sheet_name;
use crate::{Result, Workbook};

const CHART_GRAPHIC_DATA_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CAT_AX_ID: u32 = 111_111_111;
const VAL_AX_ID: u32 = 222_222_222;

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
                let space = chart_part
                    .root_element(&mut self.doc)
                    .map_err(sdk_err_to_api)?
                    .clone();
                let parsed = read_chart_space(&space);
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
                    category_axis_title: parsed.category_axis_title,
                    value_axis_title: parsed.value_axis_title,
                    stacking: parsed.stacking,
                    data_labels: parsed.data_labels,
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
            if matches!(patch.kind, ChartKind::Scatter | ChartKind::Bubble)
                && s.x_values_ref.as_deref().map(|v| v.trim().is_empty()).unwrap_or(true)
            {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidChart,
                    "scatter/bubble chart series require x_values_ref",
                )
                .with_sheet(&patch.sheet));
            }
            if matches!(patch.kind, ChartKind::Bubble)
                && s.bubble_sizes_ref.as_deref().map(|v| v.trim().is_empty()).unwrap_or(true)
            {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidChart,
                    "bubble chart series require bubble_sizes_ref",
                )
                .with_sheet(&patch.sheet));
            }
            if let Some(color) = s.color.as_deref() {
                if !is_valid_hex_color(color) {
                    return Err(ApiError::new(
                        ApiErrorCode::InvalidChart,
                        format!("chart series color must be 6-hex RRGGBB, got: {color}"),
                    )
                    .with_sheet(&patch.sheet));
                }
            }
        }

        if !self.sheet_exists(&patch.sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {}", patch.sheet),
            )
            .with_sheet(&patch.sheet));
        }

        if patch.stacking.is_some()
            && !matches!(
                patch.kind,
                ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area
            )
        {
            self.push_warning(
                ApiWarning::new(
                    ApiErrorCode::LossyOperation,
                    format!(
                        "stacking is not supported on {:?} charts; ignored",
                        patch.kind
                    ),
                )
                .with_sheet(&patch.sheet),
            );
        }

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

        let chart_space = build_chart_space(&patch);
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

        let new_anchor = build_two_cell_anchor(&patch.anchor, &chart_name, chart_index, &chart_rid);

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
                    x_values_ref: s.x_values_ref.clone(),
                    bubble_sizes_ref: s.bubble_sizes_ref.clone(),
                    color: s.color.clone(),
                    data_labels: s.data_labels.clone(),
                })
                .collect(),
            anchor: patch.anchor,
            category_axis_title: patch.category_axis_title.clone(),
            value_axis_title: patch.value_axis_title.clone(),
            stacking: stacking_for_kind(patch.kind, patch.stacking),
            data_labels: patch.data_labels.clone(),
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
    category_axis_title: Option<String>,
    value_axis_title: Option<String>,
    stacking: Option<ChartStacking>,
    data_labels: Option<ChartDataLabels>,
}

fn read_chart_space(space: &c::ChartSpace) -> ParsedChart {
    let plot = &space.chart.plot_area;

    let mut kind = ChartKind::Column;
    let mut series: Vec<ChartSeriesInfo> = Vec::new();
    let mut categories_ref: Option<String> = None;
    let mut stacking: Option<ChartStacking> = None;
    let mut data_labels: Option<ChartDataLabels> = None;

    for ch in &plot.plot_area_choice1 {
        match ch {
            c::PlotAreaChoice::BarChart(bc) => {
                kind = match bc.bar_direction.val {
                    c::BarDirectionValues::Bar => ChartKind::Bar,
                    c::BarDirectionValues::Column => ChartKind::Column,
                };
                stacking = bc
                    .bar_grouping
                    .as_ref()
                    .and_then(|g| g.val.as_ref())
                    .map(|v| match v {
                        c::BarGroupingValues::Clustered => ChartStacking::Clustered,
                        c::BarGroupingValues::Stacked => ChartStacking::Stacked,
                        c::BarGroupingValues::PercentStacked => ChartStacking::PercentStacked,
                        c::BarGroupingValues::Standard => ChartStacking::Clustered,
                    });
                for s in &bc.bar_chart_series {
                    series.push(read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    ));
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(bc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::LineChart(lc) => {
                kind = ChartKind::Line;
                stacking = lc.grouping.val.as_ref().map(grouping_to_stacking);
                for s in &lc.line_chart_series {
                    series.push(read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    ));
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(lc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::PieChart(pc) => {
                kind = ChartKind::Pie;
                for s in &pc.pie_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(pc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::DoughnutChart(dc) => {
                kind = ChartKind::Doughnut;
                for s in &dc.pie_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(dc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::AreaChart(ac) => {
                kind = ChartKind::Area;
                stacking = ac
                    .grouping
                    .as_ref()
                    .and_then(|g| g.val.as_ref())
                    .map(grouping_to_stacking);
                for s in &ac.area_chart_series {
                    series.push(read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    ));
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(ac.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::ScatterChart(sc) => {
                kind = ChartKind::Scatter;
                for s in &sc.scatter_chart_series {
                    let mut info = read_xy_series(
                        s.series_text.as_deref(),
                        s.x_values.as_deref().and_then(|x| x.x_values_choice.as_ref()),
                        s.y_values.as_deref().and_then(|y| y.y_values_choice.as_ref()),
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(sc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::BubbleChart(bc) => {
                kind = ChartKind::Bubble;
                for s in &bc.bubble_chart_series {
                    let mut info = read_xy_series(
                        s.series_text.as_deref(),
                        s.x_values.as_deref().and_then(|x| x.x_values_choice.as_ref()),
                        s.y_values.as_deref().and_then(|y| y.y_values_choice.as_ref()),
                        s.data_labels.as_deref(),
                    );
                    info.bubble_sizes_ref =
                        s.bubble_size.as_deref().and_then(|b| match b.bubble_size_choice.as_ref()? {
                            c::BubbleSizeChoice::NumberReference(nr) => Some(nr.formula.clone()),
                            _ => None,
                        });
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(bc.data_labels.as_deref());
                }
            }
            _ => {}
        }
    }

    let mut category_axis_title: Option<String> = None;
    let mut value_axis_title: Option<String> = None;
    for ax in &plot.plot_area_choice2 {
        match ax {
            c::PlotAreaChoice2::CategoryAxis(c) => {
                if let Some(t) = c.title.as_deref() {
                    category_axis_title = extract_title_text(t);
                }
            }
            c::PlotAreaChoice2::ValueAxis(v) => {
                if let Some(t) = v.title.as_deref() {
                    if v.axis_position.val == c::AxisPositionValues::Bottom
                        && category_axis_title.is_none()
                    {
                        category_axis_title = extract_title_text(t);
                    } else if value_axis_title.is_none() {
                        value_axis_title = extract_title_text(t);
                    }
                }
            }
            _ => {}
        }
    }

    let title = space.chart.title.as_ref().and_then(|t| extract_title_text(t));

    let legend = space.chart.legend.as_ref().and_then(|l| {
        l.legend_position
            .as_ref()
            .and_then(|p| p.val.as_ref())
            .map(legend_pos_from)
    });

    ParsedChart {
        kind,
        title,
        legend,
        categories_ref,
        series,
        category_axis_title,
        value_axis_title,
        stacking,
        data_labels,
    }
}

fn grouping_to_stacking(v: &c::GroupingValues) -> ChartStacking {
    match v {
        c::GroupingValues::Standard => ChartStacking::Clustered,
        c::GroupingValues::Stacked => ChartStacking::Stacked,
        c::GroupingValues::PercentStacked => ChartStacking::PercentStacked,
    }
}

fn stacking_for_kind(kind: ChartKind, requested: Option<ChartStacking>) -> Option<ChartStacking> {
    match kind {
        ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area => requested,
        _ => None,
    }
}

fn line_area_grouping(stacking: Option<ChartStacking>) -> c::GroupingValues {
    match stacking {
        Some(ChartStacking::Stacked) => c::GroupingValues::Stacked,
        Some(ChartStacking::PercentStacked) => c::GroupingValues::PercentStacked,
        _ => c::GroupingValues::Standard,
    }
}

fn bar_grouping(stacking: Option<ChartStacking>) -> c::BarGroupingValues {
    match stacking {
        Some(ChartStacking::Stacked) => c::BarGroupingValues::Stacked,
        Some(ChartStacking::PercentStacked) => c::BarGroupingValues::PercentStacked,
        _ => c::BarGroupingValues::Clustered,
    }
}

fn read_xy_series(
    tx: Option<&c::SeriesText>,
    x_choice: Option<&c::XValuesChoice>,
    y_choice: Option<&c::YValuesChoice>,
    dl: Option<&c::DataLabels>,
) -> ChartSeriesInfo {
    let (name, name_ref) = match tx.and_then(|t| t.series_text_choice.as_ref()) {
        Some(c::SeriesTextChoice::StringReference(sr)) => (None, Some(sr.formula.clone())),
        Some(c::SeriesTextChoice::NumericValue(nv)) => (Some(nv.clone()), None),
        None => (None, None),
    };
    let x_values_ref = x_choice.and_then(|c| match c {
        c::XValuesChoice::NumberReference(nr) => Some(nr.formula.clone()),
        c::XValuesChoice::StringReference(sr) => Some(sr.formula.clone()),
        _ => None,
    });
    let values_ref = y_choice
        .and_then(|c| match c {
            c::YValuesChoice::NumberReference(nr) => Some(nr.formula.clone()),
            _ => None,
        })
        .unwrap_or_default();
    ChartSeriesInfo {
        name,
        name_ref,
        values_ref,
        x_values_ref,
        bubble_sizes_ref: None,
        color: None,
        data_labels: read_data_labels(dl),
    }
}

fn read_series_color(sp: Option<&c::ChartShapeProperties>) -> Option<String> {
    let sp = sp?;
    let choice = sp.chart_shape_properties_choice2.as_ref()?;
    let c::ChartShapePropertiesChoice2::SolidFill(sf) = choice else { return None };
    let inner = sf.solid_fill_choice.as_ref()?;
    let a::SolidFillChoice::RgbColorModelHex(rgb) = inner else { return None };
    Some(rgb.val.to_string().to_uppercase())
}

fn read_series(
    tx: Option<&c::SeriesText>,
    cat: Option<&c::CategoryAxisData>,
    val: Option<&c::Values>,
    categories_ref: &mut Option<String>,
    dl: Option<&c::DataLabels>,
) -> ChartSeriesInfo {
    let (name, name_ref) = match tx.and_then(|t| t.series_text_choice.as_ref()) {
        Some(c::SeriesTextChoice::StringReference(sr)) => (None, Some(sr.formula.clone())),
        Some(c::SeriesTextChoice::NumericValue(nv)) => (Some(nv.clone()), None),
        None => (None, None),
    };
    if categories_ref.is_none() {
        if let Some(cat_data) = cat {
            if let Some(c::CategoryAxisDataChoice::StringReference(sr)) =
                cat_data.category_axis_data_choice.as_ref()
            {
                *categories_ref = Some(sr.formula.clone());
            } else if let Some(c::CategoryAxisDataChoice::NumberReference(nr)) =
                cat_data.category_axis_data_choice.as_ref()
            {
                *categories_ref = Some(nr.formula.clone());
            }
        }
    }
    let values_ref = val
        .and_then(|v| v.values_choice.as_ref())
        .and_then(|choice| match choice {
            c::ValuesChoice::NumberReference(nr) => Some(nr.formula.clone()),
            c::ValuesChoice::NumberLiteral(_) => None,
        })
        .unwrap_or_default();
    ChartSeriesInfo {
        name,
        name_ref,
        values_ref,
        x_values_ref: None,
        bubble_sizes_ref: None,
        color: None,
        data_labels: read_data_labels(dl),
    }
}

fn extract_title_text(title: &c::Title) -> Option<String> {
    let choice = title.chart_text.as_ref()?.chart_text_choice.as_ref()?;
    match choice {
        c::ChartTextChoice::RichText(rt) => rt.paragraph.iter().find_map(|p| {
            p.paragraph_choice.iter().find_map(|pc| match pc {
                a::ParagraphChoice::Run(r) => Some(r.text.clone()),
                _ => None,
            })
        }),
        c::ChartTextChoice::StringReference(sr) => Some(sr.formula.clone()),
        c::ChartTextChoice::StringLiteral(sl) => sl
            .string_point
            .first()
            .map(|p| p.numeric_value.clone()),
    }
}

fn legend_pos_from(v: &c::LegendPositionValues) -> ChartLegendPosition {
    match v {
        c::LegendPositionValues::Right => ChartLegendPosition::Right,
        c::LegendPositionValues::Left => ChartLegendPosition::Left,
        c::LegendPositionValues::Top => ChartLegendPosition::Top,
        c::LegendPositionValues::Bottom => ChartLegendPosition::Bottom,
        c::LegendPositionValues::TopRight => ChartLegendPosition::TopRight,
    }
}

fn legend_pos_to(p: ChartLegendPosition) -> c::LegendPositionValues {
    match p {
        ChartLegendPosition::Right => c::LegendPositionValues::Right,
        ChartLegendPosition::Left => c::LegendPositionValues::Left,
        ChartLegendPosition::Top => c::LegendPositionValues::Top,
        ChartLegendPosition::Bottom => c::LegendPositionValues::Bottom,
        ChartLegendPosition::TopRight => c::LegendPositionValues::TopRight,
        ChartLegendPosition::None => c::LegendPositionValues::Right,
    }
}

fn build_chart_space(patch: &ChartPatch) -> c::ChartSpace {
    let plot_chart = build_plot_chart(patch);

    let mut plot_area = c::PlotArea {
        layout: Some(Box::new(c::Layout::default())),
        plot_area_choice1: vec![plot_chart],
        plot_area_choice2: Vec::new(),
        ..Default::default()
    };

    match patch.kind {
        ChartKind::Pie | ChartKind::Doughnut => {}
        ChartKind::Scatter | ChartKind::Bubble => {
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::ValueAxis(Box::new(build_val_axis_xy(
                    CAT_AX_ID,
                    VAL_AX_ID,
                    c::AxisPositionValues::Bottom,
                    patch.category_axis_title.as_deref(),
                ))));
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::ValueAxis(Box::new(build_val_axis_xy(
                    VAL_AX_ID,
                    CAT_AX_ID,
                    c::AxisPositionValues::Left,
                    patch.value_axis_title.as_deref(),
                ))));
        }
        _ => {
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::CategoryAxis(Box::new(build_cat_axis(
                    patch.category_axis_title.as_deref(),
                ))));
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::ValueAxis(Box::new(build_val_axis(
                    patch.value_axis_title.as_deref(),
                ))));
        }
    }

    let title = patch
        .title
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(build_title);

    let auto_title_deleted = if title.is_none() {
        Some(c::AutoTitleDeleted {
            val: Some(BooleanValue::from_bool(true)),
        })
    } else {
        Some(c::AutoTitleDeleted {
            val: Some(BooleanValue::from_bool(false)),
        })
    };

    let legend = match patch.legend_position {
        Some(ChartLegendPosition::None) => None,
        Some(pos) => Some(Box::new(build_legend(legend_pos_to(pos)))),
        None => Some(Box::new(build_legend(c::LegendPositionValues::Right))),
    };

    let chart = c::Chart {
        title: title.map(Box::new),
        auto_title_deleted,
        plot_area: Box::new(plot_area),
        legend,
        plot_visible_only: Some(c::PlotVisibleOnly {
            val: Some(BooleanValue::from_bool(true)),
        }),
        display_blanks_as: Some(c::DisplayBlanksAs {
            val: Some(c::DisplayBlanksAsValues::Gap),
        }),
        ..Default::default()
    };

    c::ChartSpace {
        xmlns: crate::ooxml_header::chart_space(),
        xml_header: crate::ooxml_header::STANDALONE,
        chart: Box::new(chart),
        ..Default::default()
    }
}

fn build_plot_chart(patch: &ChartPatch) -> c::PlotAreaChoice {
    let dl_ref = patch.data_labels.as_ref();
    let dl = build_data_labels(dl_ref);
    match patch.kind {
        ChartKind::Pie => c::PlotAreaChoice::PieChart(Box::new(c::PieChart {
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(true)),
            }),
            pie_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_pie_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            ..Default::default()
        })),
        ChartKind::Doughnut => c::PlotAreaChoice::DoughnutChart(Box::new(c::DoughnutChart {
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(true)),
            }),
            pie_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_pie_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            hole_size: Box::new(c::HoleSize { val: 50 }),
            ..Default::default()
        })),
        ChartKind::Scatter => c::PlotAreaChoice::ScatterChart(Box::new(c::ScatterChart {
            scatter_style: Box::new(c::ScatterStyle {
                val: Some(c::ScatterStyleValues::LineMarker),
            }),
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(false)),
            }),
            scatter_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_scatter_series(i, s))
                .collect(),
            data_labels: dl,
            axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
            ..Default::default()
        })),
        ChartKind::Bubble => c::PlotAreaChoice::BubbleChart(Box::new(c::BubbleChart {
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(true)),
            }),
            bubble_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_bubble_series(i, s))
                .collect(),
            data_labels: dl,
            axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
            ..Default::default()
        })),
        ChartKind::Line => c::PlotAreaChoice::LineChart(Box::new(c::LineChart {
            grouping: Box::new(c::Grouping {
                val: Some(line_area_grouping(patch.stacking)),
            }),
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(false)),
            }),
            line_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_line_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            show_marker: Some(c::ShowMarker {
                val: Some(BooleanValue::from_bool(true)),
            }),
            axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
            ..Default::default()
        })),
        ChartKind::Area => c::PlotAreaChoice::AreaChart(Box::new(c::AreaChart {
            grouping: Some(c::Grouping {
                val: Some(line_area_grouping(patch.stacking)),
            }),
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(false)),
            }),
            area_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_area_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
            ..Default::default()
        })),
        ChartKind::Column | ChartKind::Bar => {
            c::PlotAreaChoice::BarChart(Box::new(c::BarChart {
                bar_direction: Box::new(c::BarDirection {
                    val: if matches!(patch.kind, ChartKind::Bar) {
                        c::BarDirectionValues::Bar
                    } else {
                        c::BarDirectionValues::Column
                    },
                }),
                bar_grouping: Some(c::BarGrouping {
                    val: Some(bar_grouping(patch.stacking)),
                }),
                vary_colors: Some(c::VaryColors {
                    val: Some(BooleanValue::from_bool(false)),
                }),
                bar_chart_series: patch
                    .series
                    .iter()
                    .enumerate()
                    .map(|(i, s)| build_bar_series(i, s, patch.categories_ref.as_deref()))
                    .collect(),
                data_labels: dl,
                overlap: matches!(
                    patch.stacking,
                    Some(ChartStacking::Stacked | ChartStacking::PercentStacked)
                )
                .then(|| c::Overlap { val: Some(100) }),
                axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
                ..Default::default()
            }))
        }
    }
}

fn build_data_labels(dl: Option<&ChartDataLabels>) -> Option<Box<c::DataLabels>> {
    let dl = dl?;
    fn b(v: Option<bool>) -> Option<BooleanValue> {
        v.map(BooleanValue::from_bool)
    }
    let seq = c::DataLabelsChoiceSequence {
        data_label_position: dl.position.map(|p| c::DataLabelPosition {
            val: data_label_pos_to(p),
        }),
        show_legend_key: Some(c::ShowLegendKey {
            val: b(dl.show_legend_key).or(Some(BooleanValue::from_bool(false))),
        }),
        show_value: Some(c::ShowValue {
            val: b(dl.show_value).or(Some(BooleanValue::from_bool(false))),
        }),
        show_category_name: Some(c::ShowCategoryName {
            val: b(dl.show_category_name).or(Some(BooleanValue::from_bool(false))),
        }),
        show_series_name: Some(c::ShowSeriesName {
            val: b(dl.show_series_name).or(Some(BooleanValue::from_bool(false))),
        }),
        show_percent: Some(c::ShowPercent {
            val: b(dl.show_percent).or(Some(BooleanValue::from_bool(false))),
        }),
        show_bubble_size: Some(c::ShowBubbleSize {
            val: Some(BooleanValue::from_bool(false)),
        }),
        separator: dl.separator.clone(),
        ..Default::default()
    };
    Some(Box::new(c::DataLabels {
        data_labels_choice: Some(c::DataLabelsChoice::Sequence(Box::new(seq))),
        ..Default::default()
    }))
}

fn data_label_pos_to(p: ChartDataLabelPosition) -> c::DataLabelPositionValues {
    match p {
        ChartDataLabelPosition::Center => c::DataLabelPositionValues::Center,
        ChartDataLabelPosition::InsideEnd => c::DataLabelPositionValues::InsideEnd,
        ChartDataLabelPosition::InsideBase => c::DataLabelPositionValues::InsideBase,
        ChartDataLabelPosition::OutsideEnd => c::DataLabelPositionValues::OutsideEnd,
        ChartDataLabelPosition::Top => c::DataLabelPositionValues::Top,
        ChartDataLabelPosition::Bottom => c::DataLabelPositionValues::Bottom,
        ChartDataLabelPosition::Left => c::DataLabelPositionValues::Left,
        ChartDataLabelPosition::Right => c::DataLabelPositionValues::Right,
        ChartDataLabelPosition::BestFit => c::DataLabelPositionValues::BestFit,
    }
}

fn data_label_pos_from(v: &c::DataLabelPositionValues) -> ChartDataLabelPosition {
    match v {
        c::DataLabelPositionValues::Center => ChartDataLabelPosition::Center,
        c::DataLabelPositionValues::InsideEnd => ChartDataLabelPosition::InsideEnd,
        c::DataLabelPositionValues::InsideBase => ChartDataLabelPosition::InsideBase,
        c::DataLabelPositionValues::OutsideEnd => ChartDataLabelPosition::OutsideEnd,
        c::DataLabelPositionValues::Top => ChartDataLabelPosition::Top,
        c::DataLabelPositionValues::Bottom => ChartDataLabelPosition::Bottom,
        c::DataLabelPositionValues::Left => ChartDataLabelPosition::Left,
        c::DataLabelPositionValues::Right => ChartDataLabelPosition::Right,
        c::DataLabelPositionValues::BestFit => ChartDataLabelPosition::BestFit,
    }
}

fn read_data_labels(dl: Option<&c::DataLabels>) -> Option<ChartDataLabels> {
    let dl = dl?;
    let choice = dl.data_labels_choice.as_ref()?;
    let seq = match choice {
        c::DataLabelsChoice::Sequence(s) => s,
        _ => return None,
    };
    let bv = |b: Option<&BooleanValue>| b.map(|v| bool::from(*v));
    let out = ChartDataLabels {
        show_value: bv(seq.show_value.as_ref().and_then(|s| s.val.as_ref())),
        show_category_name: bv(
            seq.show_category_name.as_ref().and_then(|s| s.val.as_ref()),
        ),
        show_series_name: bv(seq.show_series_name.as_ref().and_then(|s| s.val.as_ref())),
        show_percent: bv(seq.show_percent.as_ref().and_then(|s| s.val.as_ref())),
        show_legend_key: bv(seq.show_legend_key.as_ref().and_then(|s| s.val.as_ref())),
        position: seq.data_label_position.as_ref().map(|p| data_label_pos_from(&p.val)),
        separator: seq.separator.clone(),
    };
    if out == ChartDataLabels::default() {
        None
    } else {
        Some(out)
    }
}

fn axis_id(val: u32) -> c::AxisId {
    c::AxisId { val }
}

fn build_series_text(s: &ChartSeriesPatch) -> Option<Box<c::SeriesText>> {
    if let Some(r) = s.name_ref.as_deref() {
        Some(Box::new(c::SeriesText {
            series_text_choice: Some(c::SeriesTextChoice::StringReference(Box::new(
                c::StringReference {
                    formula: r.to_string(),
                    ..Default::default()
                },
            ))),
        }))
    } else if let Some(name) = s.name.as_deref() {
        Some(Box::new(c::SeriesText {
            series_text_choice: Some(c::SeriesTextChoice::NumericValue(name.to_string())),
        }))
    } else {
        None
    }
}

fn build_categories(categories_ref: Option<&str>) -> Option<Box<c::CategoryAxisData>> {
    let r = categories_ref?;
    if r.is_empty() {
        return None;
    }
    Some(Box::new(c::CategoryAxisData {
        category_axis_data_choice: Some(c::CategoryAxisDataChoice::StringReference(Box::new(
            c::StringReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    }))
}

fn build_values(values_ref: &str) -> Box<c::Values> {
    Box::new(c::Values {
        values_choice: Some(c::ValuesChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: values_ref.to_string(),
                ..Default::default()
            },
        ))),
    })
}

fn build_bar_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::BarChartSeries {
    c::BarChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

fn build_line_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::LineChartSeries {
    c::LineChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

fn build_area_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::AreaChartSeries {
    c::AreaChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

fn build_pie_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::PieChartSeries {
    c::PieChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

fn build_scatter_series(
    idx: usize,
    s: &ChartSeriesPatch,
) -> c::ScatterChartSeries {
    c::ScatterChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        x_values: s.x_values_ref.as_deref().map(build_x_values),
        y_values: Some(build_y_values(&s.values_ref)),
        smooth: Some(c::Smooth {
            val: Some(BooleanValue::from_bool(false)),
        }),
        ..Default::default()
    }
}

fn build_bubble_series(
    idx: usize,
    s: &ChartSeriesPatch,
) -> c::BubbleChartSeries {
    c::BubbleChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        x_values: s.x_values_ref.as_deref().map(build_x_values),
        y_values: Some(build_y_values(&s.values_ref)),
        bubble_size: s.bubble_sizes_ref.as_deref().map(build_bubble_size),
        ..Default::default()
    }
}

fn build_x_values(r: &str) -> Box<c::XValues> {
    Box::new(c::XValues {
        x_values_choice: Some(c::XValuesChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    })
}

fn build_y_values(r: &str) -> Box<c::YValues> {
    Box::new(c::YValues {
        y_values_choice: Some(c::YValuesChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    })
}

fn build_bubble_size(r: &str) -> Box<c::BubbleSize> {
    Box::new(c::BubbleSize {
        bubble_size_choice: Some(c::BubbleSizeChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    })
}

fn build_series_shape(color: Option<&str>) -> Option<Box<c::ChartShapeProperties>> {
    let hex = color?;
    let solid = a::SolidFill {
        solid_fill_choice: Some(a::SolidFillChoice::RgbColorModelHex(Box::new(
            a::RgbColorModelHex {
                val: hex.trim_start_matches('#').to_uppercase(),
                ..Default::default()
            },
        ))),
        ..Default::default()
    };
    Some(Box::new(c::ChartShapeProperties {
        chart_shape_properties_choice2: Some(c::ChartShapePropertiesChoice2::SolidFill(
            Box::new(solid),
        )),
        ..Default::default()
    }))
}

fn is_valid_hex_color(s: &str) -> bool {
    let s = s.trim_start_matches('#');
    s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn build_title(text: &str) -> c::Title {
    let run = a::Run {
        run_properties: Some(Box::new(a::RunProperties {
            language: Some("en-US".to_string()),
            ..Default::default()
        })),
        text: text.to_string(),
        ..Default::default()
    };
    let paragraph = a::Paragraph {
        paragraph_choice: vec![a::ParagraphChoice::Run(Box::new(run))],
        ..Default::default()
    };
    let rich = c::RichText {
        body_properties: Box::new(a::BodyProperties {
            rotation: Some(0),
            use_paragraph_spacing: Some(BooleanValue::from_bool(true)),
            vertical_overflow: Some(a::TextVerticalOverflowValues::Ellipsis),
            wrap: Some(a::TextWrappingValues::Square),
            anchor: Some(a::TextAnchoringTypeValues::Center),
            anchor_center: Some(BooleanValue::from_bool(true)),
            ..Default::default()
        }),
        list_style: Some(Box::new(a::ListStyle::default())),
        paragraph: vec![paragraph],
        ..Default::default()
    };
    c::Title {
        chart_text: Some(Box::new(c::ChartText {
            chart_text_choice: Some(c::ChartTextChoice::RichText(Box::new(rich))),
        })),
        overlay: Some(c::Overlay {
            val: Some(BooleanValue::from_bool(false)),
        }),
        ..Default::default()
    }
}

fn build_legend(pos: c::LegendPositionValues) -> c::Legend {
    c::Legend {
        legend_position: Some(c::LegendPosition { val: Some(pos) }),
        overlay: Some(c::Overlay {
            val: Some(BooleanValue::from_bool(false)),
        }),
        ..Default::default()
    }
}

fn build_cat_axis(title: Option<&str>) -> c::CategoryAxis {
    c::CategoryAxis {
        axis_id: Box::new(axis_id(CAT_AX_ID)),
        scaling: Box::new(c::Scaling {
            orientation: Some(c::Orientation {
                val: Some(c::OrientationValues::MinMax),
            }),
            ..Default::default()
        }),
        delete: Some(c::Delete {
            val: Some(BooleanValue::from_bool(false)),
        }),
        axis_position: Box::new(c::AxisPosition {
            val: c::AxisPositionValues::Bottom,
        }),
        title: title
            .filter(|t| !t.is_empty())
            .map(|t| Box::new(build_title(t))),
        crossing_axis: Box::new(c::CrossingAxis { val: VAL_AX_ID }),
        ..Default::default()
    }
}

fn build_val_axis(title: Option<&str>) -> c::ValueAxis {
    build_val_axis_xy(
        VAL_AX_ID,
        CAT_AX_ID,
        c::AxisPositionValues::Left,
        title,
    )
}

fn build_val_axis_xy(
    id: u32,
    cross: u32,
    pos: c::AxisPositionValues,
    title: Option<&str>,
) -> c::ValueAxis {
    c::ValueAxis {
        axis_id: Box::new(axis_id(id)),
        scaling: Box::new(c::Scaling {
            orientation: Some(c::Orientation {
                val: Some(c::OrientationValues::MinMax),
            }),
            ..Default::default()
        }),
        delete: Some(c::Delete {
            val: Some(BooleanValue::from_bool(false)),
        }),
        axis_position: Box::new(c::AxisPosition { val: pos }),
        title: title
            .filter(|t| !t.is_empty())
            .map(|t| Box::new(build_title(t))),
        crossing_axis: Box::new(c::CrossingAxis { val: cross }),
        ..Default::default()
    }
}

fn build_two_cell_anchor(
    anchor: &ChartAnchor,
    chart_name: &str,
    chart_index: usize,
    chart_rid: &str,
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
        id: chart_index as u32 + 1,
        name: chart_name.to_string(),
        ..Default::default()
    };
    let nv_props = xdr::NonVisualGraphicFrameProperties {
        non_visual_drawing_properties: Box::new(nv_drawing),
        non_visual_graphic_frame_drawing_properties: Box::new(
            xdr::NonVisualGraphicFrameDrawingProperties::default(),
        ),
    };

    let xfrm = xdr::Transform {
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

    let chart_ref = c::ChartReference {
        id: chart_rid.to_string(),
        ..Default::default()
    };

    let graphic_data = a::GraphicData {
        uri: CHART_GRAPHIC_DATA_URI.to_string(),
        graphic_data_choice: vec![a::GraphicDataChoice::ChartReference(Box::new(chart_ref))],
        ..Default::default()
    };

    let graphic = a::Graphic {
        graphic_data: Box::new(graphic_data),
        ..Default::default()
    };

    let graphic_frame = xdr::GraphicFrame {
        r#macro: Some(String::new()),
        non_visual_graphic_frame_properties: Box::new(nv_props),
        transform: Box::new(xfrm),
        graphic: Box::new(graphic),
        ..Default::default()
    };

    xdr::TwoCellAnchor {
        from_marker: Box::new(from),
        to_marker: Box::new(to),
        two_cell_anchor_choice: Some(xdr::TwoCellAnchorChoice::GraphicFrame(Box::new(
            graphic_frame,
        ))),
        client_data: Box::new(xdr::ClientData::default()),
        ..Default::default()
    }
}

#[allow(dead_code)]
fn quote_sheet(name: &str) -> String {
    quote_sheet_name(name)
}
