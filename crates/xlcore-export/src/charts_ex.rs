use crate::chart_colors::*;
use crate::schema::*;
use ooxmlsdk::schemas::schemas_microsoft_com_office_drawing_2014_chartex as cx;

/// Map a chartEx `SeriesLayout` to the schema's `cx_layout` string.
fn cx_layout_name(l: &cx::SeriesLayout) -> &'static str {
    match l {
        cx::SeriesLayout::Waterfall => "waterfall",
        cx::SeriesLayout::Funnel => "funnel",
        cx::SeriesLayout::Treemap => "treemap",
        cx::SeriesLayout::Sunburst => "sunburst",
        cx::SeriesLayout::BoxWhisker => "boxWhisker",
        cx::SeriesLayout::ParetoLine => "paretoLine",
        cx::SeriesLayout::RegionMap => "regionMap",
        cx::SeriesLayout::ClusteredColumn => "clusteredColumn",
    }
}

/// Extract title text from a chartEx `<cx:title>` element. Mirrors
/// `extract_title` for legacy charts but walks the chartEx-namespaced
/// `Text` / `RichTextBody` shape.
fn extract_chart_ex_title(t: Option<&cx::ChartTitle>) -> Option<String> {
    let t = t?;
    let text = t.text.as_deref()?;
    let choice = text.text_choice.as_ref()?;
    match choice {
        cx::TextChoice::CxTxData(td) => extract_text_data_v(td),
        cx::TextChoice::CxRich(rich) => {
            // Concatenate `<a:t>` text across each paragraph's runs.
            // chartEx rich text reuses the regular drawingml namespace
            // for paragraphs / runs, so we walk the `a:` types from
            // `ooxmlsdk::schemas::...drawingml_2006_main`.
            use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
            let mut out = String::new();
            for p in &rich.a_p {
                for ch in &p.paragraph_choice {
                    if let a::ParagraphChoice::AR(run) = ch {
                        out.push_str(run.text.as_str());
                    }
                }
            }
            if out.is_empty() {
                None
            } else {
                Some(out)
            }
        }
    }
}

/// Pull the inline `<cx:v>` text from a `<cx:txData>` block, whether
/// the schema picked the bare `CxV` variant or the multi-child
/// `Sequence` variant.
fn extract_text_data_v(td: &cx::TextData) -> Option<String> {
    match td.text_data_choice.as_ref()? {
        cx::TextDataChoice::CxV(s) => Some(s.clone()),
        cx::TextDataChoice::Sequence { v_xsdstring, .. } => v_xsdstring.clone(),
    }
}

/// One series' parsed data — produced by `parse_series_data`. Captures
/// the categories from the series's data block (only relevant for the
/// first/primary series in multi-series chartEx — subsequent series'
/// own categories are ignored), plus that series's numeric values and
/// the formula reference used to fill them in via `refs.rs`.
struct ParsedSeriesData {
    categories: Vec<String>,
    categories_ref: Option<String>,
    values: Vec<f64>,
    values_ref: Option<String>,
    value_format: Option<String>,
}

