use super::*;

pub(super) fn set_axis_title(slot: &mut Option<Box<c::Title>>, text: &str) {
    if text.is_empty() {
        *slot = None;
    } else {
        *slot = Some(Box::new(build_title(text)));
    }
}

pub(super) fn validate_chart_series(
    sheet: &str,
    kind: ChartKind,
    series: &[ChartSeriesPatch],
) -> Result<()> {
    if series.is_empty() {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            "chart must have at least one series",
        )
        .with_sheet(sheet));
    }
    for s in series {
        if s.values_ref.trim().is_empty() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                "chart series values_ref must not be empty",
            )
            .with_sheet(sheet));
        }
        if matches!(kind, ChartKind::Scatter | ChartKind::Bubble)
            && s.x_values_ref
                .as_deref()
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                "scatter/bubble chart series require x_values_ref",
            )
            .with_sheet(sheet));
        }
        if matches!(kind, ChartKind::Bubble)
            && s.bubble_sizes_ref
                .as_deref()
                .map(|v| v.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                "bubble chart series require bubble_sizes_ref",
            )
            .with_sheet(sheet));
        }
        if let Some(color) = s.color.as_deref() {
            if !is_valid_hex_color(color) {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidChart,
                    format!(
                        "chart series color must be 6-hex RRGGBB or 8-hex AARRGGBB, got: {color}"
                    ),
                )
                .with_sheet(sheet));
            }
        }
        if let Some(points) = s.data_points.as_ref() {
            for p in points {
                if let Some(fill) = p.fill.as_deref() {
                    if !fill.trim().eq_ignore_ascii_case("none") && !is_valid_hex_color(fill) {
                        return Err(ApiError::new(
                            ApiErrorCode::InvalidChart,
                            format!(
                                "chart data point fill must be 6-hex RRGGBB, 8-hex AARRGGBB, or \"none\", got: {fill}"
                            ),
                        )
                        .with_sheet(sheet));
                    }
                }
            }
        }
        if let Some(size) = s.marker.as_ref().and_then(|m| m.size) {
            if !(2..=72).contains(&size) {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidChart,
                    format!("chart series marker size must be 2..=72, got: {size}"),
                )
                .with_sheet(sheet));
            }
        }
        if (s.kind.is_some() || s.axis.is_some()) && !is_cartesian(kind) {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                "per-series kind/axis (combo charts) require a column/bar/line/area chart",
            )
            .with_sheet(sheet));
        }
        if let Some(k) = s.kind {
            if !is_cartesian(k) {
                return Err(ApiError::new(
                    ApiErrorCode::InvalidChart,
                    format!("per-series kind must be column/bar/line/area, got: {k:?}"),
                )
                .with_sheet(sheet));
            }
        }
    }
    Ok(())
}

pub(super) fn is_cartesian(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area
    )
}

pub(super) fn effective_series_kind(chart_kind: ChartKind, s: &ChartSeriesPatch) -> ChartKind {
    s.kind.unwrap_or(chart_kind)
}

pub(super) fn series_secondary(s: &ChartSeriesPatch) -> bool {
    matches!(s.axis, Some(ChartAxisGroup::Secondary))
}

pub(super) fn validate_bar_options(
    sheet: &str,
    gap_width: Option<u16>,
    overlap: Option<i8>,
) -> Result<()> {
    if let Some(g) = gap_width {
        if g > 500 {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart gap_width must be 0..=500, got: {g}"),
            )
            .with_sheet(sheet));
        }
    }
    if let Some(o) = overlap {
        if !(-100..=100).contains(&o) {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart overlap must be -100..=100, got: {o}"),
            )
            .with_sheet(sheet));
        }
    }
    Ok(())
}

pub(super) fn bar_option_for_kind<T>(kind: ChartKind, value: Option<T>) -> Option<T> {
    match kind {
        ChartKind::Column | ChartKind::Bar => value,
        _ => None,
    }
}

pub(super) fn stacking_for_kind(
    kind: ChartKind,
    requested: Option<ChartStacking>,
) -> Option<ChartStacking> {
    match kind {
        ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area => requested,
        _ => None,
    }
}

pub(super) fn line_area_grouping(stacking: Option<ChartStacking>) -> c::GroupingValues {
    match stacking {
        Some(ChartStacking::Stacked) => c::GroupingValues::Stacked,
        Some(ChartStacking::PercentStacked) => c::GroupingValues::PercentStacked,
        _ => c::GroupingValues::Standard,
    }
}

