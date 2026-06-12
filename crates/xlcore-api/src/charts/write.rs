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
    if matches!(kind, ChartKind::Stock) && !(3..=6).contains(&series.len()) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!(
                "stock chart requires 3..=6 series (high-low-close, open-high-low-close, or volume + OHLC), got: {}",
                series.len()
            ),
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
        if matches!(kind, ChartKind::Stock) && s.x_values_ref.is_some() {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                "stock chart series take values_ref only (no x_values_ref)",
            )
            .with_sheet(sheet));
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
        let no_decorations = matches!(
            kind,
            ChartKind::Pie | ChartKind::Doughnut | ChartKind::Radar | ChartKind::Stock
        );
        if let Some(t) = s.trendline.as_ref() {
            let msg = if no_decorations {
                Some(format!("trendlines are not supported on {kind:?} charts"))
            } else if t.polynomial_order.is_some_and(|o| !(2..=6).contains(&o)) {
                Some("trendline polynomial_order must be 2..=6".to_string())
            } else if t.period.is_some_and(|p| p < 2) {
                Some("trendline period must be >= 2".to_string())
            } else {
                None
            };
            if let Some(msg) = msg {
                return Err(ApiError::new(ApiErrorCode::InvalidChart, msg).with_sheet(sheet));
            }
        }
        if let Some(e) = s.error_bars.as_ref() {
            let custom = matches!(e.value_type, ChartErrorValueType::Custom);
            let has_custom_data = e.plus_ref.is_some()
                || e.minus_ref.is_some()
                || e.plus_values.is_some()
                || e.minus_values.is_some();
            let msg = if no_decorations {
                Some(format!("error bars are not supported on {kind:?} charts"))
            } else if custom && !has_custom_data {
                Some(
                    "custom error bars require plus_ref/minus_ref or plus_values/minus_values"
                        .to_string(),
                )
            } else if !custom && e.value.is_none() {
                Some(format!("{:?} error bars require a value", e.value_type))
            } else {
                None
            };
            if let Some(msg) = msg {
                return Err(ApiError::new(ApiErrorCode::InvalidChart, msg).with_sheet(sheet));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_data_table(
    sheet: &str,
    kind: ChartKind,
    data_table: Option<&ChartDataTable>,
) -> Result<()> {
    if data_table.is_some() && !is_cartesian(kind) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!("data tables are not supported on {kind:?} charts (cartesian only)"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

pub(super) fn validate_view_3d(
    sheet: &str,
    kind: ChartKind,
    view: Option<&ChartView3D>,
) -> Result<()> {
    let Some(v) = view else { return Ok(()) };
    if !is_3d(kind) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!("view3D is only supported on 3D charts, not {kind:?}"),
        )
        .with_sheet(sheet));
    }
    let check = |ok: bool, msg: &str| {
        if ok {
            Ok(())
        } else {
            Err(ApiError::new(ApiErrorCode::InvalidChart, msg.to_string()).with_sheet(sheet))
        }
    };
    if let Some(x) = v.rot_x {
        check((-90..=90).contains(&x), "view3D rot_x must be -90..=90")?;
    }
    if let Some(y) = v.rot_y {
        check(y <= 360, "view3D rot_y must be 0..=360")?;
    }
    if let Some(p) = v.perspective {
        check(p <= 240, "view3D perspective must be 0..=240")?;
    }
    if let Some(d) = v.depth_percent {
        check(
            (20..=2000).contains(&d),
            "view3D depth_percent must be 20..=2000",
        )?;
    }
    if let Some(h) = v.height_percent {
        check(
            (5..=500).contains(&h),
            "view3D height_percent must be 5..=500",
        )?;
    }
    Ok(())
}

pub(super) fn validate_bar_shape(
    sheet: &str,
    kind: ChartKind,
    shape: Option<&Bar3DShape>,
) -> Result<()> {
    if shape.is_some() && !is_bar_3d(kind) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!("bar_shape is only supported on bar3D/column3D charts, not {kind:?}"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

pub(super) fn validate_wireframe(sheet: &str, kind: ChartKind, wireframe: Option<bool>) -> Result<()> {
    if wireframe.is_some() && !is_surface(kind) {
        return Err(ApiError::new(
            ApiErrorCode::InvalidChart,
            format!("wireframe is only supported on surface/surface3D charts, not {kind:?}"),
        )
        .with_sheet(sheet));
    }
    Ok(())
}

pub(super) fn is_cartesian(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::Column | ChartKind::Bar | ChartKind::Line | ChartKind::Area
    )
}

pub(super) fn gap_width_for_kind(kind: ChartKind, value: Option<u16>) -> Option<u16> {
    match kind {
        ChartKind::Column | ChartKind::Bar | ChartKind::Column3D | ChartKind::Bar3D => value,
        _ => None,
    }
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

pub(super) fn validate_pie_options(
    sheet: &str,
    hole_size: Option<u8>,
    first_slice_angle: Option<u16>,
) -> Result<()> {
    if let Some(h) = hole_size {
        if !(10..=90).contains(&h) {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart hole_size must be 10..=90, got: {h}"),
            )
            .with_sheet(sheet));
        }
    }
    if let Some(a) = first_slice_angle {
        if a > 360 {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart first_slice_angle must be 0..=360, got: {a}"),
            )
            .with_sheet(sheet));
        }
    }
    Ok(())
}