/// Resolve a chartEx series's `<cx:dataId>` to its `<cx:data>` block
/// under `<cx:chartData>`, then walk the inner dimensions to extract
/// categories + numeric values. Returns `None` when the chartData
/// block is missing entirely.
fn parse_series_data(space: &cx::ChartSpace, series: &cx::Series) -> Option<ParsedSeriesData> {
    let data_id = series.cx_data_id.as_ref().map(|d| d.val).unwrap_or(0);
    let data_block = space
        .chart_data
        .as_deref()
        .and_then(|cd| cd.cx_data.iter().find(|d| d.id == data_id))
        .or_else(|| {
            space
                .chart_data
                .as_deref()
                .and_then(|cd| cd.cx_data.first())
        })?;

    let mut categories: Vec<String> = Vec::new();
    let mut categories_ref: Option<String> = None;
    let mut values: Vec<f64> = Vec::new();
    let mut values_ref: Option<String> = None;
    let mut value_format: Option<String> = None;

    for choice in &data_block.data_choice {
        match choice {
            cx::DataChoice::CxStrDim(sd) => {
                if !matches!(sd.r#type, cx::StringDimensionType::Cat) {
                    continue;
                }
                let levels: Vec<&cx::StringLevel> = match sd.string_dimension_choice.as_ref() {
                    Some(cx::StringDimensionChoice::Sequence(seq)) => {
                        if let Some(s) = seq.formula.xml_content.as_ref() {
                            categories_ref = Some(s.clone());
                        }
                        seq.string_level.iter().collect()
                    }
                    Some(cx::StringDimensionChoice::CxLvl(lvl)) => vec![lvl.as_ref()],
                    None => Vec::new(),
                };
                if let Some(lvl) = levels.first() {
                    let n = lvl.pt_count as usize;
                    categories = vec![String::new(); n];
                    for pt in &lvl.cx_pt {
                        let i = pt.index as usize;
                        if i < n {
                            categories[i] = pt.xml_content.clone().unwrap_or_default();
                        }
                    }
                }
            }
            cx::DataChoice::CxNumDim(nd) => {
                // Funnel / waterfall / pareto use `type="val"`; treemap /
                // sunburst / histogram use `type="size"`; regionMap
                // uses `type="colorVal"` (the dimension drives the
                // choropleth color scale rather than a y-axis value).
                // All three map to the same `values` vector — the
                // per-layout painter knows what the numbers mean.
                if !matches!(
                    nd.r#type,
                    cx::NumericDimensionType::Val
                        | cx::NumericDimensionType::Size
                        | cx::NumericDimensionType::ColorVal
                ) {
                    continue;
                }
                let levels: Vec<&cx::NumericLevel> = match nd.numeric_dimension_choice.as_ref() {
                    Some(cx::NumericDimensionChoice::Sequence(seq)) => {
                        if let Some(s) = seq.formula.xml_content.as_ref() {
                            values_ref = Some(s.clone());
                        }
                        seq.numeric_level.iter().collect()
                    }
                    Some(cx::NumericDimensionChoice::CxLvl(lvl)) => vec![lvl.as_ref()],
                    None => Vec::new(),
                };
                if let Some(lvl) = levels.first() {
                    let n = lvl.pt_count as usize;
                    values = vec![0.0; n];
                    for pt in &lvl.cx_pt {
                        let i = pt.idx as usize;
                        if i < n {
                            values[i] = pt.xml_content.unwrap_or(0.0);
                        }
                    }
                    if let Some(fc) = &lvl.format_code {
                        value_format = Some(fc.clone());
                    }
                }
            }
        }
    }

    Some(ParsedSeriesData {
        categories,
        categories_ref,
        values,
        values_ref,
        value_format,
    })
}

/// Extract a series's display name from its `<cx:tx><cx:txData><cx:v>`.
fn parse_series_name(series: &cx::Series) -> String {
    series
        .text
        .as_deref()
        .and_then(|t| t.text_choice.as_ref())
        .and_then(|c| match c {
            cx::TextChoice::CxTxData(td) => extract_text_data_v(td),
            cx::TextChoice::CxRich(_) => None,
        })
        .unwrap_or_default()
}

/// Build a bare `ChartSeries` carrying just name + values. chartEx
/// series don't currently surface per-series colors / data labels /
/// axis-group toggles — those slots stay at defaults for the renderer
/// to fill in (e.g. boxWhisker accents come from the theme).
fn make_chart_series(name: String, values: Vec<f64>, values_ref: Option<String>) -> ChartSeries {
    ChartSeries {
        name,
        name_ref: None,
        color: None,
        values,
        values_ref,
        x_values: Vec::new(),
        x_values_ref: None,
        bubble_sizes: Vec::new(),
        bubble_sizes_ref: None,
        point_colors: Vec::new(),
        data_labels: None,
        axis_group: None,
        chart_type: None,
        marker_symbol: None,
    }
}