pub(super) fn bar_grouping(stacking: Option<ChartStacking>) -> c::BarGroupingValues {
    match stacking {
        Some(ChartStacking::Stacked) => c::BarGroupingValues::Stacked,
        Some(ChartStacking::PercentStacked) => c::BarGroupingValues::PercentStacked,
        _ => c::BarGroupingValues::Clustered,
    }
}

pub(super) fn legend_pos_to(p: ChartLegendPosition) -> c::LegendPositionValues {
    match p {
        ChartLegendPosition::Right => c::LegendPositionValues::Right,
        ChartLegendPosition::Left => c::LegendPositionValues::Left,
        ChartLegendPosition::Top => c::LegendPositionValues::Top,
        ChartLegendPosition::Bottom => c::LegendPositionValues::Bottom,
        ChartLegendPosition::TopRight => c::LegendPositionValues::TopRight,
        ChartLegendPosition::None => c::LegendPositionValues::Right,
    }
}

pub(super) fn build_chart_space(patch: &ChartPatch) -> c::ChartSpace {
    let plot_charts = build_plot_charts(patch);
    let cat_axis = merge_axis_title(patch.category_axis.as_ref(), &patch.category_axis_title);
    let val_axis = merge_axis_title(patch.value_axis.as_ref(), &patch.value_axis_title);

    let mut plot_area = c::PlotArea {
        layout: Some(Box::new(c::Layout::default())),
        plot_area_choice1: plot_charts,
        plot_area_choice2: Vec::new(),
        ..Default::default()
    };

    match patch.kind {
        ChartKind::Pie | ChartKind::Doughnut => {}
        ChartKind::Scatter | ChartKind::Bubble => {
            let mut bottom = build_val_axis_xy(CAT_AX_ID, VAL_AX_ID, c::AxisPositionValues::Bottom);
            if let Some(p) = &cat_axis {
                apply_val_axis_patch(&mut bottom, p);
            }
            let mut left = build_val_axis_xy(VAL_AX_ID, CAT_AX_ID, c::AxisPositionValues::Left);
            if let Some(p) = &val_axis {
                apply_val_axis_patch(&mut left, p);
            }
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::ValueAxis(Box::new(bottom)));
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::ValueAxis(Box::new(left)));
        }
        _ => {
            let mut cat = build_cat_axis();
            if let Some(p) = &cat_axis {
                apply_cat_axis_patch(&mut cat, p);
            }
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::CategoryAxis(Box::new(cat)));
            let mut val = build_val_axis();
            if let Some(p) = &val_axis {
                apply_val_axis_patch(&mut val, p);
            }
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::ValueAxis(Box::new(val)));
            if patch.series.iter().any(series_secondary) {
                plot_area
                    .plot_area_choice2
                    .push(c::PlotAreaChoice2::ValueAxis(
                        Box::new(build_sec_val_axis()),
                    ));
                plot_area
                    .plot_area_choice2
                    .push(c::PlotAreaChoice2::CategoryAxis(Box::new(
                        build_sec_cat_axis(),
                    )));
            }
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

pub(super) fn build_plot_charts(patch: &ChartPatch) -> Vec<c::PlotAreaChoice> {
    match patch.kind {
        ChartKind::Pie
        | ChartKind::Doughnut
        | ChartKind::Scatter
        | ChartKind::Bubble
        | ChartKind::Radar => {
            vec![build_single_plot_chart(patch)]
        }
        _ => build_cartesian_plot_charts(patch),
    }
}

pub(super) fn radar_style_to(s: RadarStyle) -> c::RadarStyleValues {
    match s {
        RadarStyle::Standard => c::RadarStyleValues::Standard,
        RadarStyle::Marker => c::RadarStyleValues::Marker,
        RadarStyle::Filled => c::RadarStyleValues::Filled,
    }
}

