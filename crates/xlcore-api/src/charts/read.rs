use super::*;

pub(super) fn anchor_chart_rid(anchor: &xdr::TwoCellAnchor) -> Option<String> {
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

pub(super) fn anchor_chart_name(anchor: &xdr::TwoCellAnchor) -> Option<String> {
    let xdr::TwoCellAnchorChoice::GraphicFrame(gf) = anchor.two_cell_anchor_choice.as_ref()? else {
        return None;
    };
    let cnv = &gf
        .non_visual_graphic_frame_properties
        .non_visual_drawing_properties;
    let n = cnv.name.as_str();
    if n.is_empty() {
        None
    } else {
        Some(n.to_string())
    }
}

pub(super) fn anchor_to_chart_anchor(anchor: &xdr::TwoCellAnchor) -> ChartAnchor {
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

pub(super) struct ParsedChart {
    pub(super) kind: ChartKind,
    pub(super) title: Option<String>,
    pub(super) legend: Option<ChartLegendPosition>,
    pub(super) categories_ref: Option<String>,
    pub(super) series: Vec<ChartSeriesInfo>,
    pub(super) category_axis_title: Option<String>,
    pub(super) value_axis_title: Option<String>,
    pub(super) category_axis: Option<ChartAxisPatch>,
    pub(super) value_axis: Option<ChartAxisPatch>,
    pub(super) stacking: Option<ChartStacking>,
    pub(super) gap_width: Option<u16>,
    pub(super) overlap: Option<i8>,
    pub(super) data_labels: Option<ChartDataLabels>,
}

pub(super) fn read_chart_space(space: &c::ChartSpace) -> ParsedChart {
    let plot = &space.chart.plot_area;

    let mut kind = ChartKind::Column;
    let mut series: Vec<ChartSeriesInfo> = Vec::new();
    let mut categories_ref: Option<String> = None;
    let mut stacking: Option<ChartStacking> = None;
    let mut gap_width: Option<u16> = None;
    let mut overlap: Option<i8> = None;
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
                gap_width = bc.gap_width.as_ref().and_then(|g| g.val);
                overlap = bc.overlap.as_ref().and_then(|o| o.val);
                for s in &bc.bar_chart_series {
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
                    data_labels = read_data_labels(bc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::LineChart(lc) => {
                kind = ChartKind::Line;
                stacking = lc.grouping.val.as_ref().map(grouping_to_stacking);
                for s in &lc.line_chart_series {
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
                        s.x_values
                            .as_deref()
                            .and_then(|x| x.x_values_choice.as_ref()),
                        s.y_values
                            .as_deref()
                            .and_then(|y| y.y_values_choice.as_ref()),
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
                        s.x_values
                            .as_deref()
                            .and_then(|x| x.x_values_choice.as_ref()),
                        s.y_values
                            .as_deref()
                            .and_then(|y| y.y_values_choice.as_ref()),
                        s.data_labels.as_deref(),
                    );
                    info.bubble_sizes_ref = s.bubble_size.as_deref().and_then(|b| {
                        match b.bubble_size_choice.as_ref()? {
                            c::BubbleSizeChoice::NumberReference(nr) => Some(nr.formula.clone()),
                            _ => None,
                        }
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
    let mut category_axis: Option<ChartAxisPatch> = None;
    let mut value_axis: Option<ChartAxisPatch> = None;
    for ax in &plot.plot_area_choice2 {
        match ax {
            c::PlotAreaChoice2::CategoryAxis(c) => {
                if let Some(t) = c.title.as_deref() {
                    category_axis_title = extract_title_text(t);
                }
                category_axis = read_cat_axis_patch(c);
            }
            c::PlotAreaChoice2::ValueAxis(v) => {
                let is_cat = v.axis_position.val == c::AxisPositionValues::Bottom
                    && category_axis_title.is_none()
                    && category_axis.is_none();
                if let Some(t) = v.title.as_deref() {
                    if is_cat {
                        category_axis_title = extract_title_text(t);
                    } else if value_axis_title.is_none() {
                        value_axis_title = extract_title_text(t);
                    }
                }
                if is_cat {
                    category_axis = read_val_axis_patch(v);
                } else if value_axis.is_none() {
                    value_axis = read_val_axis_patch(v);
                }
            }
            _ => {}
        }
    }

    let title = space
        .chart
        .title
        .as_ref()
        .and_then(|t| extract_title_text(t));

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
        category_axis,
        value_axis,
        stacking,
        gap_width,
        overlap,
        data_labels,
    }
}

pub(super) fn grouping_to_stacking(v: &c::GroupingValues) -> ChartStacking {
    match v {
        c::GroupingValues::Standard => ChartStacking::Clustered,
        c::GroupingValues::Stacked => ChartStacking::Stacked,
        c::GroupingValues::PercentStacked => ChartStacking::PercentStacked,
    }
}

pub(super) fn read_xy_series(
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

pub(super) fn read_series_color(sp: Option<&c::ChartShapeProperties>) -> Option<String> {
    let sp = sp?;
    let choice = sp.chart_shape_properties_choice2.as_ref()?;
    let c::ChartShapePropertiesChoice2::SolidFill(sf) = choice else {
        return None;
    };
    let inner = sf.solid_fill_choice.as_ref()?;
    let a::SolidFillChoice::RgbColorModelHex(rgb) = inner else {
        return None;
    };
    Some(rgb.val.to_string().to_uppercase())
}

pub(super) fn read_series(
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

pub(super) fn extract_title_text(title: &c::Title) -> Option<String> {
    let choice = title.chart_text.as_ref()?.chart_text_choice.as_ref()?;
    match choice {
        c::ChartTextChoice::RichText(rt) => rt.paragraph.iter().find_map(|p| {
            p.paragraph_choice.iter().find_map(|pc| match pc {
                a::ParagraphChoice::Run(r) => Some(r.text.clone()),
                _ => None,
            })
        }),
        c::ChartTextChoice::StringReference(sr) => Some(sr.formula.clone()),
        c::ChartTextChoice::StringLiteral(sl) => {
            sl.string_point.first().map(|p| p.numeric_value.clone())
        }
    }
}

pub(super) fn legend_pos_from(v: &c::LegendPositionValues) -> ChartLegendPosition {
    match v {
        c::LegendPositionValues::Right => ChartLegendPosition::Right,
        c::LegendPositionValues::Left => ChartLegendPosition::Left,
        c::LegendPositionValues::Top => ChartLegendPosition::Top,
        c::LegendPositionValues::Bottom => ChartLegendPosition::Bottom,
        c::LegendPositionValues::TopRight => ChartLegendPosition::TopRight,
    }
}

pub(super) fn data_label_pos_from(v: &c::DataLabelPositionValues) -> ChartDataLabelPosition {
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

pub(super) fn read_data_labels(dl: Option<&c::DataLabels>) -> Option<ChartDataLabels> {
    let dl = dl?;
    let choice = dl.data_labels_choice.as_ref()?;
    let seq = match choice {
        c::DataLabelsChoice::Sequence(s) => s,
        _ => return None,
    };
    let bv = |b: Option<&BooleanValue>| b.map(|v| bool::from(*v));
    let out = ChartDataLabels {
        show_value: bv(seq.show_value.as_ref().and_then(|s| s.val.as_ref())),
        show_category_name: bv(seq.show_category_name.as_ref().and_then(|s| s.val.as_ref())),
        show_series_name: bv(seq.show_series_name.as_ref().and_then(|s| s.val.as_ref())),
        show_percent: bv(seq.show_percent.as_ref().and_then(|s| s.val.as_ref())),
        show_legend_key: bv(seq.show_legend_key.as_ref().and_then(|s| s.val.as_ref())),
        position: seq
            .data_label_position
            .as_ref()
            .map(|p| data_label_pos_from(&p.val)),
        separator: seq.separator.clone(),
    };
    if out == ChartDataLabels::default() {
        None
    } else {
        Some(out)
    }
}

pub(super) fn tick_mark_from(v: &c::TickMarkValues) -> TickMark {
    match v {
        c::TickMarkValues::Cross => TickMark::Cross,
        c::TickMarkValues::Inside => TickMark::Inside,
        c::TickMarkValues::Outside => TickMark::Outside,
        c::TickMarkValues::None => TickMark::None,
    }
}

pub(super) fn tick_label_pos_from(v: &c::TickLabelPositionValues) -> TickLabelPosition {
    match v {
        c::TickLabelPositionValues::High => TickLabelPosition::High,
        c::TickLabelPositionValues::Low => TickLabelPosition::Low,
        c::TickLabelPositionValues::NextTo => TickLabelPosition::NextTo,
        c::TickLabelPositionValues::None => TickLabelPosition::None,
    }
}

pub(super) fn cross_between_from(v: &c::CrossBetweenValues) -> CrossBetween {
    match v {
        c::CrossBetweenValues::Between => CrossBetween::Between,
        c::CrossBetweenValues::MidpointCategory => CrossBetween::MidpointCategory,
    }
}

pub(super) fn read_cat_axis_patch(ax: &c::CategoryAxis) -> Option<ChartAxisPatch> {
    let mut p = ChartAxisPatch {
        title: ax.title.as_deref().and_then(extract_title_text),
        hidden: read_axis_hidden(ax.delete.as_ref()),
        min: ax.scaling.min_axis_value.as_ref().map(|m| m.val),
        max: ax.scaling.max_axis_value.as_ref().map(|m| m.val),
        reversed: read_axis_reversed(ax.scaling.orientation.as_ref()),
        major_gridlines: ax.major_gridlines.as_ref().map(|_| true),
        minor_gridlines: ax.minor_gridlines.as_ref().map(|_| true),
        major_tick_mark: ax
            .major_tick_mark
            .as_ref()
            .and_then(|t| t.val.as_ref())
            .map(tick_mark_from),
        minor_tick_mark: ax
            .minor_tick_mark
            .as_ref()
            .and_then(|t| t.val.as_ref())
            .map(tick_mark_from),
        tick_label_position: ax
            .tick_label_position
            .as_ref()
            .and_then(|t| t.val.as_ref())
            .map(tick_label_pos_from),
        number_format: ax.numbering_format.as_ref().map(|n| n.format_code.clone()),
        crosses_at: match ax.category_axis_choice.as_ref() {
            Some(c::CategoryAxisChoice::CrossesAt(c)) => Some(c.val),
            _ => None,
        },
        ..Default::default()
    };
    p.title = p.title.filter(|t| !t.is_empty());
    if p == ChartAxisPatch::default() {
        None
    } else {
        Some(p)
    }
}

pub(super) fn read_val_axis_patch(ax: &c::ValueAxis) -> Option<ChartAxisPatch> {
    let mut p = ChartAxisPatch {
        title: ax.title.as_deref().and_then(extract_title_text),
        hidden: read_axis_hidden(ax.delete.as_ref()),
        min: ax.scaling.min_axis_value.as_ref().map(|m| m.val),
        max: ax.scaling.max_axis_value.as_ref().map(|m| m.val),
        log_base: ax.scaling.log_base.as_ref().map(|l| l.val),
        reversed: read_axis_reversed(ax.scaling.orientation.as_ref()),
        major_unit: ax.major_unit.as_ref().map(|u| u.val),
        minor_unit: ax.minor_unit.as_ref().map(|u| u.val),
        major_gridlines: ax.major_gridlines.as_ref().map(|_| true),
        minor_gridlines: ax.minor_gridlines.as_ref().map(|_| true),
        major_tick_mark: ax
            .major_tick_mark
            .as_ref()
            .and_then(|t| t.val.as_ref())
            .map(tick_mark_from),
        minor_tick_mark: ax
            .minor_tick_mark
            .as_ref()
            .and_then(|t| t.val.as_ref())
            .map(tick_mark_from),
        tick_label_position: ax
            .tick_label_position
            .as_ref()
            .and_then(|t| t.val.as_ref())
            .map(tick_label_pos_from),
        number_format: ax.numbering_format.as_ref().map(|n| n.format_code.clone()),
        cross_between: ax
            .cross_between
            .as_ref()
            .map(|c| cross_between_from(&c.val)),
        crosses_at: match ax.value_axis_choice.as_ref() {
            Some(c::ValueAxisChoice::CrossesAt(c)) => Some(c.val),
            _ => None,
        },
    };
    p.title = p.title.filter(|t| !t.is_empty());
    if p == ChartAxisPatch::default() {
        None
    } else {
        Some(p)
    }
}

pub(super) fn read_axis_hidden(delete: Option<&c::Delete>) -> Option<bool> {
    let d = delete?;
    let v = d.val.as_ref()?.as_bool();
    if v {
        Some(true)
    } else {
        None
    }
}

pub(super) fn read_axis_reversed(orientation: Option<&c::Orientation>) -> Option<bool> {
    let o = orientation?;
    match o.val.as_ref()? {
        c::OrientationValues::MaxMin => Some(true),
        c::OrientationValues::MinMax => None,
    }
}