/// True when this series's layoutPr carries a `<cx:binning>` element —
/// the marker for a clusteredColumn that should render as a histogram
/// (auto- or explicit-binned columns over a continuous value axis)
/// rather than as a plain categorical column chart.
fn series_has_binning(series: &cx::Series) -> bool {
    series
        .cx_layout_pr
        .as_deref()
        .and_then(|lp| lp.series_layout_properties_choice.as_ref())
        .is_some_and(|c| matches!(c, cx::SeriesLayoutPropertiesChoice::CxBinning(_)))
}

/// chartEx (cx:) extractor. Surfaces all series with their values +
/// `cx_layout` set to a renderer-friendly tag:
///
///   - `"waterfall"` / `"funnel"` / `"treemap"` / `"sunburst"` /
///     `"regionMap"` — single-series layouts (existing v1 scope).
///   - `"histogram"` — single clusteredColumn series whose layoutPr
///     carries `<cx:binning>`. The renderer auto-bins the raw values.
///   - `"pareto"` — two series: a primary clusteredColumn plus a
///     secondary paretoLine that shares the primary's data (the
///     cumulative-% line is computed at draw time).
///   - `"boxWhisker"` — N parallel boxWhisker series; each carries
///     a column of raw observations. Quartiles / whiskers are
///     computed at draw time per the layoutPr `quartileMethod`.
///
/// Returns `Some(Chart)` with `chart_type = "chartex"`.
pub(super) fn extract_chart_ex(space: &cx::ChartSpace, theme: Option<&Theme>) -> Option<Chart> {
    let chart = space.chart.as_ref();
    let plot_area = chart.plot_area.as_ref();
    let region = plot_area.plot_area_region.as_ref();
    let series_list = &region.cx_series;
    let first = series_list.first()?;

    // Detect the layout family. Most legacy chartEx layouts are
    // single-series and map straight from the primary series's
    // `layoutId`; histogram / pareto / boxWhisker compose multiple
    // series or signal via layoutPr.
    let has_pareto_line = series_list
        .iter()
        .any(|s| matches!(s.layout_id, cx::SeriesLayout::ParetoLine));
    let all_box_whisker = !series_list.is_empty()
        && series_list
            .iter()
            .all(|s| matches!(s.layout_id, cx::SeriesLayout::BoxWhisker));
    let single_histogram = series_list.len() == 1
        && matches!(first.layout_id, cx::SeriesLayout::ClusteredColumn)
        && series_has_binning(first);
    let layout = if has_pareto_line {
        "pareto".to_string()
    } else if all_box_whisker {
        "boxWhisker".to_string()
    } else if single_histogram {
        "histogram".to_string()
    } else {
        cx_layout_name(&first.layout_id).to_string()
    };

    // RegionMap workbooks (Excel's 2-color / 3-color map templates)
    // often carry several `hidden="1"` placeholder series alongside
    // the one visible series — Excel uses the hidden ones as alternate
    // color presets selectable from the chart properties pane. Only
    // the non-hidden series carries the data the user expects to see;
    // pick that one as primary so the renderer doesn't pull empty
    // `_xlchart` aliases.
    let primary_series: &cx::Series = if layout == "regionMap" {
        series_list
            .iter()
            .find(|s| !s.hidden.unwrap_or(false))
            .unwrap_or(first)
    } else {
        first
    };
    // Primary series's parsed data also supplies the chart-level
    // categories + value format (consumed by axis-tick rendering even
    // for multi-series layouts).
    let primary_data = parse_series_data(space, primary_series)?;

    // Build the schema's `series` vector. Pareto + boxWhisker carry
    // multiple series; everything else surfaces just the primary so
    // the existing single-series consumers stay backwards compatible.
    let series: Vec<ChartSeries> = if layout == "boxWhisker" {
        series_list
            .iter()
            .filter_map(|s| {
                let parsed = parse_series_data(space, s)?;
                Some(make_chart_series(
                    parse_series_name(s),
                    parsed.values,
                    parsed.values_ref,
                ))
            })
            .collect()
    } else if layout == "pareto" {
        // Walk in source order so legend / series indexing stays
        // predictable. The paretoLine companion shares the primary's
        // data block (no own `<cx:dataId>`); its values are filled in
        // at render time as a cumulative percentage.
        let mut out: Vec<ChartSeries> = Vec::with_capacity(series_list.len());
        for s in series_list {
            let name = parse_series_name(s);
            match s.layout_id {
                cx::SeriesLayout::ParetoLine => {
                    let display = if name.is_empty() {
                        "Cumulative %".to_string()
                    } else {
                        name
                    };
                    out.push(make_chart_series(display, Vec::new(), None));
                }
                _ => {
                    let parsed = parse_series_data(space, s)?;
                    out.push(make_chart_series(name, parsed.values, parsed.values_ref));
                }
            }
        }
        out
    } else {
        vec![make_chart_series(
            parse_series_name(first),
            primary_data.values.clone(),
            primary_data.values_ref.clone(),
        )]
    };

    // Subtotal indices (`<cx:layoutPr><cx:subtotals><cx:idx val="N"/>`).
    let subtotal_indices: Vec<u32> = first
        .cx_layout_pr
        .as_deref()
        .and_then(|lp| lp.cx_subtotals.as_ref())
        .map(|sub| sub.cx_idx.iter().map(|i| i.val).collect())
        .unwrap_or_default();

    let title = extract_chart_ex_title(chart.chart_title.as_deref());

    // RegionMap-only: parse the visible series's `<cx:valueColors>`
    // 2- or 3-stop color palette. Resolved hex strings flow into the
    // schema's `cx_region_map_{min,mid,max}_color` slots; the renderer
    // builds either a 2-stop (min→max) or 3-stop (min→mid→max)
    // diverging palette from those. The 2-color Map Chart fixture has
    // no `<cx:valueColors>` (Excel defaults the palette); the 3-color
    // Map Chart fixture authors three explicit stops.
    let (rm_min, rm_mid, rm_max) = if layout == "regionMap" {
        extract_region_map_colors(primary_series, theme)
    } else {
        (None, None, None)
    };

    // Legend presence: chartEx legends are uncommon for waterfall;
    // honour the same "absent => no paint" rule used for legacy charts.
    let legend_pos = chart.legend.as_ref().map(|l| {
        match l.pos.as_ref() {
            Some(cx::SidePos::B) => "b",
            Some(cx::SidePos::T) => "t",
            Some(cx::SidePos::L) => "l",
            Some(cx::SidePos::R) => "r",
            None => "r",
        }
        .to_string()
    });

    Some(Chart {
        chart_type: "chartex".to_string(),
        title,
        series,
        categories: primary_data.categories,
        categories_ref: primary_data.categories_ref,
        categories_format: None,
        legend_pos,
        value_format: primary_data.value_format,
        grouping: None,
        bar_dir: None,
        scatter_style: None,
        radar_style: None,
        data_labels: None,
        secondary_axis: false,
        value_format_secondary: None,
        value_min: None,
        value_max: None,
        value_min_secondary: None,
        value_max_secondary: None,
        major_unit: None,
        major_unit_secondary: None,
        bar_gap_width: None,
        bar_overlap: None,
        x_axis_title: None,
        y_axis_title: None,
        y_axis_title_secondary: None,
        show_major_gridlines: None,
        show_major_gridlines_secondary: None,
        disp_units: None,
        disp_units_label: None,
        disp_units_secondary: None,
        disp_units_label_secondary: None,
        bubble_scale: None,
        size_represents: None,
        stock_hi_low_lines: false,
        stock_up_down_bars: false,
        stock_drop_lines: false,
        cx_layout: Some(layout),
        cx_subtotal_indices: subtotal_indices,
        cx_category_levels: Vec::new(),
        cx_waterfall_increment_color: None,
        cx_waterfall_decrement_color: None,
        cx_waterfall_subtotal_color: None,
        cx_region_map_min_color: rm_min,
        cx_region_map_mid_color: rm_mid,
        cx_region_map_max_color: rm_max,
    })
}