pub(super) fn validate_axis_options(sheet: &str, axis: Option<&ChartAxisPatch>) -> Result<()> {
    if let Some(rot) = axis.and_then(|a| a.label_rotation) {
        if !(-90..=90).contains(&rot) {
            return Err(ApiError::new(
                ApiErrorCode::InvalidChart,
                format!("chart axis label_rotation must be -90..=90, got: {rot}"),
            )
            .with_sheet(sheet));
        }
    }
    Ok(())
}

pub(super) fn hole_size_for_kind(kind: ChartKind, value: Option<u8>) -> Option<u8> {
    match kind {
        ChartKind::Doughnut => value,
        _ => None,
    }
}

pub(super) fn first_slice_angle_for_kind(kind: ChartKind, value: Option<u16>) -> Option<u16> {
    match kind {
        ChartKind::Pie | ChartKind::Doughnut => value,
        _ => None,
    }
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
    if supports_stacking(kind) {
        requested
    } else {
        None
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

pub(super) fn disp_blanks_as_to(v: DispBlanksAs) -> c::DisplayBlanksAsValues {
    match v {
        DispBlanksAs::Span => c::DisplayBlanksAsValues::Span,
        DispBlanksAs::Gap => c::DisplayBlanksAsValues::Gap,
        DispBlanksAs::Zero => c::DisplayBlanksAsValues::Zero,
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
        ChartKind::Pie | ChartKind::Doughnut | ChartKind::Pie3D => {}
        k if is_3d_cartesian(k) || is_surface(k) => {
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
            plot_area
                .plot_area_choice2
                .push(c::PlotAreaChoice2::SeriesAxis(Box::new(build_ser_axis())));
        }
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
            plot_area.data_table = build_data_table(patch.data_table.as_ref());
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
        view3_d: patch
            .view_3d
            .as_ref()
            .filter(|_| is_3d(patch.kind))
            .map(|v| Box::new(build_view_3d(v))),
        plot_area: Box::new(plot_area),
        legend,
        plot_visible_only: Some(c::PlotVisibleOnly {
            val: Some(BooleanValue::from_bool(true)),
        }),
        display_blanks_as: Some(c::DisplayBlanksAs {
            val: Some(disp_blanks_as_to(
                patch.disp_blanks_as.unwrap_or(DispBlanksAs::Gap),
            )),
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
        ChartKind::Stock => vec![build_stock_chart(patch)],
        ChartKind::Pie3D => vec![build_pie_3d_chart(patch)],
        ChartKind::Bar3D | ChartKind::Column3D | ChartKind::Line3D | ChartKind::Area3D => {
            vec![build_3d_cartesian_chart(patch)]
        }
        ChartKind::Surface | ChartKind::Surface3D => vec![build_surface_chart(patch)],
        _ => build_cartesian_plot_charts(patch),
    }
}

pub(super) fn is_3d(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::Bar3D
            | ChartKind::Column3D
            | ChartKind::Line3D
            | ChartKind::Pie3D
            | ChartKind::Area3D
            | ChartKind::Surface
            | ChartKind::Surface3D
    )
}

pub(super) fn is_surface(kind: ChartKind) -> bool {
    matches!(kind, ChartKind::Surface | ChartKind::Surface3D)
}

pub(super) fn is_3d_cartesian(kind: ChartKind) -> bool {
    matches!(
        kind,
        ChartKind::Bar3D | ChartKind::Column3D | ChartKind::Line3D | ChartKind::Area3D
    )
}

pub(super) fn is_bar_3d(kind: ChartKind) -> bool {
    matches!(kind, ChartKind::Bar3D | ChartKind::Column3D)
}

pub(super) fn supports_stacking(kind: ChartKind) -> bool {
    is_cartesian(kind) || is_3d_cartesian(kind)
}

pub(super) fn shape_to(s: Bar3DShape) -> c::ShapeValues {
    match s {
        Bar3DShape::Cone => c::ShapeValues::Cone,
        Bar3DShape::ConeToMax => c::ShapeValues::ConeToMax,
        Bar3DShape::Box => c::ShapeValues::Box,
        Bar3DShape::Cylinder => c::ShapeValues::Cylinder,
        Bar3DShape::Pyramid => c::ShapeValues::Pyramid,
        Bar3DShape::PyramidToMaximum => c::ShapeValues::PyramidToMaximum,
    }
}

pub(super) fn build_view_3d(v: &ChartView3D) -> c::View3D {
    c::View3D {
        rotate_x: v.rot_x.map(|val| c::RotateX { val: Some(val) }),
        height_percent: v
            .height_percent
            .map(|val| c::HeightPercent { val: Some(val) }),
        rotate_y: v.rot_y.map(|val| c::RotateY { val: Some(val) }),
        depth_percent: v
            .depth_percent
            .map(|val| c::DepthPercent { val: Some(val) }),
        right_angle_axes: v.right_angle_axes.map(|val| c::RightAngleAxes {
            val: Some(BooleanValue::from_bool(val)),
        }),
        perspective: v.perspective.map(|val| c::Perspective { val: Some(val) }),
        ..Default::default()
    }
}

pub(super) fn build_ser_axis() -> c::SeriesAxis {
    c::SeriesAxis {
        axis_id: Box::new(axis_id(SER_AX_ID)),
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
        crossing_axis: Box::new(c::CrossingAxis { val: VAL_AX_ID }),
        ..Default::default()
    }
}

pub(super) fn build_3d_axis_ids() -> Vec<c::AxisId> {
    vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID), axis_id(SER_AX_ID)]
}

pub(super) fn build_3d_cartesian_chart(patch: &ChartPatch) -> c::PlotAreaChoice {
    let cat_ref = patch.categories_ref.as_deref();
    let dl = build_data_labels(patch.data_labels.as_ref());
    let series = || patch.series.iter().enumerate();
    match patch.kind {
        ChartKind::Line3D => c::PlotAreaChoice::Line3DChart(Box::new(c::Line3DChart {
            grouping: Box::new(c::Grouping {
                val: Some(line_area_grouping(patch.stacking)),
            }),
            vary_colors: vary_colors_el(patch.vary_colors, false),
            line_chart_series: series()
                .map(|(i, s)| build_line_series(i, s, cat_ref))
                .collect(),
            data_labels: dl,
            axis_id: build_3d_axis_ids(),
            ..Default::default()
        })),
        ChartKind::Area3D => c::PlotAreaChoice::Area3DChart(Box::new(c::Area3DChart {
            grouping: Some(c::Grouping {
                val: Some(line_area_grouping(patch.stacking)),
            }),
            vary_colors: vary_colors_el(patch.vary_colors, false),
            area_chart_series: series()
                .map(|(i, s)| build_area_series(i, s, cat_ref))
                .collect(),
            data_labels: dl,
            axis_id: build_3d_axis_ids(),
            ..Default::default()
        })),
        _ => c::PlotAreaChoice::Bar3DChart(Box::new(c::Bar3DChart {
            bar_direction: Box::new(c::BarDirection {
                val: if matches!(patch.kind, ChartKind::Bar3D) {
                    c::BarDirectionValues::Bar
                } else {
                    c::BarDirectionValues::Column
                },
            }),
            bar_grouping: Some(c::BarGrouping {
                val: Some(bar_grouping(patch.stacking)),
            }),
            vary_colors: vary_colors_el(patch.vary_colors, false),
            bar_chart_series: series()
                .map(|(i, s)| build_bar_series(i, s, cat_ref))
                .collect(),
            data_labels: dl,
            gap_width: patch.gap_width.map(|g| c::GapWidth { val: Some(g) }),
            shape: patch.bar_shape.map(|s| c::Shape {
                val: Some(shape_to(s)),
                ..Default::default()
            }),
            axis_id: build_3d_axis_ids(),
            ..Default::default()
        })),
    }
}

pub(super) fn build_pie_3d_chart(patch: &ChartPatch) -> c::PlotAreaChoice {
    c::PlotAreaChoice::Pie3DChart(Box::new(c::Pie3DChart {
        vary_colors: vary_colors_el(patch.vary_colors, true),
        pie_chart_series: patch
            .series
            .iter()
            .enumerate()
            .map(|(i, s)| build_pie_series(i, s, patch.categories_ref.as_deref()))
            .collect(),
        data_labels: build_data_labels(patch.data_labels.as_ref()),
        ..Default::default()
    }))
}

pub(super) fn build_wireframe_el(wireframe: Option<bool>) -> Option<c::Wireframe> {
    wireframe.map(|v| c::Wireframe {
        val: Some(BooleanValue::from_bool(v)),
    })
}

pub(super) fn build_surface_series(idx: usize, s: &ChartSeriesPatch, cat_ref: Option<&str>) -> c::SurfaceChartSeries {
    c::SurfaceChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        ..Default::default()
    }
}

