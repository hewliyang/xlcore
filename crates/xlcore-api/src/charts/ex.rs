use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::parts::extended_chart_part::ExtendedChartPart;
use ooxmlsdk::schemas::schemas_microsoft_com_office_drawing_2014_chartex as cx;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::simple_type::{BooleanValue, CoordinateValue};
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    ApiError, ApiErrorCode, ChartAnchor, ChartExInfo, ChartExKind, ChartExPatch,
    ChartExQuartileMethod, ChartExSeriesInfo, ChartLegendPosition,
};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

const CHARTEX_GRAPHIC_DATA_URI: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";
const CHARTEX_NS: &str = "http://schemas.microsoft.com/office/drawing/2014/chartex";
const RELATIONSHIPS_NS: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn layout_for(kind: ChartExKind) -> cx::SeriesLayout {
    match kind {
        ChartExKind::Waterfall => cx::SeriesLayout::Waterfall,
        ChartExKind::Funnel => cx::SeriesLayout::Funnel,
        ChartExKind::Treemap => cx::SeriesLayout::Treemap,
        ChartExKind::Sunburst => cx::SeriesLayout::Sunburst,
        ChartExKind::Histogram | ChartExKind::Pareto => cx::SeriesLayout::ClusteredColumn,
        ChartExKind::BoxWhisker => cx::SeriesLayout::BoxWhisker,
        ChartExKind::RegionMap => cx::SeriesLayout::RegionMap,
    }
}

fn num_dim_type(kind: ChartExKind) -> cx::NumericDimensionType {
    match kind {
        ChartExKind::Treemap | ChartExKind::Sunburst => cx::NumericDimensionType::Size,
        ChartExKind::RegionMap => cx::NumericDimensionType::ColorVal,
        _ => cx::NumericDimensionType::Val,
    }
}

fn is_cartesian(kind: ChartExKind) -> bool {
    matches!(
        kind,
        ChartExKind::Waterfall
            | ChartExKind::Funnel
            | ChartExKind::Histogram
            | ChartExKind::Pareto
            | ChartExKind::BoxWhisker
    )
}

fn formula(reference: &str) -> Box<cx::Formula> {
    Box::new(cx::Formula(cx::OpenXmlFormulaElement {
        dir: None,
        xml_content: Some(reference.to_string()),
    }))
}

fn num_dim(kind: ChartExKind, values_ref: &str) -> cx::NumericDimension {
    cx::NumericDimension {
        r#type: num_dim_type(kind),
        numeric_dimension_choice: Some(cx::NumericDimensionChoice::Sequence(Box::new(
            cx::NumericDimensionChoiceSequence {
                formula: formula(values_ref),
                nf_formula: None,
                numeric_level: Vec::new(),
            },
        ))),
    }
}

fn str_dim(categories_ref: &str) -> cx::StringDimension {
    cx::StringDimension {
        r#type: cx::StringDimensionType::Cat,
        string_dimension_choice: Some(cx::StringDimensionChoice::Sequence(Box::new(
            cx::StringDimensionChoiceSequence {
                formula: formula(categories_ref),
                nf_formula: None,
                string_level: Vec::new(),
            },
        ))),
    }
}

fn series_text(name: Option<&str>, name_ref: Option<&str>) -> Option<Box<cx::Text>> {
    let choice = match (name_ref, name) {
        (Some(r), Some(v)) => cx::TextDataChoice::Sequence(Box::new(cx::TextDataChoiceSequence {
            formula: formula(r),
            v_xsdstring: Some(v.to_string()),
        })),
        (Some(r), None) => cx::TextDataChoice::Sequence(Box::new(cx::TextDataChoiceSequence {
            formula: formula(r),
            v_xsdstring: None,
        })),
        (None, Some(v)) => cx::TextDataChoice::VXsdstring(v.to_string()),
        (None, None) => return None,
    };
    Some(Box::new(cx::Text {
        text_choice: Some(cx::TextChoice::TextData(Box::new(cx::TextData {
            text_data_choice: Some(choice),
        }))),
    }))
}

