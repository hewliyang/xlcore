use crate::chart_colors::*;
use crate::charts_helpers::*;
use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;

pub(super) fn extract_chart(space: &c::ChartSpace, theme: Option<&Theme>) -> Option<Chart> {
    let chart = space.c_chart.as_ref()?;
    let plot_area = &chart.plot_area;

    // Pre-scan plot_area_choice2 for value axes. We need to know which
    // axId belongs to the primary (left/bottom) vs secondary (right/top)
    // value axis so that, per chart-type group below, we can tag its
    // series with axis_group = primary/secondary. We also stash the
    // secondary numFmt to expose as Chart.value_format_secondary.
    let mut secondary_ax_ids: Vec<u32> = Vec::new();
    let mut primary_ax_ids: Vec<u32> = Vec::new();
    let mut primary_val_fmt: Option<String> = None;
    let mut secondary_val_fmt: Option<String> = None;
    let mut value_min: Option<f64> = None;
    let mut value_max: Option<f64> = None;
    let mut value_min_secondary: Option<f64> = None;
    let mut value_max_secondary: Option<f64> = None;
    // Axis titles. ECMA-376 §21.2.2.210 — every axis CT carries an
    // optional `<c:title>` (same `CT_Title` shape as the chart title).
    // We route by `axPos`: `b`/`t` → x-axis (catAx/dateAx), `l` →
    // y-axis, `r` → secondary y-axis.
    let mut x_axis_title: Option<String> = None;
    let mut y_axis_title: Option<String> = None;
    let mut y_axis_title_secondary: Option<String> = None;
    // `<c:majorGridlines>` toggle per value axis. ECMA-376 §21.2.2.100:
    // gridlines paint iff the element is present, and `<a:noFill/>` on
    // its line suppresses the stroke even when present. None ⇒ the
    // value axis is absent on this side; we collapse that to "don't
    // paint" at the renderer.
    let mut show_major_gridlines: Option<bool> = None;
    let mut show_major_gridlines_secondary: Option<bool> = None;
    // `<c:dispUnits>` per value axis. ECMA-376 §21.2.2.45:
    // tick labels on the axis are divided by `disp_units` before
    // formatting, and `disp_units_label` (if present) is painted near
    // the axis as a caption (e.g. "S$ mn" with `builtInUnit=thousands`).
    let mut disp_units: Option<f64> = None;
    let mut disp_units_label: Option<String> = None;
    let mut disp_units_secondary: Option<f64> = None;
    let mut disp_units_label_secondary: Option<String> = None;
    // `<c:majorUnit val="N"/>` per value axis (ECMA-376 §21.2.2.103).
    // When authored, the renderer steps ticks by exactly N source
    // units instead of niceTicks; lets workbooks pin cadences like
    // 9000 (NWC line chart, dispUnits=thousands → 0/9/18/27/36/45).
    let mut major_unit: Option<f64> = None;
    let mut major_unit_secondary: Option<f64> = None;
    // Route an axis's title to x / y / y-secondary by its `axPos`.
    // ECMA-376: `b`/`t` → horizontal, `l` → vertical, `r` → secondary
    // vertical. This is intentionally axis-*position*-driven (not
    // axis-*type*-driven) so horizontal bar charts — where the
    // catAx is at `l` and the valAx is at `b` — land their titles
    // on the correct edge.
    let route_title = |pos: Option<&c::AxisPositionValues>,
                       title: Option<&c::Title>,
                       x: &mut Option<String>,
                       y: &mut Option<String>,
                       y2: &mut Option<String>| {
        let Some(t) = extract_title(title) else {
            return;
        };
        match pos {
            Some(c::AxisPositionValues::Bottom) | Some(c::AxisPositionValues::Top) => {
                if x.is_none() {
                    *x = Some(t);
                }
            }
            Some(c::AxisPositionValues::Right) => {
                if y2.is_none() {
                    *y2 = Some(t);
                }
            }
            // Default / `Left` / unknown → primary y.
            _ => {
                if y.is_none() {
                    *y = Some(t);
                }
            }
        }
    };
    for choice in &plot_area.plot_area_choice2 {
        match choice {
            c::PlotAreaChoice2::CCatAx(ca) => route_title(
                ca.axis_position.as_ref().map(|p| &p.val),
                ca.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
            ),
            c::PlotAreaChoice2::CDateAx(da) => route_title(
                da.axis_position.as_ref().map(|p| &p.val),
                da.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
            ),
            _ => {}
        }
        if let c::PlotAreaChoice2::CValAx(va) = choice {
            let axid = va.axis_id.as_ref().map(|a| a.val).unwrap_or(0);
            let pos = va.axis_position.as_ref().map(|p| &p.val);
            let is_secondary = matches!(
                pos,
                Some(c::AxisPositionValues::Right) | Some(c::AxisPositionValues::Top)
            );
            // Scatter charts emit TWO `<c:valAx>` blocks — the numeric
            // x-axis at `axPos="b"` and the y-axis at `axPos="l"`. Only
            // the latter is the conceptual "primary value axis" whose
            // gridlines/format/scaling we want; the x-axis valAx should
            // be ignored for those concerns. (For bar/line/area charts
            // the catAx sits at b/t and there's only one valAx at l/r,
            // so this filter is a no-op.)
            let is_horizontal_value_axis =
                matches!(pos, Some(c::AxisPositionValues::Bottom)) && !is_secondary;
            // Major gridlines: present ∧ line not `<a:noFill/>`.
            // The MajorGridlines element only carries an optional
            // `<c:spPr>`; we look at its Debug repr for an `ANoFill`
            // token inside the line block (same pragma as the series
            // color resolver). When spPr is absent the default stroke
            // applies, so "present without noFill" ⇒ show. Element
            // entirely absent ⇒ don't paint (ECMA-376 §21.2.2.100).
            //
            // We deliberately always emit `Some(bool)` here instead of
            // mapping through `.map()` — the schema's `Option<bool>` is
            // for "no value axis exists at all" (pie/doughnut), not for
            // "value axis exists but its `<c:majorGridlines>` element is
            // absent". A None on the wire would let the renderer's
            // `!== false` back-compat fallback paint gridlines that
            // weren't authored.
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
            // `<c:scaling><c:min>` / `<c:max>`: explicit axis bounds.
            // Either may be absent (Excel auto-picks); both come
            // through as `f64` so just forward to schema.
            let scaling_min = va
                .scaling
                .as_ref()
                .and_then(|s| s.min_axis_value.as_ref())
                .map(|m| m.val);
            let scaling_max = va
                .scaling
                .as_ref()
                .and_then(|s| s.max_axis_value.as_ref())
                .map(|m| m.val);
            // `<c:majorUnit>` sits *outside* `<c:scaling>` directly on
            // the valAx (per ECMA-376 §21.2.2.226) — not nested like
            // min/max. Positive-finite values only; OOXML allows any
            // positive double but niceTicks already handles boundary
            // weirdness so we re-validate here for safety.
            let axis_major_unit = va
                .c_major_unit
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
                    if let Some((f, lbl)) = extract_disp_units(va.c_disp_units.as_deref()) {
                        disp_units_secondary = Some(f);
                        disp_units_label_secondary = lbl;
                    }
                }
                if major_unit_secondary.is_none() {
                    major_unit_secondary = axis_major_unit;
                }
            } else if is_horizontal_value_axis {
                // Scatter x-axis: contributes its axId to primary_ax_ids
                // (so series axis-group resolution still works) but
                // not its gridlines / numFmt / scaling — those belong
                // to the y-axis.
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
                    if let Some((f, lbl)) = extract_disp_units(va.c_disp_units.as_deref()) {
                        disp_units = Some(f);
                        disp_units_label = lbl;
                    }
                }
                if major_unit.is_none() {
                    major_unit = axis_major_unit;
                }
            }
            route_title(
                va.axis_position.as_ref().map(|p| &p.val),
                va.title.as_deref(),
                &mut x_axis_title,
                &mut y_axis_title,
                &mut y_axis_title_secondary,
            );
        }
    }

    // Edge case: no axPos on either side. Treat first valAx encountered
    // as primary so single-axis charts still render.
    let secondary_axis = !secondary_ax_ids.is_empty() && !primary_ax_ids.is_empty();

    /// Resolve a chart-type group's axIds (a Vec<AxisId> with 2 entries:
    /// one cat-axis ref, one val-axis ref) to "primary" / "secondary".
    /// Defaults to primary when neither axId matches a known valAx —
    /// the safest fallback for malformed/legacy files.
    fn axis_group_for(ax_ids: &[c::AxisId], sec: &[u32]) -> Option<String> {
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
    // ECMA-376 §21.2.2.75 / §21.2.2.131. Captured from the first
    // `<c:barChart>` group encountered (a chart can technically host
    // multiple bar groups but Excel writes one); combo charts get the
    // bar-side values, line/area groups don't carry these.
    let mut bar_gap_width: Option<u16> = None;
    let mut bar_overlap: Option<i8> = None;
    // Stock-chart decoration toggles. ECMA-376 §21.2.2.198 lets
    // `<c:stockChart>` host `<c:hiLowLines/>`, `<c:upDownBars/>`,
    // `<c:dropLines/>` as optional children.
    let mut stock_hi_low_lines = false;
    let mut stock_up_down_bars = false;
    let mut stock_drop_lines = false;
    let mut series: Vec<ChartSeries> = Vec::new();
    let mut categories: Vec<String> = Vec::new();
    let mut _categories_ref: Option<String> = None;
    let mut categories_format: Option<String> = None;
    let mut value_format: Option<String> = None;
    let mut chart_data_labels: Option<DataLabels> = None;

    // Track every chart-type tag we encounter so we can emit `combo`
    // when more than one is present in the same plotArea.
    let mut group_types: Vec<&'static str> = Vec::new();

    // Helper macro: extract a chart-type group's series, tagging each
    // with `axis_group` (when the chart has a secondary axis) and
    // `chart_type` (per-series override, for combo rendering). The
    // chart-level categories/value_format are taken from the first
    // primary-axis group we see; combo charts otherwise inherit the
    // primary scale.
    macro_rules! extract_chartlike {
        ($coll:expr, $kind:expr, $ax_ids:expr, $is_primary_group:expr) => {{
            let ag = axis_group_for($ax_ids, &secondary_ax_ids);
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
                row.axis_group = ag.clone();
                row.chart_type = Some($kind.to_string());
                series.push(row);
                if $is_primary_group && categories.is_empty() {
                    let (cs, r, fmt) = ax_data_values(ser.c_cat.as_deref());
                    categories = cs;
                    _categories_ref = r;
                    categories_format = fmt;
                }
                if $is_primary_group && value_format.is_none() {
                    value_format = values_format(ser.c_val.as_deref());
                }
            }
        }};
    }

    for choice in &plot_area.plot_area_choice1 {
        match choice {
            c::PlotAreaChoice::CBarChart(bc) => {
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
                    chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                }
                if bar_gap_width.is_none() {
                    bar_gap_width = bc.c_gap_width.as_ref().and_then(|g| g.val);
                }
                if bar_overlap.is_none() {
                    bar_overlap = bc.c_overlap.as_ref().and_then(|o| o.val);
                }
                let ag = axis_group_for(&bc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&bc.c_ser, kind, &bc.c_ax_id, is_primary);
                group_types.push(kind);
            }
            c::PlotAreaChoice::CLineChart(lc) => {
                if grouping.is_none() {
                    grouping = lc
                        .grouping
                        .val
                        .as_ref()
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(lc.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&lc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&lc.c_ser, "line", &lc.c_ax_id, is_primary);
                // Propagate per-series `<c:marker><c:symbol val="..."/>`
                // (LineChartSeries is the only series shape with a top-
                // level `marker` field, so this can't go in the macro).
                for (offset, ser) in lc.c_ser.iter().enumerate() {
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
            c::PlotAreaChoice::CAreaChart(ac) => {
                if grouping.is_none() {
                    grouping = ac
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(ac.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&ac.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&ac.c_ser, "area", &ac.c_ax_id, is_primary);
                group_types.push("area");
            }
            c::PlotAreaChoice::CPieChart(pc) => {
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                // Pie has no axes in OOXML; pass an empty slice.
                extract_chartlike!(&pc.c_ser, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::CDoughnutChart(dc) => {
                chart_data_labels = extract_data_labels(dc.c_d_lbls.as_deref());
                extract_chartlike!(&dc.c_ser, "doughnut", &[] as &[c::AxisId], true);
                group_types.push("doughnut");
                break;
            }
            c::PlotAreaChoice::CScatterChart(sc) => {
                chart_data_labels = extract_data_labels(sc.c_d_lbls.as_deref());
                // ECMA-376 §21.2.2.162 / §21.2.3.40: scatterStyle / ST_ScatterStyle val is required
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
                    row.axis_group = axis_group_for(&sc.c_ax_id, &secondary_ax_ids);
                    row.chart_type = Some("scatter".to_string());
                    row.marker_symbol = ser
                        .marker
                        .as_ref()
                        .and_then(|m| m.symbol.as_ref())
                        .map(|s| marker_symbol_str(&s.val));
                    series.push(row);
                    if categories.is_empty() {
                        // Stash the x-axis ref for axis labels (numeric).
                        let (cs, r, fmt) = x_axis_values(ser.c_x_val.as_deref());
                        categories = cs;
                        _categories_ref = r;
                        categories_format = fmt;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.c_y_val.as_deref());
                    }
                }
                group_types.push("scatter");
                break;
            }
            c::PlotAreaChoice::CBar3DChart(bc) => {
                // Legacy 3D variant: dispatch to the 2D bar painter.
                // 3D-only flourishes (gap_depth, shape, perspective)
                // are intentionally dropped — Excel's 3D chart visuals
                // are out of scope for v0; the data + 2D layout match
                // ECMA-376 well enough for HITL preview.
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
                    chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                }
                if bar_gap_width.is_none() {
                    bar_gap_width = bc.c_gap_width.as_ref().and_then(|g| g.val);
                }
                let ag = axis_group_for(&bc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&bc.c_ser, kind, &bc.c_ax_id, is_primary);
                group_types.push(kind);
            }
            c::PlotAreaChoice::CLine3DChart(lc) => {
                if grouping.is_none() {
                    grouping = lc
                        .grouping
                        .val
                        .as_ref()
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(lc.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&lc.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                let series_before = series.len();
                extract_chartlike!(&lc.c_ser, "line", &lc.c_ax_id, is_primary);
                for (offset, ser) in lc.c_ser.iter().enumerate() {
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
            c::PlotAreaChoice::CArea3DChart(ac) => {
                if grouping.is_none() {
                    grouping = ac
                        .grouping
                        .as_ref()
                        .and_then(|g| g.val.as_ref())
                        .map(|v| format!("{:?}", v).to_ascii_lowercase());
                }
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(ac.c_d_lbls.as_deref());
                }
                let ag = axis_group_for(&ac.c_ax_id, &secondary_ax_ids);
                let is_primary = !matches!(ag.as_deref(), Some("secondary"));
                extract_chartlike!(&ac.c_ser, "area", &ac.c_ax_id, is_primary);
                group_types.push("area");
            }
            c::PlotAreaChoice::CPie3DChart(pc) => {
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                extract_chartlike!(&pc.c_ser, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::CRadarChart(rc) => {
                // ECMA-376 §21.2.2.153 (radarChart) / §21.2.2.154 (radarStyle). Radar charts are
                // category-axis + value-axis the same way line charts
                // are, so we reuse the line series shape: idx/order/tx/
                // spPr/cat/val/dPt/dLbls. The renderer wraps the
                // category axis into a polar layout. `radarStyle`:
                //   - `standard` — line only
                //   - `marker`   — line + markers (Excel UI default)
                //   - `filled`   — filled polygon (semi-transparent)
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
                    chart_data_labels = extract_data_labels(rc.c_d_lbls.as_deref());
                }
                let series_before = series.len();
                extract_chartlike!(&rc.c_ser, "radar", &rc.c_ax_id, true);
                // RadarChartSeries also carries a top-level `marker`
                // node (same shape as LineChartSeries); propagate it
                // so per-series marker symbol overrides survive.
                for (offset, ser) in rc.c_ser.iter().enumerate() {
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
            c::PlotAreaChoice::CStockChart(sc) => {
                // ECMA-376 §21.2.2.198. Stock charts are line-shaped
                // series (LineChartSeries) with optional hiLowLines /
                // upDownBars / dropLines decoration. The series count
                // implies subtype:
                //   3  → High-Low-Close (HLC)
                //   4  → Open-High-Low-Close (OHLC)
                //   4  → Volume-High-Low-Close (VHLC, if first series
                //          is on a secondary axis as a column group;
                //          xlsxwriter emits this differently, with a
                //          parallel `<c:barChart>` for volume)
                //   5  → Volume-Open-High-Low-Close (VOHLC)
                // We just expose series + decoration flags; the
                // renderer infers subtype from `series.length`.
                if chart_data_labels.is_none() {
                    chart_data_labels = extract_data_labels(sc.c_d_lbls.as_deref());
                }
                stock_hi_low_lines = stock_hi_low_lines || sc.c_hi_low_lines.is_some();
                stock_up_down_bars = stock_up_down_bars || sc.c_up_down_bars.is_some();
                stock_drop_lines = stock_drop_lines || sc.c_drop_lines.is_some();
                let series_before = series.len();
                extract_chartlike!(&sc.c_ser, "stock", &sc.c_ax_id, true);
                // StockChart shares LineChartSeries; propagate the
                // per-series marker symbol (same as line). xlsxwriter
                // emits `<c:marker><c:symbol val="none"/></c:marker>`
                // on high/low and `<c:symbol val="dot"/>` on close —
                // we honor that so only close paints a marker.
                for (offset, ser) in sc.c_ser.iter().enumerate() {
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
            c::PlotAreaChoice::COfPieChart(pc) => {
                // ECMA-376 §21.2.2.127. `ofPieType` (`pie` | `bar`)
                // would split the second plot into either a satellite
                // pie or bar of grouped slices; we approximate as a
                // plain pie until the satellite layout lands.
                chart_data_labels = extract_data_labels(pc.c_d_lbls.as_deref());
                extract_chartlike!(&pc.c_ser, "pie", &[] as &[c::AxisId], true);
                group_types.push("pie");
                break;
            }
            c::PlotAreaChoice::CBubbleChart(bc) => {
                // ECMA-376 §21.2.2.21 (bubbleScale) / §21.2.2.19 (bubble3D): bubbleScale (0..=300,
                // default 100), sizeRepresents (`area` default or `w`).
                bubble_scale = bc.c_bubble_scale.as_ref().and_then(|s| s.val);
                size_represents = bc
                    .c_size_represents
                    .as_ref()
                    .and_then(|s| s.val.as_ref())
                    .map(|v| {
                        match v {
                            c::SizeRepresentsValues::Area => "area",
                            c::SizeRepresentsValues::Width => "w",
                        }
                        .to_string()
                    });
                chart_data_labels = extract_data_labels(bc.c_d_lbls.as_deref());
                for ser in &bc.c_ser {
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
                    // BubbleSize shares the `CT_NumDataSource` shape
                    // with YValues / Values. Reuse the YValues parser
                    // by transmuting through a shared accessor.
                    let (sizes, sref) = bubble_size_values(ser.c_bubble_size.as_deref());
                    row.bubble_sizes = sizes;
                    row.bubble_sizes_ref = sref;
                    row.data_labels = extract_data_labels(ser.c_d_lbls.as_deref());
                    row.axis_group = axis_group_for(&bc.c_ax_id, &secondary_ax_ids);
                    row.chart_type = Some("bubble".to_string());
                    series.push(row);
                    if categories.is_empty() {
                        let (cs, r, fmt) = x_axis_values(ser.c_x_val.as_deref());
                        categories = cs;
                        _categories_ref = r;
                        categories_format = fmt;
                    }
                    if value_format.is_none() {
                        value_format = y_values_format(ser.c_y_val.as_deref());
                    }
                }
                group_types.push("bubble");
                break;
            }
            _ => {}
        }
    }

    // Single-type charts: collapse to the historical scalar type.
    // Multi-type plotArea ⇒ "combo"; renderer dispatches per-series.
    let unique_types: std::collections::BTreeSet<&&str> = group_types.iter().collect();
    let chart_type = match unique_types.len() {
        0 => "unknown".to_string(),
        1 => (*group_types.first().unwrap()).to_string(),
        _ => "combo".to_string(),
    };
    // When the chart isn't a combo, clear per-series chart_type so we
    // don't bloat the JSON output for the common case.
    if chart_type != "combo" {
        for s in &mut series {
            s.chart_type = None;
        }
    }
    // Same for axis_group when no secondary axis exists.
    if !secondary_axis {
        for s in &mut series {
            s.axis_group = None;
        }
    }
    let value_format = value_format.or(primary_val_fmt);

    // Title resolution (ECMA-376 §21.2.2.210 title + §21.2.2.7 autoTitleDeleted):
    //   - `<c:title><c:tx>...` explicit text wins.
    //   - `<c:title>` present *without* `<c:tx>` AND `<c:autoTitleDeleted
    //     val="0"/>` (or element absent, which defaults to false) AND
    //     the chart has exactly one series → Excel auto-generates the
    //     title from that series's name. This is how the AGS NWC line
    //     chart picks up its "NWC" title even though chart15.xml's
    //     `<c:title>` carries only `<c:spPr>`/`<c:txPr>` (formatting,
    //     no text node).
    //   - `<c:autoTitleDeleted val="1"/>` → user explicitly cleared
    //     the auto title; we honor that and emit no title.
    let auto_title_deleted = chart
        .auto_title_deleted
        .as_ref()
        .and_then(|a| a.val)
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
    // Legend presence + position. Critical distinction:
    //   - `<c:legend>` absent             → legend_pos = None        (don't paint)
    //   - `<c:legend>` present, no <c:legendPos>  → legend_pos = Some("r")  (Excel default)
    //   - `<c:legend>` present with <c:legendPos>  → legend_pos = Some(<that>)
    // The renderer treats `None` as "no legend". This matters because
    // many AGS workbook charts have no `<c:legend>` element at all
    // (e.g. the per-data-point waterfall on `Charts_Chart_2.xlsx` —
    // see parity-charts.md Bug #3) and Excel desktop / hsx correctly
    // omit the legend; pre-fix we were defaulting to "b" whenever
    // `legend_pos` was `None`, fabricating a legend for every chart.
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

    Some(Chart {
        chart_type,
        title,
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
        x_axis_title,
        y_axis_title,
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
    })
}