pub(super) fn build_radar_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::RadarChartSeries {
    c::RadarChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        marker: build_marker(s.marker.as_ref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

pub(super) fn build_cartesian_plot_charts(patch: &ChartPatch) -> Vec<c::PlotAreaChoice> {
    let mut groups: Vec<(ChartKind, bool, Vec<usize>)> = Vec::new();
    for (i, s) in patch.series.iter().enumerate() {
        let k = effective_series_kind(patch.kind, s);
        let sec = series_secondary(s);
        if let Some(g) = groups.iter_mut().find(|g| g.0 == k && g.1 == sec) {
            g.2.push(i);
        } else {
            groups.push((k, sec, vec![i]));
        }
    }
    groups
        .iter()
        .enumerate()
        .map(|(gi, (k, sec, idxs))| build_cartesian_group(*k, *sec, idxs, patch, gi == 0))
        .collect()
}

pub(super) fn build_cartesian_group(
    kind: ChartKind,
    secondary: bool,
    idxs: &[usize],
    patch: &ChartPatch,
    attach_chart_dl: bool,
) -> c::PlotAreaChoice {
    let (cat_id, val_id) = if secondary {
        (SEC_CAT_AX_ID, SEC_VAL_AX_ID)
    } else {
        (CAT_AX_ID, VAL_AX_ID)
    };
    let cat_ref = patch.categories_ref.as_deref();
    let dl = if attach_chart_dl {
        build_data_labels(patch.data_labels.as_ref())
    } else {
        None
    };
    let series = || idxs.iter().map(|&i| (i, &patch.series[i]));
    match kind {
        ChartKind::Line => c::PlotAreaChoice::LineChart(Box::new(c::LineChart {
            grouping: Box::new(c::Grouping {
                val: Some(line_area_grouping(patch.stacking)),
            }),
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(false)),
            }),
            line_chart_series: series()
                .map(|(i, s)| build_line_series(i, s, cat_ref))
                .collect(),
            data_labels: dl,
            show_marker: Some(c::ShowMarker {
                val: Some(BooleanValue::from_bool(true)),
            }),
            axis_id: vec![axis_id(cat_id), axis_id(val_id)],
            ..Default::default()
        })),
        ChartKind::Area => c::PlotAreaChoice::AreaChart(Box::new(c::AreaChart {
            grouping: Some(c::Grouping {
                val: Some(line_area_grouping(patch.stacking)),
            }),
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(false)),
            }),
            area_chart_series: series()
                .map(|(i, s)| build_area_series(i, s, cat_ref))
                .collect(),
            data_labels: dl,
            axis_id: vec![axis_id(cat_id), axis_id(val_id)],
            ..Default::default()
        })),
        _ => c::PlotAreaChoice::BarChart(Box::new(c::BarChart {
            bar_direction: Box::new(c::BarDirection {
                val: if matches!(kind, ChartKind::Bar) {
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
            bar_chart_series: series()
                .map(|(i, s)| build_bar_series(i, s, cat_ref))
                .collect(),
            data_labels: dl,
            gap_width: patch.gap_width.map(|g| c::GapWidth { val: Some(g) }),
            overlap: patch
                .overlap
                .map(|o| c::Overlap { val: Some(o) })
                .or_else(|| {
                    matches!(
                        patch.stacking,
                        Some(ChartStacking::Stacked | ChartStacking::PercentStacked)
                    )
                    .then(|| c::Overlap { val: Some(100) })
                }),
            axis_id: vec![axis_id(cat_id), axis_id(val_id)],
            ..Default::default()
        })),
    }
}

pub(super) fn build_single_plot_chart(patch: &ChartPatch) -> c::PlotAreaChoice {
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
        ChartKind::Radar => c::PlotAreaChoice::RadarChart(Box::new(c::RadarChart {
            radar_style: Box::new(c::RadarStyle {
                val: radar_style_to(patch.radar_style.unwrap_or(RadarStyle::Standard)),
            }),
            vary_colors: Some(c::VaryColors {
                val: Some(BooleanValue::from_bool(false)),
            }),
            radar_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_radar_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
            ..Default::default()
        })),
        ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area => {
            unreachable!("cartesian kinds are built via build_cartesian_plot_charts")
        }
    }
}