fn chart_title(title: &str) -> Box<cx::ChartTitle> {
    Box::new(cx::ChartTitle {
        pos: Some(cx::SidePos::T),
        align: Some(cx::PosAlign::Ctr),
        overlay: Some(BooleanValue::from_bool(false)),
        text: Some(Box::new(cx::Text {
            text_choice: Some(cx::TextChoice::TextData(Box::new(cx::TextData {
                text_data_choice: Some(cx::TextDataChoice::VXsdstring(title.to_string())),
            }))),
        })),
        ..Default::default()
    })
}

fn legend_side(pos: ChartLegendPosition) -> Option<cx::SidePos> {
    match pos {
        ChartLegendPosition::None => None,
        ChartLegendPosition::Left => Some(cx::SidePos::L),
        ChartLegendPosition::Top => Some(cx::SidePos::T),
        ChartLegendPosition::Bottom => Some(cx::SidePos::B),
        ChartLegendPosition::Right | ChartLegendPosition::TopRight => Some(cx::SidePos::R),
    }
}

fn cartesian_axes() -> Vec<cx::Axis> {
    vec![
        cx::Axis {
            id: 0,
            axis_choice: Some(cx::AxisChoice::CategoryAxisScaling(Box::new(
                cx::CategoryAxisScaling::default(),
            ))),
            tick_labels: Some(Box::new(cx::TickLabels::default())),
            ..Default::default()
        },
        cx::Axis {
            id: 1,
            axis_choice: Some(cx::AxisChoice::ValueAxisScaling(Box::new(
                cx::ValueAxisScaling::default(),
            ))),
            major_gridlines_gridlines: Some(Box::new(cx::MajorGridlinesGridlines::default())),
            tick_labels: Some(Box::new(cx::TickLabels::default())),
            ..Default::default()
        },
    ]
}

fn build_chart_ex_space(patch: &ChartExPatch) -> cx::ChartSpace {
    let kind = patch.kind;
    let layout = layout_for(kind);

    let mut data: Vec<cx::Data> = Vec::new();
    let mut series: Vec<cx::Series> = Vec::new();

    for (idx, s) in patch.series.iter().enumerate() {
        let mut choices: Vec<cx::DataChoice> = Vec::new();
        if let Some(cat) = patch.categories_ref.as_deref() {
            choices.push(cx::DataChoice::StringDimension(Box::new(str_dim(cat))));
        }
        choices.push(cx::DataChoice::NumericDimension(Box::new(num_dim(
            kind,
            &s.values_ref,
        ))));
        data.push(cx::Data {
            id: idx as u32,
            data_choice: choices,
            extension_list: None,
        });

        let layout_pr = build_layout_pr(patch, idx);
        series.push(cx::Series {
            layout_id: layout,
            text: series_text(s.name.as_deref(), s.name_ref.as_deref()),
            data_id: Some(cx::DataId { val: idx as u32 }),
            series_layout_properties: layout_pr,
            ..Default::default()
        });
    }

    if kind == ChartExKind::Pareto {
        series.push(cx::Series {
            layout_id: cx::SeriesLayout::ParetoLine,
            text: series_text(Some("Cumulative %"), None),
            data_id: Some(cx::DataId { val: 0 }),
            ..Default::default()
        });
    }

    let chart_data = cx::ChartData {
        external_data: None,
        data,
        extension_list: None,
    };

    let plot_area = cx::PlotArea {
        plot_area_region: Box::new(cx::PlotAreaRegion {
            plot_surface: None,
            series,
            extension_list: None,
        }),
        axis: if is_cartesian(kind) {
            cartesian_axes()
        } else {
            Vec::new()
        },
        shape_properties: None,
        extension_list: None,
    };

    let legend = patch
        .legend_position
        .and_then(|pos| legend_side(pos).map(|side| (pos, side)))
        .map(|(_, side)| {
            Box::new(cx::Legend {
                pos: Some(side),
                align: Some(cx::PosAlign::Ctr),
                overlay: Some(BooleanValue::from_bool(false)),
                ..Default::default()
            })
        });

    let chart = cx::Chart {
        xmlns: Vec::new(),
        chart_title: patch
            .title
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(chart_title),
        plot_area: Box::new(plot_area),
        legend,
        extension_list: None,
    };

    cx::ChartSpace {
        xmlns: crate::ooxml_header::chart_ex_space(),
        xml_header: crate::ooxml_header::STANDALONE,
        chart_data: Some(Box::new(chart_data)),
        chart: Box::new(chart),
        ..Default::default()
    }
}