/// Parse a chartEx `<cx:valueColors>` block into resolved `#RRGGBB`
/// hex strings. Each of the three slots accepts the same six
/// DrawingML color choices (`scrgbClr` / `srgbClr` / `hslClr` /
/// `sysClr` / `schemeClr` / `prstClr`); ooxmlsdk codegen splits those
/// into three slot-specific enums, so we have three near-identical
/// resolvers below. The two paths exercised by Excel-authored region
/// maps are `<a:srgbClr val="FF0000"/>` (literal hex) and `<a:schemeClr
/// val="accent1"/>` (theme accent); the remaining four DrawingML
/// color choices fall through to `None` and the renderer substitutes
/// its default ramp.
fn extract_region_map_colors(
    series: &cx::Series,
    theme: Option<&Theme>,
) -> (Option<String>, Option<String>, Option<String>) {
    let vc = match series.value_colors.as_deref() {
        Some(v) => v,
        None => return (None, None, None),
    };
    let min = vc
        .min_color_solid_color_fill_properties
        .as_deref()
        .and_then(|p| p.min_color_solid_color_fill_properties_choice.as_ref())
        .and_then(|c| min_color_choice_hex(c, theme));
    let mid = vc
        .mid_color_solid_color_fill_properties
        .as_deref()
        .and_then(|p| p.mid_color_solid_color_fill_properties_choice.as_ref())
        .and_then(|c| mid_color_choice_hex(c, theme));
    let max = vc
        .max_color_solid_color_fill_properties
        .as_deref()
        .and_then(|p| p.max_color_solid_color_fill_properties_choice.as_ref())
        .and_then(|c| max_color_choice_hex(c, theme));
    (min, mid, max)
}

