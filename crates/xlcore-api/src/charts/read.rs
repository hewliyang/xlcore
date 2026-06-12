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
    pub(super) radar_style: Option<RadarStyle>,
    pub(super) hole_size: Option<u8>,
    pub(super) first_slice_angle: Option<u16>,
    pub(super) hi_low_lines: Option<bool>,
    pub(super) up_down_bars: Option<bool>,
    pub(super) drop_lines: Option<bool>,
    pub(super) disp_blanks_as: Option<DispBlanksAs>,
    pub(super) vary_colors: Option<bool>,
    pub(super) data_labels: Option<ChartDataLabels>,
    pub(super) data_table: Option<ChartDataTable>,
    pub(super) view_3d: Option<ChartView3D>,
    pub(super) bar_shape: Option<Bar3DShape>,
}

pub(super) fn group_is_secondary(axis_ids: &[c::AxisId], sec: &[u32]) -> bool {
    !sec.is_empty() && axis_ids.iter().any(|a| sec.contains(&a.val))
}

pub(super) fn read_chart_space(space: &c::ChartSpace) -> ParsedChart {
    let plot = &space.chart.plot_area;

    let secondary_ax_ids: Vec<u32> = plot
        .plot_area_choice2
        .iter()
        .filter_map(|ax| match ax {
            c::PlotAreaChoice2::ValueAxis(v) => matches!(
                v.axis_position.val,
                c::AxisPositionValues::Right | c::AxisPositionValues::Top
            )
            .then_some(v.axis_id.val),
            _ => None,
        })
        .collect();

    let mut kind = ChartKind::Column;
    let mut kind_set = false;
    let mut series: Vec<ChartSeriesInfo> = Vec::new();
    let mut categories_ref: Option<String> = None;
    let mut stacking: Option<ChartStacking> = None;
    let mut gap_width: Option<u16> = None;
    let mut overlap: Option<i8> = None;
    let mut radar_style: Option<RadarStyle> = None;
    let mut hole_size: Option<u8> = None;
    let mut first_slice_angle: Option<u16> = None;
    let mut hi_low_lines: Option<bool> = None;
    let mut up_down_bars: Option<bool> = None;
    let mut drop_lines: Option<bool> = None;
    let mut vary_colors: Option<bool> = None;
    let mut data_labels: Option<ChartDataLabels> = None;
    let mut bar_shape: Option<Bar3DShape> = None;

    for ch in &plot.plot_area_choice1 {
        match ch {
            c::PlotAreaChoice::BarChart(bc) => {
                let this_kind = match bc.bar_direction.val {
                    c::BarDirectionValues::Bar => ChartKind::Bar,
                    c::BarDirectionValues::Column => ChartKind::Column,
                };
                if !kind_set {
                    kind = this_kind;
                    kind_set = true;
                }
                let gsec = group_is_secondary(&bc.axis_id, &secondary_ax_ids);
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
                if vary_colors.is_none() {
                    vary_colors = read_vary_colors(bc.vary_colors.as_ref());
                }
                for s in &bc.bar_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.kind = Some(this_kind);
                    info.axis = gsec.then_some(ChartAxisGroup::Secondary);
                    info.invert_if_negative =
                        read_invert_if_negative(s.invert_if_negative.as_ref());
                    info.data_points = read_data_points(&s.data_point);
                    info.trendline = read_trendline(&s.trendline);
                    info.error_bars = read_error_bars(s.error_bars.as_deref());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(bc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::LineChart(lc) => {
                if !kind_set {
                    kind = ChartKind::Line;
                    kind_set = true;
                }
                let gsec = group_is_secondary(&lc.axis_id, &secondary_ax_ids);
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
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.marker = read_marker(s.marker.as_deref());
                    info.smooth = read_smooth(s.smooth.as_ref());
                    info.kind = Some(ChartKind::Line);
                    info.axis = gsec.then_some(ChartAxisGroup::Secondary);
                    info.data_points = read_data_points(&s.data_point);
                    info.trendline = read_trendline(&s.trendline);
                    info.error_bars = read_error_bars(s.error_bars.as_deref());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(lc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::PieChart(pc) => {
                if !kind_set {
                    kind = ChartKind::Pie;
                    kind_set = true;
                }
                first_slice_angle = pc.first_slice_angle.as_ref().and_then(|f| f.val);
                for s in &pc.pie_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(pc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::DoughnutChart(dc) => {
                if !kind_set {
                    kind = ChartKind::Doughnut;
                    kind_set = true;
                }
                hole_size = Some(dc.hole_size.val);
                first_slice_angle = dc.first_slice_angle.as_ref().and_then(|f| f.val);
                for s in &dc.pie_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(dc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::AreaChart(ac) => {
                if !kind_set {
                    kind = ChartKind::Area;
                    kind_set = true;
                }
                let gsec = group_is_secondary(&ac.axis_id, &secondary_ax_ids);
                stacking = ac
                    .grouping
                    .as_ref()
                    .and_then(|g| g.val.as_ref())
                    .map(grouping_to_stacking);
                for s in &ac.area_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.kind = Some(ChartKind::Area);
                    info.axis = gsec.then_some(ChartAxisGroup::Secondary);
                    info.data_points = read_data_points(&s.data_point);
                    info.trendline = read_trendline(&s.trendline);
                    info.error_bars = read_error_bars(s.error_bars.first());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(ac.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::ScatterChart(sc) => {
                if !kind_set {
                    kind = ChartKind::Scatter;
                    kind_set = true;
                }
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
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.marker = read_marker(s.marker.as_deref());
                    info.smooth = read_smooth(s.smooth.as_ref());
                    info.data_points = read_data_points(&s.data_point);
                    info.trendline = read_trendline(&s.trendline);
                    info.error_bars = read_error_bars(s.error_bars.first());
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(sc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::BubbleChart(bc) => {
                if !kind_set {
                    kind = ChartKind::Bubble;
                    kind_set = true;
                }
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
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.invert_if_negative =
                        read_invert_if_negative(s.invert_if_negative.as_ref());
                    info.data_points = read_data_points(&s.data_point);
                    info.trendline = read_trendline(&s.trendline);
                    info.error_bars = read_error_bars(s.error_bars.first());
                    series.push(info);
                }
                if vary_colors.is_none() {
                    vary_colors = read_vary_colors(bc.vary_colors.as_ref());
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(bc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::RadarChart(rc) => {
                if !kind_set {
                    kind = ChartKind::Radar;
                    kind_set = true;
                }
                radar_style = Some(match rc.radar_style.val {
                    c::RadarStyleValues::Standard => RadarStyle::Standard,
                    c::RadarStyleValues::Marker => RadarStyle::Marker,
                    c::RadarStyleValues::Filled => RadarStyle::Filled,
                });
                for s in &rc.radar_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.marker = read_marker(s.marker.as_deref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(rc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::Bar3DChart(bc) => {
                let this_kind = match bc.bar_direction.val {
                    c::BarDirectionValues::Bar => ChartKind::Bar3D,
                    c::BarDirectionValues::Column => ChartKind::Column3D,
                };
                if !kind_set {
                    kind = this_kind;
                    kind_set = true;
                }
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
                bar_shape = bc
                    .shape
                    .as_ref()
                    .and_then(|s| s.val.as_ref())
                    .map(shape_from);
                if vary_colors.is_none() {
                    vary_colors = read_vary_colors(bc.vary_colors.as_ref());
                }
                for s in &bc.bar_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.invert_if_negative =
                        read_invert_if_negative(s.invert_if_negative.as_ref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(bc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::Line3DChart(lc) => {
                if !kind_set {
                    kind = ChartKind::Line3D;
                    kind_set = true;
                }
                stacking = lc.grouping.val.as_ref().map(grouping_to_stacking);
                if vary_colors.is_none() {
                    vary_colors = read_vary_colors(lc.vary_colors.as_ref());
                }
                for s in &lc.line_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.marker = read_marker(s.marker.as_deref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(lc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::Area3DChart(ac) => {
                if !kind_set {
                    kind = ChartKind::Area3D;
                    kind_set = true;
                }
                stacking = ac
                    .grouping
                    .as_ref()
                    .and_then(|g| g.val.as_ref())
                    .map(grouping_to_stacking);
                if vary_colors.is_none() {
                    vary_colors = read_vary_colors(ac.vary_colors.as_ref());
                }
                for s in &ac.area_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(ac.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::Pie3DChart(pc) => {
                if !kind_set {
                    kind = ChartKind::Pie3D;
                    kind_set = true;
                }
                if vary_colors.is_none() {
                    vary_colors = read_vary_colors(pc.vary_colors.as_ref());
                }
                for s in &pc.pie_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(pc.data_labels.as_deref());
                }
            }
            c::PlotAreaChoice::StockChart(sc) => {
                if !kind_set {
                    kind = ChartKind::Stock;
                    kind_set = true;
                }
                hi_low_lines = Some(sc.high_low_lines.is_some());
                up_down_bars = Some(sc.up_down_bars.is_some());
                drop_lines = Some(sc.drop_lines.is_some());
                for s in &sc.line_chart_series {
                    let mut info = read_series(
                        s.series_text.as_deref(),
                        s.category_axis_data.as_deref(),
                        s.values.as_deref(),
                        &mut categories_ref,
                        s.data_labels.as_deref(),
                    );
                    info.color = read_series_color(s.chart_shape_properties.as_deref());
                    info.line = read_line(s.chart_shape_properties.as_deref());
                    info.marker = read_marker(s.marker.as_deref());
                    info.data_points = read_data_points(&s.data_point);
                    series.push(info);
                }
                if data_labels.is_none() {
                    data_labels = read_data_labels(sc.data_labels.as_deref());
                }
            }
            _ => {}
        }
    }

    let first_kind = series.iter().find_map(|s| s.kind);
    let multi_kind = series.iter().any(|s| s.kind != first_kind);
    let any_secondary = series.iter().any(|s| s.axis.is_some());
    if !multi_kind && !any_secondary {
        for s in &mut series {
            s.kind = None;
        }
    }

    let mut category_axis_title: Option<String> = None;
    let mut value_axis_title: Option<String> = None;
    let mut category_axis: Option<ChartAxisPatch> = None;
    let mut value_axis: Option<ChartAxisPatch> = None;
    for ax in &plot.plot_area_choice2 {
        match ax {
            c::PlotAreaChoice2::CategoryAxis(c) => {
                if c.axis_id.val == SEC_CAT_AX_ID || category_axis.is_some() {
                    continue;
                }
                if let Some(t) = c.title.as_deref() {
                    category_axis_title = extract_title_text(t);
                }
                category_axis = read_cat_axis_patch(c);
            }
            c::PlotAreaChoice2::ValueAxis(v) => {
                if v.axis_id.val == SEC_VAL_AX_ID {
                    continue;
                }
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

    let data_table = read_data_table(plot.data_table.as_deref());
    let view_3d = read_view_3d(space.chart.view3_d.as_deref());

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

    let disp_blanks_as = space
        .chart
        .display_blanks_as
        .as_ref()
        .and_then(|d| d.val.as_ref())
        .map(|v| match v {
            c::DisplayBlanksAsValues::Span => DispBlanksAs::Span,
            c::DisplayBlanksAsValues::Gap => DispBlanksAs::Gap,
            c::DisplayBlanksAsValues::Zero => DispBlanksAs::Zero,
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
        radar_style,
        hole_size,
        first_slice_angle,
        hi_low_lines,
        up_down_bars,
        drop_lines,
        disp_blanks_as,
        vary_colors,
        data_labels,
        data_table,
        view_3d,
        bar_shape,
    }
}

pub(super) fn shape_from(v: &c::ShapeValues) -> Bar3DShape {
    match v {
        c::ShapeValues::Cone => Bar3DShape::Cone,
        c::ShapeValues::ConeToMax => Bar3DShape::ConeToMax,
        c::ShapeValues::Box => Bar3DShape::Box,
        c::ShapeValues::Cylinder => Bar3DShape::Cylinder,
        c::ShapeValues::Pyramid => Bar3DShape::Pyramid,
        c::ShapeValues::PyramidToMaximum => Bar3DShape::PyramidToMaximum,
    }
}

pub(super) fn read_view_3d(v: Option<&c::View3D>) -> Option<ChartView3D> {
    let v = v?;
    let out = ChartView3D {
        rot_x: v.rotate_x.as_ref().and_then(|r| r.val),
        rot_y: v.rotate_y.as_ref().and_then(|r| r.val),
        perspective: v.perspective.as_ref().and_then(|p| p.val),
        right_angle_axes: v
            .right_angle_axes
            .as_ref()
            .and_then(|r| r.val.as_ref())
            .map(|b| b.as_bool()),
        depth_percent: v.depth_percent.as_ref().and_then(|d| d.val),
        height_percent: v.height_percent.as_ref().and_then(|h| h.val),
    };
    if out == ChartView3D::default() {
        None
    } else {
        Some(out)
    }
}

pub(super) fn read_data_table(dt: Option<&c::DataTable>) -> Option<ChartDataTable> {
    let dt = dt?;
    let b = |v: Option<&BooleanValue>| v.map(|x| bool::from(*x));
    let out = ChartDataTable {
        show_horizontal_border: b(dt
            .show_horizontal_border
            .as_ref()
            .and_then(|s| s.val.as_ref())),
        show_vertical_border: b(dt
            .show_vertical_border
            .as_ref()
            .and_then(|s| s.val.as_ref())),
        show_outline: b(dt.show_outline_border.as_ref().and_then(|s| s.val.as_ref())),
        show_keys: b(dt.show_keys.as_ref().and_then(|s| s.val.as_ref())),
    };
    if out == ChartDataTable::default() {
        None
    } else {
        Some(out)
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
        marker: None,
        line: None,
        smooth: None,
        data_points: None,
        kind: None,
        axis: None,
        invert_if_negative: None,
        trendline: None,
        error_bars: None,
    }
}

pub(super) fn trendline_kind_from(v: &c::TrendlineValues) -> TrendlineKind {
    match v {
        c::TrendlineValues::Exponential => TrendlineKind::Exponential,
        c::TrendlineValues::Linear => TrendlineKind::Linear,
        c::TrendlineValues::Logarithmic => TrendlineKind::Logarithmic,
        c::TrendlineValues::MovingAverage => TrendlineKind::MovingAverage,
        c::TrendlineValues::Polynomial => TrendlineKind::Polynomial,
        c::TrendlineValues::Power => TrendlineKind::Power,
    }
}

pub(super) fn read_trendline(tls: &[c::Trendline]) -> Option<ChartTrendline> {
    let t = tls.first()?;
    Some(ChartTrendline {
        kind: t
            .trendline_type
            .val
            .as_ref()
            .map(trendline_kind_from)
            .unwrap_or(TrendlineKind::Linear),
        name: t.trendline_name.clone(),
        polynomial_order: t.polynomial_order.as_ref().map(|o| o.val),
        period: t.period.as_ref().map(|p| p.val),
        forward: t.forward.as_ref().map(|f| f.val),
        backward: t.backward.as_ref().map(|b| b.val),
        intercept: t.intercept.as_ref().map(|i| i.val),
        display_equation: t
            .display_equation
            .as_ref()
            .and_then(|d| d.val.as_ref())
            .map(|b| b.as_bool()),
        display_r_squared: t
            .display_r_squared_value
            .as_ref()
            .and_then(|d| d.val.as_ref())
            .map(|b| b.as_bool()),
    })
}

pub(super) fn read_num_source_ref(formula: Option<&str>) -> Option<String> {
    let f = formula?;
    (!f.is_empty()).then(|| f.to_string())
}

pub(super) fn read_error_bars(eb: Option<&c::ErrorBars>) -> Option<ChartErrorBars> {
    let e = eb?;
    let read_plus = |p: &c::PlusChoice| match p {
        c::PlusChoice::NumberReference(nr) => {
            (read_num_source_ref(Some(nr.formula.as_str())), None)
        }
        c::PlusChoice::NumberLiteral(nl) => (None, Some(read_num_literal(nl))),
    };
    let read_minus = |m: &c::MinusChoice| match m {
        c::MinusChoice::NumberReference(nr) => {
            (read_num_source_ref(Some(nr.formula.as_str())), None)
        }
        c::MinusChoice::NumberLiteral(nl) => (None, Some(read_num_literal(nl))),
    };
    let (plus_ref, plus_values) = e
        .plus
        .as_deref()
        .and_then(|p| p.plus_choice.as_ref())
        .map(read_plus)
        .unwrap_or((None, None));
    let (minus_ref, minus_values) = e
        .minus
        .as_deref()
        .and_then(|m| m.minus_choice.as_ref())
        .map(read_minus)
        .unwrap_or((None, None));
    Some(ChartErrorBars {
        direction: e.error_direction.as_ref().map(|d| match d.val {
            c::ErrorBarDirectionValues::X => ChartErrorDirection::X,
            c::ErrorBarDirectionValues::Y => ChartErrorDirection::Y,
        }),
        bar_type: match e.error_bar_type.val {
            c::ErrorBarValues::Both => ChartErrorBarType::Both,
            c::ErrorBarValues::Minus => ChartErrorBarType::Minus,
            c::ErrorBarValues::Plus => ChartErrorBarType::Plus,
        },
        value_type: match e.error_bar_value_type.val {
            c::ErrorValues::Custom => ChartErrorValueType::Custom,
            c::ErrorValues::FixedValue => ChartErrorValueType::FixedValue,
            c::ErrorValues::Percentage => ChartErrorValueType::Percentage,
            c::ErrorValues::StandardDeviation => ChartErrorValueType::StandardDeviation,
            c::ErrorValues::StandardError => ChartErrorValueType::StandardError,
        },
        value: e.error_bar_value.as_ref().map(|v| v.val),
        no_end_cap: e
            .no_end_cap
            .as_ref()
            .and_then(|n| n.val.as_ref())
            .map(|b| b.as_bool()),
        plus_ref,
        minus_ref,
        plus_values,
        minus_values,
    })
}

pub(super) fn read_num_literal(nl: &c::NumberLiteral) -> Vec<f64> {
    nl.numeric_point
        .iter()
        .filter_map(|p| p.numeric_value.parse::<f64>().ok())
        .collect()
}

pub(super) fn marker_style_from(v: &c::MarkerStyleValues) -> MarkerStyle {
    match v {
        c::MarkerStyleValues::Auto => MarkerStyle::Auto,
        c::MarkerStyleValues::Circle => MarkerStyle::Circle,
        c::MarkerStyleValues::Dash => MarkerStyle::Dash,
        c::MarkerStyleValues::Diamond => MarkerStyle::Diamond,
        c::MarkerStyleValues::Dot => MarkerStyle::Dot,
        c::MarkerStyleValues::None => MarkerStyle::None,
        c::MarkerStyleValues::Picture => MarkerStyle::Picture,
        c::MarkerStyleValues::Plus => MarkerStyle::Plus,
        c::MarkerStyleValues::Square => MarkerStyle::Square,
        c::MarkerStyleValues::Star => MarkerStyle::Star,
        c::MarkerStyleValues::Triangle => MarkerStyle::Triangle,
        c::MarkerStyleValues::X => MarkerStyle::X,
    }
}

pub(super) fn read_marker(m: Option<&c::Marker>) -> Option<ChartMarker> {
    let m = m?;
    let out = ChartMarker {
        style: m.symbol.as_ref().map(|s| marker_style_from(&s.val)),
        size: m.size.as_ref().and_then(|s| s.val),
    };
    if out == ChartMarker::default() {
        None
    } else {
        Some(out)
    }
}

pub(super) fn read_smooth(s: Option<&c::Smooth>) -> Option<bool> {
    s?.val.as_ref().map(|v| v.as_bool())
}

pub(super) fn read_vary_colors(v: Option<&c::VaryColors>) -> Option<bool> {
    v?.val.as_ref().map(|b| b.as_bool())
}

pub(super) fn read_invert_if_negative(v: Option<&c::InvertIfNegative>) -> Option<bool> {
    v?.val.as_ref().map(|b| b.as_bool())
}

pub(super) fn line_dash_from(v: &a::PresetLineDashValues) -> LineDash {
    match v {
        a::PresetLineDashValues::Solid => LineDash::Solid,
        a::PresetLineDashValues::Dot => LineDash::Dot,
        a::PresetLineDashValues::Dash => LineDash::Dash,
        a::PresetLineDashValues::LargeDash => LineDash::LargeDash,
        a::PresetLineDashValues::DashDot => LineDash::DashDot,
        a::PresetLineDashValues::LargeDashDot => LineDash::LargeDashDot,
        a::PresetLineDashValues::LargeDashDotDot => LineDash::LargeDashDotDot,
        a::PresetLineDashValues::SystemDash => LineDash::SystemDash,
        a::PresetLineDashValues::SystemDot => LineDash::SystemDot,
        a::PresetLineDashValues::SystemDashDot => LineDash::SystemDashDot,
        a::PresetLineDashValues::SystemDashDotDot => LineDash::SystemDashDotDot,
    }
}

pub(super) fn read_line(sp: Option<&c::ChartShapeProperties>) -> Option<ChartLine> {
    let outline = sp?.outline.as_deref()?;
    let none = matches!(outline.outline_choice1, Some(a::OutlineChoice::NoFill(_))).then_some(true);
    let dash = match outline.outline_choice2.as_ref() {
        Some(a::OutlineChoice2::PresetDash(d)) => d.val.as_ref().map(line_dash_from),
        _ => None,
    };
    let out = ChartLine {
        width_emu: outline.width,
        dash,
        none,
    };
    (out != ChartLine::default()).then_some(out)
}

pub(super) fn read_data_points(dps: &[c::DataPoint]) -> Option<Vec<ChartDataPoint>> {
    let mut out: Vec<ChartDataPoint> = Vec::new();
    for dp in dps {
        let sp = dp.chart_shape_properties.as_deref();
        let fill = read_shape_fill(sp);
        let explosion = dp.explosion.as_ref().map(|e| e.val);
        if fill.is_some() || explosion.is_some() {
            out.push(ChartDataPoint {
                index: dp.index.val,
                fill,
                explosion,
            });
        }
    }
    (!out.is_empty()).then_some(out)
}

pub(super) fn read_shape_fill(sp: Option<&c::ChartShapeProperties>) -> Option<String> {
    let sp = sp?;
    match sp.chart_shape_properties_choice2.as_ref()? {
        c::ChartShapePropertiesChoice2::NoFill(_) => Some("none".to_string()),
        c::ChartShapePropertiesChoice2::SolidFill(sf) => {
            let a::SolidFillChoice::RgbColorModelHex(rgb) = sf.solid_fill_choice.as_ref()? else {
                return None;
            };
            Some(rgb.val.to_string().to_uppercase())
        }
        _ => None,
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
        marker: None,
        line: None,
        smooth: None,
        data_points: None,
        kind: None,
        axis: None,
        invert_if_negative: None,
        trendline: None,
        error_bars: None,
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
        number_format: seq.numbering_format.as_ref().map(|n| n.format_code.clone()),
        per_point: dl.data_label.iter().map(read_point_data_label).collect(),
    };
    if out == ChartDataLabels::default() {
        None
    } else {
        Some(out)
    }
}

pub(super) fn read_point_data_label(d: &c::DataLabel) -> ChartDataLabel {
    let index = d.index.val;
    let bv = |b: Option<&BooleanValue>| b.map(|v| bool::from(*v));
    match d.data_label_choice.as_ref() {
        Some(c::DataLabelChoice::Delete(_)) => ChartDataLabel {
            index,
            delete: true,
            ..Default::default()
        },
        Some(c::DataLabelChoice::Sequence(seq)) => ChartDataLabel {
            index,
            delete: false,
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
            number_format: seq.numbering_format.as_ref().map(|n| n.format_code.clone()),
        },
        None => ChartDataLabel {
            index,
            ..Default::default()
        },
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

pub(super) fn built_in_unit_from(v: &c::BuiltInUnitValues) -> BuiltInUnit {
    match v {
        c::BuiltInUnitValues::Hundreds => BuiltInUnit::Hundreds,
        c::BuiltInUnitValues::Thousands => BuiltInUnit::Thousands,
        c::BuiltInUnitValues::TenThousands => BuiltInUnit::TenThousands,
        c::BuiltInUnitValues::HundredThousands => BuiltInUnit::HundredThousands,
        c::BuiltInUnitValues::Millions => BuiltInUnit::Millions,
        c::BuiltInUnitValues::TenMillions => BuiltInUnit::TenMillions,
        c::BuiltInUnitValues::HundredMillions => BuiltInUnit::HundredMillions,
        c::BuiltInUnitValues::Billions => BuiltInUnit::Billions,
        c::BuiltInUnitValues::Trillions => BuiltInUnit::Trillions,
    }
}

pub(super) fn read_display_units(du: Option<&c::DisplayUnits>) -> Option<DisplayUnits> {
    match du?.display_units_choice.as_ref()? {
        c::DisplayUnitsChoice::BuiltInUnit(b) => Some(DisplayUnits::Builtin(built_in_unit_from(
            b.val.as_ref().unwrap_or(&c::BuiltInUnitValues::Hundreds),
        ))),
        c::DisplayUnitsChoice::CustomDisplayUnit(cu) => Some(DisplayUnits::Custom(cu.val)),
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
        label_rotation: read_axis_label_rotation(ax.text_properties.as_deref()),
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
        display_units: read_display_units(ax.display_units.as_deref()),
        label_rotation: read_axis_label_rotation(ax.text_properties.as_deref()),
    };
    p.title = p.title.filter(|t| !t.is_empty());
    if p == ChartAxisPatch::default() {
        None
    } else {
        Some(p)
    }
}

pub(super) fn read_axis_label_rotation(txpr: Option<&c::TextProperties>) -> Option<i32> {
    let rot = txpr?.body_properties.rotation?;
    Some(rot / 60000)
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