fn build_layout_pr(
    patch: &ChartExPatch,
    series_idx: usize,
) -> Option<Box<cx::SeriesLayoutProperties>> {
    let kind = patch.kind;
    match kind {
        ChartExKind::Waterfall => Some(Box::new(cx::SeriesLayoutProperties {
            subtotals: Some(cx::Subtotals {
                unsigned_integer_type: patch
                    .subtotals
                    .iter()
                    .map(|i| cx::UnsignedIntegerType { val: *i })
                    .collect(),
            }),
            ..Default::default()
        })),
        ChartExKind::Histogram | ChartExKind::Pareto if series_idx == 0 => {
            let binning_choice = if let Some(c) = patch.bin_count {
                Some(cx::BinningChoice::BinCountXsdunsignedInt(c))
            } else {
                patch.bin_size.map(cx::BinningChoice::Xsddouble)
            };
            Some(Box::new(cx::SeriesLayoutProperties {
                series_layout_properties_choice: Some(cx::SeriesLayoutPropertiesChoice::Binning(
                    Box::new(cx::Binning {
                        binning_choice,
                        ..Default::default()
                    }),
                )),
                ..Default::default()
            }))
        }
        ChartExKind::BoxWhisker => Some(Box::new(cx::SeriesLayoutProperties {
            statistics: Some(cx::Statistics {
                quartile_method: Some(match patch.quartile_method.unwrap_or_default() {
                    ChartExQuartileMethod::Inclusive => cx::QuartileMethod::Inclusive,
                    ChartExQuartileMethod::Exclusive => cx::QuartileMethod::Exclusive,
                }),
            }),
            ..Default::default()
        })),
        _ => None,
    }
}

