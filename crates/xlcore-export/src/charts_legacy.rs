use crate::chart_colors::*;
use crate::charts_helpers::*;
use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;

pub(super) fn extract_chart(space: &c::ChartSpace, theme: Option<&Theme>) -> Option<Chart> {
    let chart = &space.chart;
    let plot_area = &chart.plot_area;

    let mut secondary_ax_ids: Vec<i32> = Vec::new();
    let mut primary_ax_ids: Vec<i32> = Vec::new();
    let mut primary_val_fmt: Option<String> = None;
    let mut secondary_val_fmt: Option<String> = None;
    let mut value_min: Option<f64> = None;
    let mut value_max: Option<f64> = None;
    let mut value_min_secondary: Option<f64> = None;
    let mut value_max_secondary: Option<f64> = None;

    let mut x_axis_title: Option<String> = None;
    let mut y_axis_title: Option<String> = None;
    let mut y_axis_title_secondary: Option<String> = None;
    let mut x_axis_title_font: Option<ChartFont> = None;
    let mut y_axis_title_font: Option<ChartFont> = None;
    let mut x_axis_title_fill: Option<String> = None;
    let mut y_axis_title_fill: Option<String> = None;
    let mut x_axis_title_border: Option<ChartStyleBorder> = None;
    let mut y_axis_title_border: Option<ChartStyleBorder> = None;

    let mut show_major_gridlines: Option<bool> = None;
    let mut show_major_gridlines_secondary: Option<bool> = None;

    let mut disp_units: Option<f64> = None;
    let mut disp_units_label: Option<String> = None;
    let mut disp_units_secondary: Option<f64> = None;
    let mut disp_units_label_secondary: Option<String> = None;

    let mut major_unit: Option<f64> = None;
    let mut major_unit_secondary: Option<f64> = None;

    let mut cat_axis_label_rotation: Option<i32> = None;
    let mut val_axis_label_rotation: Option<i32> = None;

    let data_table = plot_area.data_table.as_deref().map(|dt| ChartDataTable {
        show_horz_border: dt
            .show_horizontal_border
            .as_ref()
            .map(|s| s.val.map(bool::from).unwrap_or(true))
            .unwrap_or(false),
        show_vert_border: dt
            .show_vertical_border
            .as_ref()
            .map(|s| s.val.map(bool::from).unwrap_or(true))
            .unwrap_or(false),
        show_outline: dt
            .show_outline_border
            .as_ref()
            .map(|s| s.val.map(bool::from).unwrap_or(true))
            .unwrap_or(false),
        show_keys: dt
            .show_keys
            .as_ref()
            .map(|s| s.val.map(bool::from).unwrap_or(true))
            .unwrap_or(false),
    });

    let axis_label_rotation = |tp: Option<&c::TextProperties>| -> Option<i32> {
        tp.and_then(|t| t.body_properties.rotation)
            .map(|r| r / 60000)
    };

    let route_title = |pos: Option<&c::AxisPositionValues>,
                       title: Option<&c::Title>,
                       x: &mut Option<String>,
                       y: &mut Option<String>,
                       y2: &mut Option<String>,
                       xf: &mut Option<ChartFont>,
                       yf: &mut Option<ChartFont>,
                       xfill: &mut Option<String>,
                       yfill: &mut Option<String>,
                       xborder: &mut Option<ChartStyleBorder>,
                       yborder: &mut Option<ChartStyleBorder>| {
        let Some(t) = extract_title(title) else {
            return;
        };
        let font = extract_title_font(title);
        let title_sp = title.and_then(|t| t.chart_shape_properties.as_deref());
        let fill = title_sp.and_then(|sp| fill_color_outside_outline(&format!("{:?}", sp), theme));
        let border = title_sp
            .and_then(|sp| extract_style_border(sp.outline.as_deref(), &format!("{:?}", sp), theme));
        match pos {
            Some(c::AxisPositionValues::Bottom) | Some(c::AxisPositionValues::Top) => {
                if x.is_none() {
                    *x = Some(t);
                    *xf = font;
                    *xfill = fill;
                    *xborder = border;
                }
            }
            Some(c::AxisPositionValues::Right) => {
                if y2.is_none() {
                    *y2 = Some(t);
                }
            }

            _ => {
                if y.is_none() {
                    *y = Some(t);
                    *yf = font;
                    *yfill = fill;
                    *yborder = border;
                }
            }
        }
    };
    for choice in &plot_area.plot_area_choice2 {
        match choice {
            c::PlotAreaChoice2::CategoryAxis(ca) => {
                if cat_axis_label_rotation.is_none() {
                    cat_axis_label_rotation = axis_label_rotation(ca.text_properties.as_deref());
                }
                route_title(
                    Some(&ca.axis_position.val),
                    ca.title.as_deref(),
                    &mut x_axis_title,
                    &mut y_axis_title,
                    &mut y_axis_title_secondary,
                    &mut x_axis_title_font,
                    &mut y_axis_title_font,
                    &mut x_axis_title_fill,
                    &mut y_axis_title_fill,
                    &mut x_axis_title_border,
                    &mut y_axis_title_border,
                )
            }
            c::PlotAreaChoice2::DateAxis(da) => route_title(
                Some(&da.axis_position.val),
                da.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
                &mut x_axis_title_font,
                &mut y_axis_title_font,
                    &mut x_axis_title_fill,
                    &mut y_axis_title_fill,
                    &mut x_axis_title_border,
                    &mut y_axis_title_border,
            ),
            _ => {}
        }
        if let c::PlotAreaChoice2::ValueAxis(va) = choice {
            let axid = va.axis_id.val;
            let pos = Some(&va.axis_position.val);
            let is_secondary = matches!(
                pos,
                Some(c::AxisPositionValues::Right) | Some(c::AxisPositionValues::Top)
            );

            let is_horizontal_value_axis =
                matches!(pos, Some(c::AxisPositionValues::Bottom)) && !is_secondary;

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

            let scaling_min = va.scaling.min_axis_value.as_ref().map(|m| m.val);
            let scaling_max = va.scaling.max_axis_value.as_ref().map(|m| m.val);

            let axis_major_unit = va
                .major_unit
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
                    if let Some((f, lbl)) = extract_disp_units(va.display_units.as_deref()) {
                        disp_units_secondary = Some(f);
                        disp_units_label_secondary = lbl;
                    }
                }
                if major_unit_secondary.is_none() {
                    major_unit_secondary = axis_major_unit;
                }
            } else if is_horizontal_value_axis {
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
                    if let Some((f, lbl)) = extract_disp_units(va.display_units.as_deref()) {
                        disp_units = Some(f);
                        disp_units_label = lbl;
                    }
                }
                if major_unit.is_none() {
                    major_unit = axis_major_unit;
                }
                if val_axis_label_rotation.is_none() {
                    val_axis_label_rotation = axis_label_rotation(va.text_properties.as_deref());
                }
            }
            route_title(
                Some(&va.axis_position.val),
                va.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
                &mut x_axis_title_font,
                &mut y_axis_title_font,
                    &mut x_axis_title_fill,
                    &mut y_axis_title_fill,
                    &mut x_axis_title_border,
                    &mut y_axis_title_border,
            );
        }
    }

    let secondary_axis = !secondary_ax_ids.is_empty() && !primary_ax_ids.is_empty();

    fn axis_group_for(ax_ids: &[c::AxisId], sec: &[i32]) -> Option<String> {
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

    let mut bar_gap_width: Option<u16> = None;
    let mut bar_overlap: Option<i8> = None;
    let mut is_3d = false;
    let mut wireframe = false;
    let mut gap_depth: Option<f64> = None;
    let mut hole_size: Option<u8> = None;
    let mut first_slice_angle: Option<u16> = None;
    let mut of_pie_type: Option<String> = None;
    let mut split_type: Option<String> = None;
    let mut split_pos: Option<f64> = None;
    let mut second_pie_size: Option<u16> = None;
    let mut series_lines = false;

    let mut stock_hi_low_lines = false;
    let mut stock_up_down_bars = false;
    let mut stock_drop_lines = false;
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut categories: Vec<String> = Vec::new();
    let mut _categories_ref: Option<String> = None;
    let mut categories_format: Option<String> = None;
    let mut value_format: Option<String> = None;
    let mut chart_data_labels: Option<DataLabels> = None;

    let mut group_types: Vec<&'static str> = Vec::new();

    macro_rules! extract_chartlike {
        ($coll:expr, $kind:expr, $ax_ids:expr, $is_primary_group:expr) => {{
            let ag = axis_group_for($ax_ids, &secondary_ax_ids);
            for ser in $coll {
                let mut row = common_series(
                    &ser.order,
                    ser.series_text.as_deref(),
                    ser.chart_shape_properties.as_deref(),
                    ser.values.as_deref(),
                    &ser.data_point,
                    theme,
                );
                row.data_labels = extract_data_labels(ser.data_labels.as_deref());
                row.axis_group = ag.clone();
                row.chart_type = Some($kind.to_string());
                series.push(row);
                if $is_primary_group && categories.is_empty() {
                    let (cs, r, fmt) = ax_data_values(ser.category_axis_data.as_deref());
                    categories = cs;
                    _categories_ref = r;
                    categories_format = fmt;
                }
                if $is_primary_group && value_format.is_none() {
                    value_format = values_format(ser.values.as_deref());
                }
            }
        }};
    }

    macro_rules! extract_surface {
        ($coll:expr, $ax_ids:expr) => {{
            let ag = axis_group_for($ax_ids, &secondary_ax_ids);
            for ser in $coll {
                let mut row = common_series(
                    &ser.order,
                    ser.series_text.as_deref(),
                    ser.chart_shape_properties.as_deref(),
                    ser.values.as_deref(),
                    &[],
                    theme,
                );
                row.axis_group = ag.clone();
                row.chart_type = Some("surface".to_string());
                series.push(row);
                if categories.is_empty() {
                    let (cs, r, fmt) = ax_data_values(ser.category_axis_data.as_deref());
                    categories = cs;
                    _categories_ref = r;
                    categories_format = fmt;
                }
                if value_format.is_none() {
                    value_format = values_format(ser.values.as_deref());
                }
            }
        }};
    }

    for choice in &plot_area.plot_area_choice1 {
        match choice {
            c::PlotAreaChoice::BarChart(bc) => {
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
                    chart_data_labels = extract_data_labels(bc.data_labels.as_deref());
                }
                if bar_gap_width.is_none() {
                    bar_gap_width = bc.gap_width.as_ref().and_then(|g| g.val);
                }
                if bar_overlap.is_none() {
                    bar_overlap = bc.overlap.as_ref().and_then(|o| o.val);
                }
                let ag = axis_group_for(&bc.axis_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&bc.bar_chart_series, kind, &bc.axis_id, is_primary);
                for (offset, ser) in bc.bar_chart_series.iter().enumerate() {
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.trendlines = extract_trendlines(&ser.trendline, theme);
                        row.error_bars = extract_error_bars(ser.error_bars.as_deref(), theme);
                    }
                }
                group_types.push(kind);
            }
            c::PlotAreaChoice::LineChart(lc) => {
                if grouping.is_none() {
                    grouping = lc
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(lc.data_labels.as_deref());
                }
                let ag = axis_group_for(&lc.axis_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&lc.line_chart_series, "line", &lc.axis_id, is_primary);

                for (offset, ser) in lc.line_chart_series.iter().enumerate() {
                    let sym = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.marker_symbol = sym;
                        row.trendlines = extract_trendlines(&ser.trendline, theme);
                        row.error_bars = extract_error_bars(ser.error_bars.as_deref(), theme);
                    }
                }
                group_types.push("line");
            }
            c::PlotAreaChoice::AreaChart(ac) => {
                if grouping.is_none() {
                    grouping = ac
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(ac.data_labels.as_deref());
                }
                let ag = axis_group_for(&ac.axis_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&ac.area_chart_series, "area", &ac.axis_id, is_primary);
                for (offset, ser) in ac.area_chart_series.iter().enumerate() {
                    if let Some(row) = series.get_mut(series_before + offset) {
                        row.error_bars = extract_error_bars(ser.error_bars.first(), theme);
                    }
                }
                group_types.push("area");
            }
            c::PlotAreaChoice::PieChart(pc) => {
                chart_data_labels = extract_data_labels(pc.data_labels.as_deref());
                first_slice_angle = pc.first_slice_angle.as_ref().and_then(|f| f.val);
                extract_chartlike!(&pc.pie_chart_series, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::DoughnutChart(dc) => {
                chart_data_labels = extract_data_labels(dc.data_labels.as_deref());
                hole_size = Some(dc.hole_size.val);
                first_slice_angle = dc.first_slice_angle.as_ref().and_then(|f| f.val);
                extract_chartlike!(&dc.pie_chart_series, "doughnut", &[] as &[c::AxisId], true);
                group_types.push("doughnut");
                break;
            }
            c::PlotAreaChoice::ScatterChart(sc) => {
                chart_data_labels = extract_data_labels(sc.data_labels.as_deref());

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
                for ser in &sc.scatter_chart_series {
                    let mut row = common_series_scatter(
                        &ser.order,
                        ser.series_text.as_deref(),
                        ser.chart_shape_properties.as_deref(),
                        ser.y_values.as_deref(),
                        &ser.data_point,
                        theme,
                    );
                    let (xs, xref) = scatter_x_values(ser.x_values.as_deref());
                    row.x_values = xs;
                    row.x_values_ref = xref;
                    row.data_labels = extract_data_labels(ser.data_labels.as_deref());
                    row.axis_group = axis_group_for(&sc.axis_id, &secondary_ax_ids);
                    row.chart_type = Some("scatter".to_string());
                    row.marker_symbol = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    row.trendlines = extract_trendlines(&ser.trendline, theme);
                    let eb = ser
                        .error_bars
                        .iter()
                        .find(|e| {
                            e.error_direction
                                .as_ref()
                                .map(|d| matches!(d.val, c::ErrorBarDirectionValues::Y))
                                .unwrap_or(true)
                        })
                        .or_else(|| ser.error_bars.first());
                    row.error_bars = extract_error_bars(eb, theme);
                    series.push(row);
                    if categories.is_empty() {
                        let (cs, r, fmt) = x_axis_values(ser.x_values.as_deref());
                        categories = cs;
                        _categories_ref = r;
                        categories_format = fmt;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.y_values.as_deref());
                    }
                }
                group_types.push("scatter");
                break;
            }
            c::PlotAreaChoice::Bar3DChart(bc) => {
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
                    chart_data_labels = extract_data_labels(bc.data_labels.as_deref());
                }
                if bar_gap_width.is_none() {
                    bar_gap_width = bc.gap_width.as_ref().and_then(|g| g.val);
                }
                is_3d = true;
                if gap_depth.is_none() {
                    gap_depth = bc.gap_depth.as_ref().and_then(|g| g.val).map(f64::from);
                }
                let ag = axis_group_for(&bc.axis_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&bc.bar_chart_series, kind, &bc.axis_id, is_primary);
                group_types.push(kind);
            }
            c::PlotAreaChoice::Line3DChart(lc) => {
                if grouping.is_none() {
                    grouping = lc
                        .grouping
                        .val
                        .as_ref()
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(lc.data_labels.as_deref());
                }
                let ag = axis_group_for(&lc.axis_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&lc.line_chart_series, "line", &lc.axis_id, is_primary);
                for (offset, ser) in lc.line_chart_series.iter().enumerate() {
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
            c::PlotAreaChoice::Area3DChart(ac) => {
                if grouping.is_none() {
                    grouping = ac
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(ac.data_labels.as_deref());
                }
                let ag = axis_group_for(&ac.axis_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&ac.area_chart_series, "area", &ac.axis_id, is_primary);
                group_types.push("area");
            }
            c::PlotAreaChoice::Pie3DChart(pc) => {
                chart_data_labels = extract_data_labels(pc.data_labels.as_deref());
                extract_chartlike!(&pc.pie_chart_series, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::SurfaceChart(sc) => {
                is_3d = true;
                if sc.wireframe.as_ref().and_then(|w| w.val).map(bool::from) == Some(true) {
                    wireframe = true;
                }
                extract_surface!(&sc.surface_chart_series, &sc.axis_id);
                group_types.push("surface");
            }
            c::PlotAreaChoice::Surface3DChart(sc) => {
                is_3d = true;
                if sc.wireframe.as_ref().and_then(|w| w.val).map(bool::from) == Some(true) {
                    wireframe = true;
                }
                extract_surface!(&sc.surface_chart_series, &sc.axis_id);
                group_types.push("surface");
            }
            c::PlotAreaChoice::RadarChart(rc) => {
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
                    chart_data_labels = extract_data_labels(rc.data_labels.as_deref());
                }
                let series_before = series.len();
                extract_chartlike!(&rc.radar_chart_series, "radar", &rc.axis_id, true);

                for (offset, ser) in rc.radar_chart_series.iter().enumerate() {
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
            c::PlotAreaChoice::StockChart(sc) => {
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(sc.data_labels.as_deref());
                }
                stock_hi_low_lines = stock_hi_low_lines || sc.high_low_lines.is_some();
                stock_up_down_bars = stock_up_down_bars || sc.up_down_bars.is_some();
                stock_drop_lines = stock_drop_lines || sc.drop_lines.is_some();
                let series_before = series.len();
                extract_chartlike!(&sc.line_chart_series, "stock", &sc.axis_id, true);

                for (offset, ser) in sc.line_chart_series.iter().enumerate() {
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
            c::PlotAreaChoice::OfPieChart(pc) => {
                chart_data_labels = extract_data_labels(pc.data_labels.as_deref());
                of_pie_type = Some(
                    match pc.of_pie_type.val {
                        c::OfPieValues::Pie => "pie",
                        c::OfPieValues::Bar => "bar",
                    }
                    .to_string(),
                );
                split_type = pc.split_type.as_ref().map(|s| {
                    match s.val {
                        c::SplitValues::Custom => "cust",
                        c::SplitValues::Percent => "percent",
                        c::SplitValues::Position => "pos",
                        c::SplitValues::Value => "val",
                    }
                    .to_string()
                });
                split_pos = pc.split_position.as_ref().map(|s| s.val);
                second_pie_size = pc.second_pie_size.as_ref().and_then(|s| s.val);
                series_lines = !pc.series_lines.is_empty();
                extract_chartlike!(&pc.pie_chart_series, "ofpie", &[] as &[c::AxisId], true);
                group_types.push("ofpie");
                break;
            }
            c::PlotAreaChoice::BubbleChart(bc) => {
                bubble_scale = bc.bubble_scale.as_ref().and_then(|s| s.val);
                size_represents = bc
                    .size_represents
                    .as_ref()
                    .and_then(|s| s.val.as_ref())
                    .map(|v| {
                        match v {
                            c::SizeRepresentsValues::Area => "area",
                            c::SizeRepresentsValues::Width => "w",
                        }
                        .to_string()
                    });
                chart_data_labels = extract_data_labels(bc.data_labels.as_deref());
                for ser in &bc.bubble_chart_series {
                    let mut row = common_series_scatter(
                        &ser.order,
                        ser.series_text.as_deref(),
                        ser.chart_shape_properties.as_deref(),
                        ser.y_values.as_deref(),
                        &ser.data_point,
                        theme,
                    );
                    let (xs, xref) = scatter_x_values(ser.x_values.as_deref());
                    row.x_values = xs;
                    row.x_values_ref = xref;

                    let (sizes, sref) = bubble_size_values(ser.bubble_size.as_deref());
                    row.bubble_sizes = sizes;
                    row.bubble_sizes_ref = sref;
                    row.data_labels = extract_data_labels(ser.data_labels.as_deref());
                    row.axis_group = axis_group_for(&bc.axis_id, &secondary_ax_ids);
                    row.chart_type = Some("bubble".to_string());
                    series.push(row);
                    if categories.is_empty() {
                        let (cs, r, fmt) = x_axis_values(ser.x_values.as_deref());
                        categories = cs;
                        _categories_ref = r;
                        categories_format = fmt;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.y_values.as_deref());
                    }
                }
                group_types.push("bubble");
                break;
            }
            _ => {}
        }
    }

    let unique_types: std::collections::BTreeSet<&&str> = group_types.iter().collect();
    let chart_type = match unique_types.len() {
        0 => "unknown".to_string(),
        1 => (*group_types.first().unwrap()).to_string(),
        _ => "combo".to_string(),
    };

    if chart_type != "combo" {
        for s in &mut series {
            s.chart_type = None;
        }
    }

    if !secondary_axis {
        for s in &mut series {
            s.axis_group = None;
        }
    }
    let value_format = value_format.or(primary_val_fmt);

    let auto_title_deleted = chart
        .auto_title_deleted
        .as_ref()
        .and_then(|a| a.val)
        .map(bool::from)
        .unwrap_or(false);
    let title = extract_title(chart.title.as_deref()).or_else(|| {
        if chart.title.is_some() && !auto_title_deleted && series.len() == 1 {
            let n = series[0].name.clone();
            if n.is_empty() {
                None
            } else {
                Some(n)
            }
        } else {
            None
        }
    });
    let title_font = extract_title_font(chart.title.as_deref());
    let title_sp = chart
        .title
        .as_deref()
        .and_then(|t| t.chart_shape_properties.as_deref());
    let title_fill = title_sp.and_then(|sp| fill_color_outside_outline(&format!("{:?}", sp), theme));
    let title_border = title_sp
        .and_then(|sp| extract_style_border(sp.outline.as_deref(), &format!("{:?}", sp), theme));

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

    let plot_area_sp = plot_area.shape_properties.as_deref();
    let plot_area_fill =
        plot_area_sp.and_then(|sp| fill_color_outside_outline(&format!("{:?}", sp), theme));
    let plot_area_border = plot_area_sp
        .and_then(|sp| extract_style_border(sp.outline.as_deref(), &format!("{:?}", sp), theme));

    let legend = chart.legend.as_deref();
    let legend_sp = legend.and_then(|l| l.chart_shape_properties.as_deref());
    let legend_fill =
        legend_sp.and_then(|sp| fill_color_outside_outline(&format!("{:?}", sp), theme));
    let legend_border = legend_sp
        .and_then(|sp| extract_style_border(sp.outline.as_deref(), &format!("{:?}", sp), theme));
    let legend_font = legend.and_then(|l| extract_style_font(l.text_properties.as_deref(), theme));

    let view_3d = chart.view3_d.as_deref().map(|v| ChartView3D {
        rot_x: v.rotate_x.as_ref().and_then(|r| r.val).map(f64::from),
        rot_y: v.rotate_y.as_ref().and_then(|r| r.val).map(f64::from),
        perspective: v.perspective.as_ref().and_then(|p| p.val).map(f64::from),
        right_angle_axes: v
            .right_angle_axes
            .as_ref()
            .and_then(|r| r.val)
            .map(bool::from),
        depth_percent: v.depth_percent.as_ref().and_then(|d| d.val).map(f64::from),
        height_percent: v.height_percent.as_ref().and_then(|h| h.val).map(f64::from),
    });
    let surface_fill = |sp: Option<&c::ShapeProperties>| {
        sp.and_then(|s| fill_color_outside_outline(&format!("{:?}", s), theme))
    };
    let floor_fill = surface_fill(
        chart
            .floor
            .as_deref()
            .and_then(|f| f.shape_properties.as_deref()),
    );
    let side_wall_fill = surface_fill(
        chart
            .side_wall
            .as_deref()
            .and_then(|w| w.shape_properties.as_deref()),
    );
    let back_wall_fill = surface_fill(
        chart
            .back_wall
            .as_deref()
            .and_then(|w| w.shape_properties.as_deref()),
    );

    let plot_area_layout = extract_manual_layout(plot_area.layout.as_deref());
    let legend_layout = legend.and_then(|l| extract_manual_layout(l.layout.as_deref()));
    let title_layout = chart
        .title
        .as_deref()
        .and_then(|t| extract_manual_layout(t.layout.as_deref()));

    Some(Chart {
        chart_type,
        title,
        title_font,
        title_fill,
        title_border,
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
        hole_size,
        first_slice_angle,
        of_pie_type,
        split_type,
        split_pos,
        second_pie_size,
        series_lines,
        x_axis_title,
        x_axis_title_font,
        x_axis_title_fill,
        x_axis_title_border,
        y_axis_title,
        y_axis_title_font,
        y_axis_title_fill,
        y_axis_title_border,
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
        cx_region_map_min_color: None,
        cx_region_map_mid_color: None,
        cx_region_map_max_color: None,
        cat_axis_label_rotation,
        val_axis_label_rotation,
        data_table,
        plot_area_fill,
        plot_area_border,
        legend_fill,
        legend_border,
        legend_font,
        plot_area_layout,
        legend_layout,
        title_layout,
        is_3d,
        wireframe,
        view_3d,
        gap_depth,
        floor_fill,
        side_wall_fill,
        back_wall_fill,
    })
}
