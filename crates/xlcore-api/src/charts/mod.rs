mod read;
mod write;

use read::*;
use write::*;

use ooxmlsdk::parts::chart_part::ChartPart;
use ooxmlsdk::parts::drawings_part::DrawingsPart;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_spreadsheet_drawing as xdr;
use ooxmlsdk::sdk::SdkPart;
use ooxmlsdk::simple_type::{BooleanValue, CoordinateValue};
use xlcore_io::spreadsheetml as x;
use xlcore_types::{
    AnchorSpec, ApiError, ApiErrorCode, ApiWarning, ChartAnchor, ChartAxisGroup, ChartAxisPatch,
    ChartDataLabelPosition, ChartDataLabels, ChartInfo, ChartKind, ChartLegendPosition,
    ChartMarker, ChartPatch, ChartSeriesInfo, ChartSeriesPatch, ChartStacking, ChartUpdate,
    CrossBetween, MarkerStyle, RadarStyle, TickLabelPosition, TickMark,
};

use crate::errors::sdk_err_to_api;
use crate::{Result, Workbook};

const CHART_GRAPHIC_DATA_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const CAT_AX_ID: u32 = 111_111_111;
const VAL_AX_ID: u32 = 222_222_222;
const SEC_CAT_AX_ID: u32 = 333_333_333;
const SEC_VAL_AX_ID: u32 = 444_444_444;

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
            let chart_parts: Vec<ChartPart> = drawings_part.chart_parts(&self.doc).collect();
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
                    category_axis: parsed.category_axis,
                    value_axis: parsed.value_axis,
                    stacking: parsed.stacking,
                    gap_width: parsed.gap_width,
                    overlap: parsed.overlap,
                    radar_style: parsed.radar_style,
                    data_labels: parsed.data_labels,
                });
            }
        }
        Ok(out)
    }

    pub fn set_chart(&mut self, sheet: impl AsRef<str>, patch: ChartPatch) -> Result<ChartInfo> {
        let sheet = sheet.as_ref();
        validate_chart_series(sheet, patch.kind, &patch.series)?;
        validate_bar_options(sheet, patch.gap_width, patch.overlap)?;
        let anchor = crate::refs::resolve_anchor(&patch.anchor)?;

        if !self.sheet_exists(sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet}"),
            )
            .with_sheet(sheet));
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
                .with_sheet(sheet),
            );
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

        let new_anchor = build_two_cell_anchor(&anchor, &chart_name, chart_index, &chart_rid);

        let drawing_mut = drawings_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        drawing_mut
            .worksheet_drawing_choice
            .push(xdr::WorksheetDrawingChoice::TwoCellAnchor(Box::new(
                new_anchor,
            )));

        Ok(ChartInfo {
            sheet: sheet.to_string(),
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
                    marker: s.marker.clone(),
                    kind: s.kind,
                    axis: s.axis,
                })
                .collect(),
            anchor,
            category_axis_title: patch.category_axis_title.clone(),
            value_axis_title: patch.value_axis_title.clone(),
            category_axis: patch.category_axis.clone(),
            value_axis: patch.value_axis.clone(),
            stacking: stacking_for_kind(patch.kind, patch.stacking),
            gap_width: bar_option_for_kind(patch.kind, patch.gap_width),
            overlap: bar_option_for_kind(patch.kind, patch.overlap),
            radar_style: (patch.kind == ChartKind::Radar)
                .then(|| patch.radar_style.unwrap_or(RadarStyle::Standard)),
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

    pub fn update_chart(
        &mut self,
        sheet: impl AsRef<str>,
        id: impl AsRef<str>,
        update: ChartUpdate,
    ) -> Result<ChartInfo> {
        let sheet = sheet.as_ref().to_string();
        let id = id.as_ref().to_string();
        let resolved_anchor = update
            .anchor
            .as_ref()
            .map(crate::refs::resolve_anchor)
            .transpose()?;

        if !self.sheet_exists(&sheet)? {
            return Err(ApiError::new(
                ApiErrorCode::MissingSheet,
                format!("sheet not found: {sheet}"),
            )
            .with_sheet(&sheet));
        }

        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let drawings_part = ws_part.drawings_part(&self.doc).map(|p| p.clone());
        let Some(drawings_part) = drawings_part else {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart not found on sheet '{sheet}': {id}"),
            )
            .with_sheet(&sheet));
        };

        let chart_part = drawings_part
            .chart_parts(&self.doc)
            .find(|p| p.relationship_id().map(|r| r == id).unwrap_or(false));
        let Some(chart_part) = chart_part else {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart not found on sheet '{sheet}': {id}"),
            )
            .with_sheet(&sheet));
        };

        let existing = read_chart_space(
            chart_part
                .root_element(&mut self.doc)
                .map_err(sdk_err_to_api)?,
        );
        let kind = existing.kind;

        validate_bar_options(&sheet, update.gap_width, update.overlap)?;

        let plot_dirty = update.series.is_some()
            || update.stacking.is_some()
            || update.data_labels.is_some()
            || update.categories_ref.is_some()
            || update.gap_width.is_some()
            || update.overlap.is_some()
            || update.radar_style.is_some();

        let series: Vec<ChartSeriesPatch> = match &update.series {
            Some(s) => s.clone(),
            None => existing
                .series
                .iter()
                .map(|s| ChartSeriesPatch {
                    name: s.name.clone(),
                    name_ref: s.name_ref.clone(),
                    values_ref: s.values_ref.clone(),
                    x_values_ref: s.x_values_ref.clone(),
                    bubble_sizes_ref: s.bubble_sizes_ref.clone(),
                    color: s.color.clone(),
                    data_labels: s.data_labels.clone(),
                    marker: s.marker.clone(),
                    kind: s.kind,
                    axis: s.axis,
                })
                .collect(),
        };
        if plot_dirty {
            validate_chart_series(&sheet, kind, &series)?;
        }

        let categories_ref = update
            .categories_ref
            .clone()
            .or_else(|| existing.categories_ref.clone());
        let stacking = update.stacking.or(existing.stacking);
        let gap_width = update.gap_width.or(existing.gap_width);
        let overlap = update.overlap.or(existing.overlap);
        let radar_style = update.radar_style.or(existing.radar_style);
        let data_labels = update
            .data_labels
            .clone()
            .or_else(|| existing.data_labels.clone());

        if resolved_anchor.is_some() || update.name.is_some() {
            let drawing_mut = drawings_part
                .root_element_mut(&mut self.doc)
                .map_err(sdk_err_to_api)?;
            for choice in &mut drawing_mut.worksheet_drawing_choice {
                let xdr::WorksheetDrawingChoice::TwoCellAnchor(a) = choice else {
                    continue;
                };
                if anchor_chart_rid(a.as_ref()).as_deref() != Some(id.as_str()) {
                    continue;
                }
                if let Some(anchor) = &resolved_anchor {
                    a.from_marker = Box::new(xdr::FromMarker {
                        column_id: anchor.from_column as i32,
                        column_offset: CoordinateValue::Emu(
                            anchor.from_column_offset_emu.unwrap_or(0),
                        ),
                        row_id: anchor.from_row as i32,
                        row_offset: CoordinateValue::Emu(anchor.from_row_offset_emu.unwrap_or(0)),
                        ..Default::default()
                    });
                    a.to_marker = Box::new(xdr::ToMarker {
                        column_id: anchor.to_column as i32,
                        column_offset: CoordinateValue::Emu(
                            anchor.to_column_offset_emu.unwrap_or(0),
                        ),
                        row_id: anchor.to_row as i32,
                        row_offset: CoordinateValue::Emu(anchor.to_row_offset_emu.unwrap_or(0)),
                        ..Default::default()
                    });
                }
                if let Some(name) = &update.name {
                    if let Some(xdr::TwoCellAnchorChoice::GraphicFrame(gf)) =
                        a.two_cell_anchor_choice.as_mut()
                    {
                        gf.non_visual_graphic_frame_properties
                            .non_visual_drawing_properties
                            .name = name.clone();
                    }
                }
                break;
            }
        }

        let space = chart_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;

        if plot_dirty {
            let synth = ChartPatch {
                name: None,
                kind,
                title: None,
                legend_position: None,
                categories_ref: categories_ref.clone(),
                series: series.clone(),
                anchor: AnchorSpec::Cells(ChartAnchor::default()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking,
                gap_width,
                overlap,
                radar_style,
                data_labels: data_labels.clone(),
            };
            space.chart.plot_area.plot_area_choice1 = build_plot_charts(&synth);
            space
                .chart
                .plot_area
                .plot_area_choice2
                .retain(|ax| match ax {
                    c::PlotAreaChoice2::ValueAxis(v) => v.axis_id.val != SEC_VAL_AX_ID,
                    c::PlotAreaChoice2::CategoryAxis(cx) => cx.axis_id.val != SEC_CAT_AX_ID,
                    _ => true,
                });
            if synth.series.iter().any(series_secondary) {
                space
                    .chart
                    .plot_area
                    .plot_area_choice2
                    .push(c::PlotAreaChoice2::ValueAxis(
                        Box::new(build_sec_val_axis()),
                    ));
                space
                    .chart
                    .plot_area
                    .plot_area_choice2
                    .push(c::PlotAreaChoice2::CategoryAxis(Box::new(
                        build_sec_cat_axis(),
                    )));
            }
        }

        if let Some(title) = &update.title {
            if title.is_empty() {
                space.chart.title = None;
                space.chart.auto_title_deleted = Some(c::AutoTitleDeleted {
                    val: Some(BooleanValue::from_bool(true)),
                });
            } else {
                space.chart.title = Some(Box::new(build_title(title)));
                space.chart.auto_title_deleted = Some(c::AutoTitleDeleted {
                    val: Some(BooleanValue::from_bool(false)),
                });
            }
        }

        if let Some(pos) = update.legend_position {
            space.chart.legend = match pos {
                ChartLegendPosition::None => None,
                pos => Some(Box::new(build_legend(legend_pos_to(pos)))),
            };
        }

        let cat_axis_patch =
            merge_axis_title(update.category_axis.as_ref(), &update.category_axis_title);
        let val_axis_patch = merge_axis_title(update.value_axis.as_ref(), &update.value_axis_title);
        if cat_axis_patch.is_some() || val_axis_patch.is_some() {
            for ax in &mut space.chart.plot_area.plot_area_choice2 {
                match ax {
                    c::PlotAreaChoice2::CategoryAxis(cat) => {
                        if let Some(p) = &cat_axis_patch {
                            apply_cat_axis_patch(cat, p);
                        }
                    }
                    c::PlotAreaChoice2::ValueAxis(v) => {
                        if v.axis_position.val == c::AxisPositionValues::Bottom {
                            if let Some(p) = &cat_axis_patch {
                                apply_val_axis_patch(v, p);
                            }
                        } else if let Some(p) = &val_axis_patch {
                            apply_val_axis_patch(v, p);
                        }
                    }
                    _ => {}
                }
            }
        }

        if stacking.is_some()
            && !matches!(
                kind,
                ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area
            )
        {
            self.push_warning(
                ApiWarning::new(
                    ApiErrorCode::LossyOperation,
                    format!("stacking is not supported on {kind:?} charts; ignored"),
                )
                .with_sheet(&sheet),
            );
        }

        self.charts(Some(&sheet))?
            .into_iter()
            .find(|chart| chart.id == id)
            .ok_or_else(|| {
                ApiError::new(
                    ApiErrorCode::InvalidChart,
                    format!("chart not found on sheet '{sheet}': {id}"),
                )
                .with_sheet(&sheet)
            })
    }
}