fn chart_ex_anchor_rid(anchor: &xdr::TwoCellAnchor) -> Option<String> {
    let xdr::TwoCellAnchorChoice::GraphicFrame(gf) = anchor.two_cell_anchor_choice.as_ref()? else {
        return None;
    };
    if gf.graphic.graphic_data.uri.as_str() != CHARTEX_GRAPHIC_DATA_URI {
        return None;
    }
    for choice in &gf.graphic.graphic_data.graphic_data_choice {
        match choice {
            a::GraphicDataChoice::ChartReference(r) if !r.id.is_empty() => {
                return Some(r.id.clone());
            }
            a::GraphicDataChoice::XmlAny(raw) => {
                let raw = String::from_utf8_lossy(raw);
                if let Some(i) = raw.find("r:id=\"") {
                    let rest = &raw[i + 6..];
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

fn build_chart_ex_anchor(
    anchor: &ChartAnchor,
    name: &str,
    index: usize,
    rid: &str,
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

    let nv_props = xdr::NonVisualGraphicFrameProperties {
        non_visual_drawing_properties: Box::new(xdr::NonVisualDrawingProperties {
            id: index as u32 + 1,
            name: name.to_string(),
            ..Default::default()
        }),
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

    let chart_ref = format!(
        "<cx:chart xmlns:cx=\"{CHARTEX_NS}\" xmlns:r=\"{RELATIONSHIPS_NS}\" r:id=\"{rid}\"/>"
    );
    let graphic_data = a::GraphicData {
        uri: CHARTEX_GRAPHIC_DATA_URI.to_string(),
        graphic_data_choice: vec![a::GraphicDataChoice::XmlAny(
            chart_ref.into_bytes().into_boxed_slice(),
        )],
        ..Default::default()
    };

    let graphic_frame = xdr::GraphicFrame {
        r#macro: Some(String::new()),
        non_visual_graphic_frame_properties: Box::new(nv_props),
        transform: Box::new(xfrm),
        graphic: Box::new(a::Graphic {
            graphic_data: Box::new(graphic_data),
            ..Default::default()
        }),
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

fn validate_chart_ex(sheet: &str, patch: &ChartExPatch) -> Result<()> {
    if patch.series.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            "chartEx requires at least one series",
        )
        .with_sheet(sheet));
    }
    let multi_ok = matches!(patch.kind, ChartExKind::BoxWhisker);
    if !multi_ok && patch.series.len() > 1 {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!("{:?} chartEx supports a single series", patch.kind),
        )
        .with_sheet(sheet));
    }
    if patch.categories_ref.is_none() && patch.kind != ChartExKind::Histogram {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!("{:?} chartEx requires categoriesRef", patch.kind),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

impl Workbook {
    pub fn chart_exs(&mut self, sheet: Option<&str>) -> Result<Vec<ChartExInfo>> {
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
            let parts: Vec<ExtendedChartPart> =
                drawings_part.extended_chart_parts(&self.doc).collect();
            let by_rid: std::collections::HashMap<String, ExtendedChartPart> = parts
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
                let Some(rid) = chart_ex_anchor_rid(anchor) else {
                    continue;
                };
                let Some(part) = by_rid.get(&rid).cloned() else {
                    continue;
                };
                let space = part
                    .root_element(&mut self.doc)
                    .map_err(sdk_err_to_api)?
                    .clone();
                let name = super::read::anchor_chart_name(anchor)
                    .unwrap_or_else(|| format!("Chart {}", anchor_idx + 1));
                out.push(read_chart_ex_space(sheet_name, &rid, name, anchor, &space));
            }
        }
        Ok(out)
    }

    pub fn set_chart_ex(
        &mut self,
        sheet: impl AsRef<str>,
        patch: ChartExPatch,
    ) -> Result<ChartExInfo> {
        let sheet = sheet.as_ref();
        validate_chart_ex(sheet, &patch)?;
        let anchor = crate::refs::resolve_anchor(&patch.anchor)?;

        if !self.sheet_exists(sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet}"),
            )
            .with_sheet(sheet));
        }

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
                id: rid,
                ..Default::default()
            });
        }

        let chart_part: ExtendedChartPart = drawings_part
            .add_new_part_auto_id(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let chart_rid = chart_part
            .relationship_id()
            .ok_or_else(|| ApiError::new(ApiErrorCode::Other, "new chartEx part missing rid"))?
            .to_string();

        let space = build_chart_ex_space(&patch);
        chart_part
            .set_root_element(&mut self.doc, space)
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

        let new_anchor = build_chart_ex_anchor(&anchor, &chart_name, chart_index, &chart_rid);
        let drawing_mut = drawings_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        drawing_mut
            .worksheet_drawing_choice
            .push(xdr::WorksheetDrawingChoice::TwoCellAnchor(Box::new(
                new_anchor,
            )));

        Ok(ChartExInfo {
            sheet: sheet.to_string(),
            id: chart_rid,
            name: chart_name,
            kind: patch.kind,
            title: patch.title.clone().filter(|t| !t.is_empty()),
            anchor,
            categories_ref: patch.categories_ref.clone(),
            series: patch
                .series
                .iter()
                .map(|s| ChartExSeriesInfo {
                    name: s.name.clone(),
                    name_ref: s.name_ref.clone(),
                    values_ref: s.values_ref.clone(),
                })
                .collect(),
            legend_position: patch.legend_position,
            subtotals: if patch.kind == ChartExKind::Waterfall {
                patch.subtotals.clone()
            } else {
                Vec::new()
            },
            bin_count: patch
                .bin_count
                .filter(|_| patch.kind == ChartExKind::Histogram),
            bin_size: patch
                .bin_size
                .filter(|_| patch.kind == ChartExKind::Histogram),
            quartile_method: patch
                .quartile_method
                .filter(|_| patch.kind == ChartExKind::BoxWhisker),
        })
    }

    pub fn remove_chart_ex(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
    ) -> Result<Option<ChartExInfo>> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let all = self.chart_exs(Some(&sheet))?;
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
                    chart_ex_anchor_rid(a.as_ref()).as_deref() != Some(id.as_str())
                }
                _ => true,
            });

        let _ = drawings_part
            .delete_part_by_id(&mut self.doc, id.as_str())
            .map_err(sdk_err_to_api)?;

        Ok(Some(info))
    }
}