fn min_color_choice_hex(
    c: &cx::MinColorSolidColorFillPropertiesChoice,
    theme: Option<&Theme>,
) -> Option<String> {
    use cx::MinColorSolidColorFillPropertiesChoice as C;
    match c {
        C::ASrgbClr(rgb) => resolve_chartex_srgb(&rgb.val),
        C::ASchemeClr(sc) => resolve_chartex_scheme(&format!("{:?}", sc), theme),
        _ => None,
    }
}

fn mid_color_choice_hex(
    c: &cx::MidColorSolidColorFillPropertiesChoice,
    theme: Option<&Theme>,
) -> Option<String> {
    use cx::MidColorSolidColorFillPropertiesChoice as C;
    match c {
        C::ASrgbClr(rgb) => resolve_chartex_srgb(&rgb.val),
        C::ASchemeClr(sc) => resolve_chartex_scheme(&format!("{:?}", sc), theme),
        _ => None,
    }
}

fn max_color_choice_hex(
    c: &cx::MaxColorSolidColorFillPropertiesChoice,
    theme: Option<&Theme>,
) -> Option<String> {
    use cx::MaxColorSolidColorFillPropertiesChoice as C;
    match c {
        C::ASrgbClr(rgb) => resolve_chartex_srgb(&rgb.val),
        C::ASchemeClr(sc) => resolve_chartex_scheme(&format!("{:?}", sc), theme),
        _ => None,
    }
}

fn resolve_chartex_srgb(val: &str) -> Option<String> {
    if val.len() == 6 && val.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", val))
    } else {
        None
    }
}

/// Resolve a SchemeColor (already Debug-printed) against the workbook
/// theme, then apply any authored modifier chain (`lumMod` / `lumOff`
/// / `shade` / `tint`) via the same path the legacy extractor uses.
fn resolve_chartex_scheme(debug_block: &str, theme: Option<&Theme>) -> Option<String> {
    let base = theme_scheme_color(debug_block, theme)?;
    Some(apply_color_modifiers(&base, debug_block))
}