pub(super) fn build_surface_chart(patch: &ChartPatch) -> c::PlotAreaChoice {
    let cat_ref = patch.categories_ref.as_deref();
    let series: Vec<c::SurfaceChartSeries> = patch
        .series
        .iter()
        .enumerate()
        .map(|(i, s)| build_surface_series(i, s, cat_ref))
        .collect();
    let wireframe = build_wireframe_el(patch.wireframe);
    if matches!(patch.kind, ChartKind::Surface3D) {
        c::PlotAreaChoice::Surface3DChart(Box::new(c::Surface3DChart {
            wireframe,
            surface_chart_series: series,
            axis_id: build_3d_axis_ids(),
            ..Default::default()
        }))
    } else {
        c::PlotAreaChoice::SurfaceChart(Box::new(c::SurfaceChart {
            wireframe,
            surface_chart_series: series,
            axis_id: build_3d_axis_ids(),
            ..Default::default()
        }))
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
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
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
            vary_colors: vary_colors_el(patch.vary_colors, false),
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
            vary_colors: vary_colors_el(patch.vary_colors, false),
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
            vary_colors: vary_colors_el(patch.vary_colors, false),
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
            vary_colors: vary_colors_el(patch.vary_colors, true),
            pie_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_pie_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            first_slice_angle: patch
                .first_slice_angle
                .map(|v| c::FirstSliceAngle { val: Some(v) }),
            ..Default::default()
        })),
        ChartKind::Doughnut => c::PlotAreaChoice::DoughnutChart(Box::new(c::DoughnutChart {
            vary_colors: vary_colors_el(patch.vary_colors, true),
            pie_chart_series: patch
                .series
                .iter()
                .enumerate()
                .map(|(i, s)| build_pie_series(i, s, patch.categories_ref.as_deref()))
                .collect(),
            data_labels: dl,
            first_slice_angle: patch
                .first_slice_angle
                .map(|v| c::FirstSliceAngle { val: Some(v) }),
            hole_size: Box::new(c::HoleSize {
                val: patch.hole_size.unwrap_or(50),
            }),
            ..Default::default()
        })),
        ChartKind::Scatter => c::PlotAreaChoice::ScatterChart(Box::new(c::ScatterChart {
            scatter_style: Box::new(c::ScatterStyle {
                val: Some(if patch.series.iter().any(|s| s.smooth == Some(true)) {
                    c::ScatterStyleValues::SmoothMarker
                } else {
                    c::ScatterStyleValues::LineMarker
                }),
            }),
            vary_colors: vary_colors_el(patch.vary_colors, false),
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
            vary_colors: vary_colors_el(patch.vary_colors, true),
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
            vary_colors: vary_colors_el(patch.vary_colors, false),
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
        ChartKind::Stock => unreachable!("stock is built via build_stock_chart"),
        ChartKind::Pie3D => unreachable!("pie3D is built via build_pie_3d_chart"),
        ChartKind::Bar3D | ChartKind::Column3D | ChartKind::Line3D | ChartKind::Area3D => {
            unreachable!("3D cartesian kinds are built via build_3d_cartesian_chart")
        }
        ChartKind::Surface | ChartKind::Surface3D => {
            unreachable!("surface kinds are built via build_surface_chart")
        }
    }
}

pub(super) fn stock_hi_low_lines(patch: &ChartPatch) -> bool {
    patch.hi_low_lines.unwrap_or(true)
}

pub(super) fn stock_up_down_bars(patch: &ChartPatch) -> bool {
    patch.up_down_bars.unwrap_or(patch.series.len() >= 4)
}

pub(super) fn stock_drop_lines(patch: &ChartPatch) -> bool {
    patch.drop_lines.unwrap_or(false)
}

pub(super) fn build_stock_chart(patch: &ChartPatch) -> c::PlotAreaChoice {
    let cat_ref = patch.categories_ref.as_deref();
    c::PlotAreaChoice::StockChart(Box::new(c::StockChart {
        line_chart_series: patch
            .series
            .iter()
            .enumerate()
            .map(|(i, s)| build_stock_series(i, s, cat_ref))
            .collect(),
        data_labels: build_data_labels(patch.data_labels.as_ref()),
        drop_lines: stock_drop_lines(patch).then(|| Box::new(c::DropLines::default())),
        high_low_lines: stock_hi_low_lines(patch).then(|| Box::new(c::HighLowLines::default())),
        up_down_bars: stock_up_down_bars(patch).then(|| {
            Box::new(c::UpDownBars {
                gap_width: Some(c::GapWidth { val: Some(150) }),
                up_bars: Some(Box::new(c::UpBars::default())),
                down_bars: Some(Box::new(c::DownBars::default())),
                ..Default::default()
            })
        }),
        axis_id: vec![axis_id(CAT_AX_ID), axis_id(VAL_AX_ID)],
        ..Default::default()
    }))
}

pub(super) fn build_stock_series(
    idx: usize,
    s: &ChartSeriesPatch,
    cat_ref: Option<&str>,
) -> c::LineChartSeries {
    let mut ser = build_line_series(idx, s, cat_ref);
    ser.chart_shape_properties = Some(Box::new(c::ChartShapeProperties {
        outline: Some(Box::new(a::Outline {
            outline_choice1: Some(a::OutlineChoice::NoFill(Box::new(a::NoFill::default()))),
            ..Default::default()
        })),
        ..Default::default()
    }));
    ser
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
        data_label: dl.per_point.iter().map(build_point_data_label).collect(),
        data_labels_choice: Some(c::DataLabelsChoice::Sequence(Box::new(seq))),
        ..Default::default()
    }))
}