fn read_chart_ex_space(
    sheet: &str,
    rid: &str,
    name: String,
    anchor: &xdr::TwoCellAnchor,
    space: &cx::ChartSpace,
) -> ChartExInfo {
    let chart = space.chart.as_ref();
    let region = chart.plot_area.plot_area_region.as_ref();
    let series_list = &region.series;

    let has_pareto = series_list
        .iter()
        .any(|s| matches!(s.layout_id, cx::SeriesLayout::ParetoLine));
    let all_box = !series_list.is_empty()
        && series_list
            .iter()
            .all(|s| matches!(s.layout_id, cx::SeriesLayout::BoxWhisker));
    let first = series_list.first();
    let has_binning = first
        .and_then(|s| s.series_layout_properties.as_deref())
        .and_then(|lp| lp.series_layout_properties_choice.as_ref())
        .is_some_and(|c| matches!(c, cx::SeriesLayoutPropertiesChoice::Binning(_)));
    let single_histogram = series_list.len() == 1
        && first.is_some_and(|s| matches!(s.layout_id, cx::SeriesLayout::ClusteredColumn))
        && has_binning;

    let kind = if has_pareto {
        ChartExKind::Pareto
    } else if all_box {
        ChartExKind::BoxWhisker
    } else if single_histogram {
        ChartExKind::Histogram
    } else {
        match first.map(|s| s.layout_id) {
            Some(cx::SeriesLayout::Waterfall) => ChartExKind::Waterfall,
            Some(cx::SeriesLayout::Funnel) => ChartExKind::Funnel,
            Some(cx::SeriesLayout::Treemap) => ChartExKind::Treemap,
            Some(cx::SeriesLayout::Sunburst) => ChartExKind::Sunburst,
            Some(cx::SeriesLayout::RegionMap) => ChartExKind::RegionMap,
            _ => ChartExKind::Funnel,
        }
    };

    let data_blocks = space
        .chart_data
        .as_deref()
        .map(|cd| cd.data.as_slice())
        .unwrap_or_default();
    let find_data = |id: u32| data_blocks.iter().find(|d| d.id == id);

    let mut categories_ref: Option<String> = None;
    let mut series: Vec<ChartExSeriesInfo> = Vec::new();
    for s in series_list {
        if matches!(s.layout_id, cx::SeriesLayout::ParetoLine) {
            continue;
        }
        let data_id = s.data_id.as_ref().map(|d| d.val).unwrap_or(0);
        let block = find_data(data_id).or_else(|| data_blocks.first());
        let mut values_ref = String::new();
        if let Some(block) = block {
            for choice in &block.data_choice {
                match choice {
                    cx::DataChoice::StringDimension(sd) => {
                        if categories_ref.is_none() {
                            categories_ref =
                                dim_formula_string(sd.string_dimension_choice.as_ref());
                        }
                    }
                    cx::DataChoice::NumericDimension(nd) => {
                        if values_ref.is_empty() {
                            if let Some(f) =
                                dim_formula_numeric(nd.numeric_dimension_choice.as_ref())
                            {
                                values_ref = f;
                            }
                        }
                    }
                }
            }
        }
        let (name, name_ref) = read_series_text(s.text.as_deref());
        series.push(ChartExSeriesInfo {
            name,
            name_ref,
            values_ref,
        });
    }

    let title = chart
        .chart_title
        .as_deref()
        .and_then(|t| read_text(t.text.as_deref()));

    let legend_position = chart.legend.as_deref().and_then(|l| {
        l.pos.map(|p| match p {
            cx::SidePos::L => ChartLegendPosition::Left,
            cx::SidePos::T => ChartLegendPosition::Top,
            cx::SidePos::B => ChartLegendPosition::Bottom,
            cx::SidePos::R => ChartLegendPosition::Right,
        })
    });

    let subtotals: Vec<u32> = if kind == ChartExKind::Waterfall {
        first
            .and_then(|s| s.series_layout_properties.as_deref())
            .and_then(|lp| lp.subtotals.as_ref())
            .map(|sub| sub.unsigned_integer_type.iter().map(|i| i.val).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let (bin_count, bin_size) = if kind == ChartExKind::Histogram {
        match first
            .and_then(|s| s.series_layout_properties.as_deref())
            .and_then(|lp| lp.series_layout_properties_choice.as_ref())
        {
            Some(cx::SeriesLayoutPropertiesChoice::Binning(b)) => match &b.binning_choice {
                Some(cx::BinningChoice::BinCountXsdunsignedInt(c)) => (Some(*c), None),
                Some(cx::BinningChoice::Xsddouble(d)) => (None, Some(*d)),
                None => (None, None),
            },
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    let quartile_method = if kind == ChartExKind::BoxWhisker {
        first
            .and_then(|s| s.series_layout_properties.as_deref())
            .and_then(|lp| lp.statistics.as_ref())
            .and_then(|st| st.quartile_method)
            .map(|q| match q {
                cx::QuartileMethod::Inclusive => ChartExQuartileMethod::Inclusive,
                cx::QuartileMethod::Exclusive => ChartExQuartileMethod::Exclusive,
            })
    } else {
        None
    };

    ChartExInfo {
        sheet: sheet.to_string(),
        id: rid.to_string(),
        name,
        kind,
        title,
        anchor: super::read::anchor_to_chart_anchor(anchor),
        categories_ref,
        series,
        legend_position,
        subtotals,
        bin_count,
        bin_size,
        quartile_method,
    }
}

fn dim_formula_string(choice: Option<&cx::StringDimensionChoice>) -> Option<String> {
    match choice? {
        cx::StringDimensionChoice::Sequence(seq) => seq.formula.xml_content.clone(),
        cx::StringDimensionChoice::StringLevel(_) => None,
    }
}

fn dim_formula_numeric(choice: Option<&cx::NumericDimensionChoice>) -> Option<String> {
    match choice? {
        cx::NumericDimensionChoice::Sequence(seq) => seq.formula.xml_content.clone(),
        cx::NumericDimensionChoice::NumericLevel(_) => None,
    }
}

fn read_series_text(text: Option<&cx::Text>) -> (Option<String>, Option<String>) {
    let Some(choice) = text.and_then(|t| t.text_choice.as_ref()) else {
        return (None, None);
    };
    match choice {
        cx::TextChoice::TextData(td) => match td.text_data_choice.as_ref() {
            Some(cx::TextDataChoice::VXsdstring(v)) => (Some(v.clone()), None),
            Some(cx::TextDataChoice::Sequence(seq)) => {
                (seq.v_xsdstring.clone(), seq.formula.0.xml_content.clone())
            }
            None => (None, None),
        },
        cx::TextChoice::RichTextBody(_) => (None, None),
    }
}

fn read_text(text: Option<&cx::Text>) -> Option<String> {
    let (name, _) = read_series_text(text);
    name
}