pub(super) fn build_data_labels(dl: Option<&ChartDataLabels>) -> Option<Box<c::DataLabels>> {
    let dl = dl?;
    fn b(v: Option<bool>) -> Option<BooleanValue> {
        v.map(BooleanValue::from_bool)
    }
    let seq = c::DataLabelsChoiceSequence {
        numbering_format: dl.number_format.as_ref().map(|nf| c::NumberingFormat {
            format_code: nf.clone(),
            source_linked: Some(BooleanValue::from_bool(false)),
        }),
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

pub(super) fn data_label_pos_to(p: ChartDataLabelPosition) -> c::DataLabelPositionValues {
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

pub(super) fn axis_id(val: u32) -> c::AxisId {
    c::AxisId { val }
}

pub(super) fn build_series_text(s: &ChartSeriesPatch) -> Option<Box<c::SeriesText>> {
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

pub(super) fn build_categories(categories_ref: Option<&str>) -> Option<Box<c::CategoryAxisData>> {
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

pub(super) fn build_values(values_ref: &str) -> Box<c::Values> {
    Box::new(c::Values {
        values_choice: Some(c::ValuesChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: values_ref.to_string(),
                ..Default::default()
            },
        ))),
    })
}

pub(super) fn build_bar_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::BarChartSeries {
    c::BarChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

pub(super) fn build_line_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::LineChartSeries {
    c::LineChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        marker: build_marker(s.marker.as_ref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

pub(super) fn marker_style_to(s: MarkerStyle) -> c::MarkerStyleValues {
    match s {
        MarkerStyle::Auto => c::MarkerStyleValues::Auto,
        MarkerStyle::Circle => c::MarkerStyleValues::Circle,
        MarkerStyle::Dash => c::MarkerStyleValues::Dash,
        MarkerStyle::Diamond => c::MarkerStyleValues::Diamond,
        MarkerStyle::Dot => c::MarkerStyleValues::Dot,
        MarkerStyle::None => c::MarkerStyleValues::None,
        MarkerStyle::Picture => c::MarkerStyleValues::Picture,
        MarkerStyle::Plus => c::MarkerStyleValues::Plus,
        MarkerStyle::Square => c::MarkerStyleValues::Square,
        MarkerStyle::Star => c::MarkerStyleValues::Star,
        MarkerStyle::Triangle => c::MarkerStyleValues::Triangle,
        MarkerStyle::X => c::MarkerStyleValues::X,
    }
}

pub(super) fn build_marker(m: Option<&ChartMarker>) -> Option<Box<c::Marker>> {
    let m = m?;
    if m.style.is_none() && m.size.is_none() {
        return None;
    }
    Some(Box::new(c::Marker {
        symbol: m.style.map(|s| c::Symbol {
            val: marker_style_to(s),
        }),
        size: m.size.map(|sz| c::Size { val: Some(sz) }),
        ..Default::default()
    }))
}

pub(super) fn build_area_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::AreaChartSeries {
    c::AreaChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

pub(super) fn build_pie_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::PieChartSeries {
    c::PieChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

pub(super) fn build_scatter_series(idx: usize, s: &ChartSeriesPatch) -> c::ScatterChartSeries {
    c::ScatterChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        marker: build_marker(s.marker.as_ref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        x_values: s.x_values_ref.as_deref().map(build_x_values),
        y_values: Some(build_y_values(&s.values_ref)),
        smooth: Some(c::Smooth {
            val: Some(BooleanValue::from_bool(false)),
        }),
        ..Default::default()
    }
}

pub(super) fn build_bubble_series(idx: usize, s: &ChartSeriesPatch) -> c::BubbleChartSeries {
    c::BubbleChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape(s.color.as_deref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        x_values: s.x_values_ref.as_deref().map(build_x_values),
        y_values: Some(build_y_values(&s.values_ref)),
        bubble_size: s.bubble_sizes_ref.as_deref().map(build_bubble_size),
        ..Default::default()
    }
}

pub(super) fn build_x_values(r: &str) -> Box<c::XValues> {
    Box::new(c::XValues {
        x_values_choice: Some(c::XValuesChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    })
}

pub(super) fn build_y_values(r: &str) -> Box<c::YValues> {
    Box::new(c::YValues {
        y_values_choice: Some(c::YValuesChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    })
}

pub(super) fn build_bubble_size(r: &str) -> Box<c::BubbleSize> {
    Box::new(c::BubbleSize {
        bubble_size_choice: Some(c::BubbleSizeChoice::NumberReference(Box::new(
            c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            },
        ))),
    })
}

pub(super) fn build_data_points(points: &Option<Vec<ChartDataPoint>>) -> Vec<c::DataPoint> {
    let Some(points) = points else {
        return Vec::new();
    };
    points
        .iter()
        .map(|p| c::DataPoint {
            index: Box::new(c::Index { val: p.index }),
            chart_shape_properties: p.fill.as_deref().and_then(build_point_shape),
            ..Default::default()
        })
        .collect()
}

pub(super) fn build_point_shape(fill: &str) -> Option<Box<c::ChartShapeProperties>> {
    if fill.trim().eq_ignore_ascii_case("none") {
        return Some(Box::new(c::ChartShapeProperties {
            chart_shape_properties_choice2: Some(c::ChartShapePropertiesChoice2::NoFill(Box::new(
                a::NoFill {
                    ..Default::default()
                },
            ))),
            ..Default::default()
        }));
    }
    build_series_shape(Some(fill))
}

pub(super) fn build_series_shape(color: Option<&str>) -> Option<Box<c::ChartShapeProperties>> {
    let hex = color?;
    let val = normalize_chart_hex(hex)?;
    let solid = a::SolidFill {
        solid_fill_choice: Some(a::SolidFillChoice::RgbColorModelHex(Box::new(
            a::RgbColorModelHex {
                val,
                ..Default::default()
            },
        ))),
        ..Default::default()
    };
    Some(Box::new(c::ChartShapeProperties {
        chart_shape_properties_choice2: Some(c::ChartShapePropertiesChoice2::SolidFill(Box::new(
            solid,
        ))),
        ..Default::default()
    }))
}

pub(super) fn normalize_chart_hex(s: &str) -> Option<String> {
    let trimmed = s.trim().trim_start_matches('#');
    let hex = if trimmed.len() == 8 {
        &trimmed[2..]
    } else {
        trimmed
    };
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(hex.to_ascii_uppercase())
    } else {
        None
    }
}

pub(super) fn is_valid_hex_color(s: &str) -> bool {
    normalize_chart_hex(s).is_some()
}

pub(super) fn build_title(text: &str) -> c::Title {
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

pub(super) fn build_legend(pos: c::LegendPositionValues) -> c::Legend {
    c::Legend {
        legend_position: Some(c::LegendPosition { val: Some(pos) }),
        overlay: Some(c::Overlay {
            val: Some(BooleanValue::from_bool(false)),
        }),
        ..Default::default()
    }
}

pub(super) fn build_cat_axis() -> c::CategoryAxis {
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
        crossing_axis: Box::new(c::CrossingAxis { val: VAL_AX_ID }),
        ..Default::default()
    }
}

pub(super) fn build_val_axis() -> c::ValueAxis {
    build_val_axis_xy(VAL_AX_ID, CAT_AX_ID, c::AxisPositionValues::Left)
}

pub(super) fn build_val_axis_xy(id: u32, cross: u32, pos: c::AxisPositionValues) -> c::ValueAxis {
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
        crossing_axis: Box::new(c::CrossingAxis { val: cross }),
        ..Default::default()
    }
}

pub(super) fn build_sec_val_axis() -> c::ValueAxis {
    c::ValueAxis {
        axis_id: Box::new(axis_id(SEC_VAL_AX_ID)),
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
            val: c::AxisPositionValues::Right,
        }),
        crossing_axis: Box::new(c::CrossingAxis { val: SEC_CAT_AX_ID }),
        value_axis_choice: Some(c::ValueAxisChoice::Crosses(Box::new(c::Crosses {
            val: c::CrossesValues::Maximum,
        }))),
        ..Default::default()
    }
}

pub(super) fn build_sec_cat_axis() -> c::CategoryAxis {
    c::CategoryAxis {
        axis_id: Box::new(axis_id(SEC_CAT_AX_ID)),
        scaling: Box::new(c::Scaling {
            orientation: Some(c::Orientation {
                val: Some(c::OrientationValues::MinMax),
            }),
            ..Default::default()
        }),
        delete: Some(c::Delete {
            val: Some(BooleanValue::from_bool(true)),
        }),
        axis_position: Box::new(c::AxisPosition {
            val: c::AxisPositionValues::Bottom,
        }),
        crossing_axis: Box::new(c::CrossingAxis { val: SEC_VAL_AX_ID }),
        ..Default::default()
    }
}

pub(super) fn tick_mark_to(v: TickMark) -> c::TickMarkValues {
    match v {
        TickMark::Cross => c::TickMarkValues::Cross,
        TickMark::Inside => c::TickMarkValues::Inside,
        TickMark::Outside => c::TickMarkValues::Outside,
        TickMark::None => c::TickMarkValues::None,
    }
}

pub(super) fn tick_label_pos_to(v: TickLabelPosition) -> c::TickLabelPositionValues {
    match v {
        TickLabelPosition::High => c::TickLabelPositionValues::High,
        TickLabelPosition::Low => c::TickLabelPositionValues::Low,
        TickLabelPosition::NextTo => c::TickLabelPositionValues::NextTo,
        TickLabelPosition::None => c::TickLabelPositionValues::None,
    }
}

pub(super) fn cross_between_to(v: CrossBetween) -> c::CrossBetweenValues {
    match v {
        CrossBetween::Between => c::CrossBetweenValues::Between,
        CrossBetween::MidpointCategory => c::CrossBetweenValues::MidpointCategory,
    }
}

pub(super) fn merge_axis_title(
    axis: Option<&ChartAxisPatch>,
    legacy_title: &Option<String>,
) -> Option<ChartAxisPatch> {
    match (axis, legacy_title) {
        (None, None) => None,
        (None, Some(t)) => Some(ChartAxisPatch {
            title: Some(t.clone()),
            ..Default::default()
        }),
        (Some(p), legacy) => {
            let mut p = p.clone();
            if p.title.is_none() {
                p.title = legacy.clone();
            }
            Some(p)
        }
    }
}

macro_rules! apply_axis_common {
    ($ax:expr, $p:expr) => {{
        if let Some(min) = $p.min {
            $ax.scaling.min_axis_value = Some(c::MinAxisValue { val: min });
        }
        if let Some(max) = $p.max {
            $ax.scaling.max_axis_value = Some(c::MaxAxisValue { val: max });
        }
        if let Some(rev) = $p.reversed {
            $ax.scaling.orientation = Some(c::Orientation {
                val: Some(if rev {
                    c::OrientationValues::MaxMin
                } else {
                    c::OrientationValues::MinMax
                }),
            });
        }
        if let Some(hidden) = $p.hidden {
            $ax.delete = Some(c::Delete {
                val: Some(BooleanValue::from_bool(hidden)),
            });
        }
        if let Some(on) = $p.major_gridlines {
            $ax.major_gridlines = if on {
                Some(Box::new(c::MajorGridlines::default()))
            } else {
                None
            };
        }
        if let Some(on) = $p.minor_gridlines {
            $ax.minor_gridlines = if on {
                Some(Box::new(c::MinorGridlines::default()))
            } else {
                None
            };
        }
        if let Some(t) = &$p.title {
            set_axis_title(&mut $ax.title, t);
        }
        if let Some(nf) = &$p.number_format {
            $ax.numbering_format = Some(c::NumberingFormat {
                format_code: nf.clone(),
                source_linked: Some(BooleanValue::from_bool(false)),
            });
        }
        if let Some(tm) = $p.major_tick_mark {
            $ax.major_tick_mark = Some(c::MajorTickMark {
                val: Some(tick_mark_to(tm)),
            });
        }
        if let Some(tm) = $p.minor_tick_mark {
            $ax.minor_tick_mark = Some(c::MinorTickMark {
                val: Some(tick_mark_to(tm)),
            });
        }
        if let Some(tlp) = $p.tick_label_position {
            $ax.tick_label_position = Some(c::TickLabelPosition {
                val: Some(tick_label_pos_to(tlp)),
            });
        }
    }};
}

pub(super) fn apply_cat_axis_patch(ax: &mut c::CategoryAxis, p: &ChartAxisPatch) {
    apply_axis_common!(ax, p);
    if let Some(at) = p.crosses_at {
        ax.category_axis_choice = Some(c::CategoryAxisChoice::CrossesAt(Box::new(c::CrossesAt {
            val: at,
        })));
    }
}

pub(super) fn apply_val_axis_patch(ax: &mut c::ValueAxis, p: &ChartAxisPatch) {
    apply_axis_common!(ax, p);
    if let Some(lb) = p.log_base {
        ax.scaling.log_base = Some(c::LogBase { val: lb });
    }
    if let Some(mu) = p.major_unit {
        ax.major_unit = Some(c::MajorUnit { val: mu });
    }
    if let Some(mu) = p.minor_unit {
        ax.minor_unit = Some(c::MinorUnit { val: mu });
    }
    if let Some(cb) = p.cross_between {
        ax.cross_between = Some(c::CrossBetween {
            val: cross_between_to(cb),
        });
    }
    if let Some(at) = p.crosses_at {
        ax.value_axis_choice = Some(c::ValueAxisChoice::CrossesAt(Box::new(c::CrossesAt {
            val: at,
        })));
    }
}

pub(super) fn build_two_cell_anchor(
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