pub(super) fn build_point_data_label(p: &ChartDataLabel) -> c::DataLabel {
    let index = Box::new(c::Index { val: p.index });
    if p.delete {
        return c::DataLabel {
            index,
            data_label_choice: Some(c::DataLabelChoice::Delete(Box::new(c::Delete {
                val: Some(BooleanValue::from_bool(true)),
            }))),
            ..Default::default()
        };
    }
    fn b(v: Option<bool>) -> Option<BooleanValue> {
        v.map(BooleanValue::from_bool)
    }
    let seq = c::DataLabelChoiceSequence {
        numbering_format: p.number_format.as_ref().map(|nf| c::NumberingFormat {
            format_code: nf.clone(),
            source_linked: Some(BooleanValue::from_bool(false)),
        }),
        data_label_position: p.position.map(|pos| c::DataLabelPosition {
            val: data_label_pos_to(pos),
        }),
        show_legend_key: p.show_legend_key.map(|_| c::ShowLegendKey {
            val: b(p.show_legend_key),
        }),
        show_value: p.show_value.map(|_| c::ShowValue {
            val: b(p.show_value),
        }),
        show_category_name: p.show_category_name.map(|_| c::ShowCategoryName {
            val: b(p.show_category_name),
        }),
        show_series_name: p.show_series_name.map(|_| c::ShowSeriesName {
            val: b(p.show_series_name),
        }),
        show_percent: p.show_percent.map(|_| c::ShowPercent {
            val: b(p.show_percent),
        }),
        separator: p.separator.clone(),
        ..Default::default()
    };
    c::DataLabel {
        index,
        data_label_choice: Some(c::DataLabelChoice::Sequence(Box::new(seq))),
        ..Default::default()
    }
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

pub(super) fn vary_colors_el(opt: Option<bool>, default: bool) -> Option<c::VaryColors> {
    Some(c::VaryColors {
        val: Some(BooleanValue::from_bool(opt.unwrap_or(default))),
    })
}

pub(super) fn vary_colors_effective(kind: ChartKind, opt: Option<bool>) -> bool {
    opt.unwrap_or(matches!(
        kind,
        ChartKind::Pie | ChartKind::Doughnut | ChartKind::Bubble
    ))
}

pub(super) fn build_invert_if_negative(opt: Option<bool>) -> Option<c::InvertIfNegative> {
    opt.map(|v| c::InvertIfNegative {
        val: Some(BooleanValue::from_bool(v)),
    })
}

pub(super) fn build_trendline(t: Option<&ChartTrendline>) -> Vec<c::Trendline> {
    let Some(t) = t else {
        return Vec::new();
    };
    let bv = |v: bool| Some(BooleanValue::from_bool(v));
    let kind = match t.kind {
        TrendlineKind::Exponential => c::TrendlineValues::Exponential,
        TrendlineKind::Linear => c::TrendlineValues::Linear,
        TrendlineKind::Logarithmic => c::TrendlineValues::Logarithmic,
        TrendlineKind::MovingAverage => c::TrendlineValues::MovingAverage,
        TrendlineKind::Polynomial => c::TrendlineValues::Polynomial,
        TrendlineKind::Power => c::TrendlineValues::Power,
    };
    vec![c::Trendline {
        trendline_name: t.name.clone(),
        trendline_type: Box::new(c::TrendlineType { val: Some(kind) }),
        polynomial_order: t.polynomial_order.map(|val| c::PolynomialOrder { val }),
        period: t.period.map(|val| c::Period { val }),
        forward: t.forward.map(|val| c::Forward { val }),
        backward: t.backward.map(|val| c::Backward { val }),
        intercept: t.intercept.map(|val| c::Intercept { val }),
        display_r_squared_value: t
            .display_r_squared
            .map(|v| c::DisplayRSquaredValue { val: bv(v) }),
        display_equation: t
            .display_equation
            .map(|v| c::DisplayEquation { val: bv(v) }),
        ..Default::default()
    }]
}

pub(super) fn build_data_table(dt: Option<&ChartDataTable>) -> Option<Box<c::DataTable>> {
    let dt = dt?;
    let b = |v: Option<bool>| v.map(BooleanValue::from_bool);
    Some(Box::new(c::DataTable {
        show_horizontal_border: dt.show_horizontal_border.map(|_| c::ShowHorizontalBorder {
            val: b(dt.show_horizontal_border),
        }),
        show_vertical_border: dt.show_vertical_border.map(|_| c::ShowVerticalBorder {
            val: b(dt.show_vertical_border),
        }),
        show_outline_border: dt.show_outline.map(|_| c::ShowOutlineBorder {
            val: b(dt.show_outline),
        }),
        show_keys: dt.show_keys.map(|_| c::ShowKeys {
            val: b(dt.show_keys),
        }),
        ..Default::default()
    }))
}

pub(super) fn build_num_literal(vals: &[f64]) -> Box<c::NumberLiteral> {
    Box::new(c::NumberLiteral {
        point_count: Some(c::PointCount {
            val: vals.len() as u32,
        }),
        numeric_point: vals
            .iter()
            .enumerate()
            .map(|(i, v)| c::NumericPoint {
                index: i as u32,
                numeric_value: v.to_string(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    })
}

pub(super) fn build_error_bars(e: Option<&ChartErrorBars>) -> Option<c::ErrorBars> {
    let e = e?;
    let bar_type = match e.bar_type {
        ChartErrorBarType::Both => c::ErrorBarValues::Both,
        ChartErrorBarType::Minus => c::ErrorBarValues::Minus,
        ChartErrorBarType::Plus => c::ErrorBarValues::Plus,
    };
    let value_type = match e.value_type {
        ChartErrorValueType::Custom => c::ErrorValues::Custom,
        ChartErrorValueType::FixedValue => c::ErrorValues::FixedValue,
        ChartErrorValueType::Percentage => c::ErrorValues::Percentage,
        ChartErrorValueType::StandardDeviation => c::ErrorValues::StandardDeviation,
        ChartErrorValueType::StandardError => c::ErrorValues::StandardError,
    };
    let plus = e
        .plus_ref
        .as_deref()
        .map(|r| {
            c::PlusChoice::NumberReference(Box::new(c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            }))
        })
        .or_else(|| {
            e.plus_values
                .as_deref()
                .map(|v| c::PlusChoice::NumberLiteral(build_num_literal(v)))
        });
    let minus = e
        .minus_ref
        .as_deref()
        .map(|r| {
            c::MinusChoice::NumberReference(Box::new(c::NumberReference {
                formula: r.to_string(),
                ..Default::default()
            }))
        })
        .or_else(|| {
            e.minus_values
                .as_deref()
                .map(|v| c::MinusChoice::NumberLiteral(build_num_literal(v)))
        });
    Some(c::ErrorBars {
        error_direction: e.direction.map(|d| c::ErrorDirection {
            val: match d {
                ChartErrorDirection::X => c::ErrorBarDirectionValues::X,
                ChartErrorDirection::Y => c::ErrorBarDirectionValues::Y,
            },
        }),
        error_bar_type: Box::new(c::ErrorBarType { val: bar_type }),
        error_bar_value_type: Box::new(c::ErrorBarValueType { val: value_type }),
        no_end_cap: e.no_end_cap.map(|v| c::NoEndCap {
            val: Some(BooleanValue::from_bool(v)),
        }),
        plus: plus.map(|ch| {
            Box::new(c::Plus {
                plus_choice: Some(ch),
            })
        }),
        minus: minus.map(|ch| {
            Box::new(c::Minus {
                minus_choice: Some(ch),
            })
        }),
        error_bar_value: e.value.map(|val| c::ErrorBarValue { val }),
        ..Default::default()
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
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
        invert_if_negative: build_invert_if_negative(s.invert_if_negative),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        trendline: build_trendline(s.trendline.as_ref()),
        error_bars: build_error_bars(s.error_bars.as_ref()).map(Box::new),
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
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
        marker: build_marker(s.marker.as_ref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        trendline: build_trendline(s.trendline.as_ref()),
        error_bars: build_error_bars(s.error_bars.as_ref()).map(Box::new),
        category_axis_data: build_categories(cat_ref),
        values: Some(build_values(&s.values_ref)),
        smooth: s.smooth.map(|v| c::Smooth {
            val: Some(BooleanValue::from_bool(v)),
        }),
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
        trendline: build_trendline(s.trendline.as_ref()),
        error_bars: build_error_bars(s.error_bars.as_ref())
            .into_iter()
            .collect(),
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
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
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
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
        marker: build_marker(s.marker.as_ref()),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        trendline: build_trendline(s.trendline.as_ref()),
        error_bars: build_error_bars(s.error_bars.as_ref())
            .into_iter()
            .collect(),
        x_values: s.x_values_ref.as_deref().map(build_x_values),
        y_values: Some(build_y_values(&s.values_ref)),
        smooth: Some(c::Smooth {
            val: Some(BooleanValue::from_bool(s.smooth.unwrap_or(false))),
        }),
        ..Default::default()
    }
}

pub(super) fn build_bubble_series(idx: usize, s: &ChartSeriesPatch) -> c::BubbleChartSeries {
    c::BubbleChartSeries {
        index: Box::new(c::Index { val: idx as u32 }),
        order: Box::new(c::Order { val: idx as u32 }),
        series_text: build_series_text(s),
        chart_shape_properties: build_series_shape_with_line(s.color.as_deref(), s.line.as_ref()),
        invert_if_negative: build_invert_if_negative(s.invert_if_negative),
        data_point: build_data_points(&s.data_points),
        data_labels: build_data_labels(s.data_labels.as_ref()),
        trendline: build_trendline(s.trendline.as_ref()),
        error_bars: build_error_bars(s.error_bars.as_ref())
            .into_iter()
            .collect(),
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
            explosion: p.explosion.map(|val| c::Explosion { val: val.min(400) }),
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
    build_series_shape_with_line(color, None)
}

pub(super) fn line_dash_to(d: LineDash) -> a::PresetLineDashValues {
    match d {
        LineDash::Solid => a::PresetLineDashValues::Solid,
        LineDash::Dot => a::PresetLineDashValues::Dot,
        LineDash::Dash => a::PresetLineDashValues::Dash,
        LineDash::LargeDash => a::PresetLineDashValues::LargeDash,
        LineDash::DashDot => a::PresetLineDashValues::DashDot,
        LineDash::LargeDashDot => a::PresetLineDashValues::LargeDashDot,
        LineDash::LargeDashDotDot => a::PresetLineDashValues::LargeDashDotDot,
        LineDash::SystemDash => a::PresetLineDashValues::SystemDash,
        LineDash::SystemDot => a::PresetLineDashValues::SystemDot,
        LineDash::SystemDashDot => a::PresetLineDashValues::SystemDashDot,
        LineDash::SystemDashDotDot => a::PresetLineDashValues::SystemDashDotDot,
    }
}

pub(super) fn build_outline(line: &ChartLine, color: Option<&str>) -> Box<a::Outline> {
    let choice1 = if line.none == Some(true) {
        Some(a::OutlineChoice::NoFill(Box::new(a::NoFill::default())))
    } else {
        color.and_then(normalize_chart_hex).map(|val| {
            a::OutlineChoice::SolidFill(Box::new(a::SolidFill {
                solid_fill_choice: Some(a::SolidFillChoice::RgbColorModelHex(Box::new(
                    a::RgbColorModelHex {
                        val,
                        ..Default::default()
                    },
                ))),
                ..Default::default()
            }))
        })
    };
    let choice2 = (line.none != Some(true))
        .then(|| line.dash)
        .flatten()
        .map(|d| {
            a::OutlineChoice2::PresetDash(Box::new(a::PresetDash {
                val: Some(line_dash_to(d)),
                ..Default::default()
            }))
        });
    Box::new(a::Outline {
        width: line.width_emu,
        outline_choice1: choice1,
        outline_choice2: choice2,
        ..Default::default()
    })
}

pub(super) fn build_series_shape_with_line(
    color: Option<&str>,
    line: Option<&ChartLine>,
) -> Option<Box<c::ChartShapeProperties>> {
    let fill = color.and_then(normalize_chart_hex).map(|val| {
        c::ChartShapePropertiesChoice2::SolidFill(Box::new(a::SolidFill {
            solid_fill_choice: Some(a::SolidFillChoice::RgbColorModelHex(Box::new(
                a::RgbColorModelHex {
                    val,
                    ..Default::default()
                },
            ))),
            ..Default::default()
        }))
    });
    let outline = line.map(|l| build_outline(l, color));
    if fill.is_none() && outline.is_none() {
        return None;
    }
    Some(Box::new(c::ChartShapeProperties {
        chart_shape_properties_choice2: fill,
        outline,
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

pub(super) fn built_in_unit_to(u: BuiltInUnit) -> c::BuiltInUnitValues {
    match u {
        BuiltInUnit::Hundreds => c::BuiltInUnitValues::Hundreds,
        BuiltInUnit::Thousands => c::BuiltInUnitValues::Thousands,
        BuiltInUnit::TenThousands => c::BuiltInUnitValues::TenThousands,
        BuiltInUnit::HundredThousands => c::BuiltInUnitValues::HundredThousands,
        BuiltInUnit::Millions => c::BuiltInUnitValues::Millions,
        BuiltInUnit::TenMillions => c::BuiltInUnitValues::TenMillions,
        BuiltInUnit::HundredMillions => c::BuiltInUnitValues::HundredMillions,
        BuiltInUnit::Billions => c::BuiltInUnitValues::Billions,
        BuiltInUnit::Trillions => c::BuiltInUnitValues::Trillions,
    }
}

pub(super) fn build_axis_txpr(rotation_degrees: i32) -> c::TextProperties {
    c::TextProperties {
        body_properties: Box::new(a::BodyProperties {
            rotation: Some(rotation_degrees * 60000),
            ..Default::default()
        }),
        list_style: Some(Box::new(a::ListStyle::default())),
        paragraph: vec![a::Paragraph {
            paragraph_properties: Some(Box::new(a::ParagraphProperties::default())),
            ..Default::default()
        }],
        ..Default::default()
    }
}

pub(super) fn build_display_units(du: &DisplayUnits) -> c::DisplayUnits {
    let choice = match du {
        DisplayUnits::Builtin(u) => c::DisplayUnitsChoice::BuiltInUnit(Box::new(c::BuiltInUnit {
            val: Some(built_in_unit_to(*u)),
        })),
        DisplayUnits::Custom(v) => {
            c::DisplayUnitsChoice::CustomDisplayUnit(Box::new(c::CustomDisplayUnit { val: *v }))
        }
    };
    c::DisplayUnits {
        display_units_choice: Some(choice),
        display_units_label: Some(Box::new(c::DisplayUnitsLabel::default())),
        ..Default::default()
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
        if let Some(rot) = $p.label_rotation {
            $ax.text_properties = Some(Box::new(build_axis_txpr(rot)));
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
    if let Some(du) = &p.display_units {
        ax.display_units = Some(Box::new(build_display_units(du)));
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
