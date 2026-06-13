use crate::chart_colors::*;
use crate::schema::*;
use ooxmlsdk::schemas::schemas_microsoft_com_office_drawing_2014_chartex as cx;

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

fn extract_chart_ex_title(t: Option<&cx::ChartTitle>) -> Option<String> {
    let t = t?;
    let text = t.text.as_deref()?;
    let choice = text.text_choice.as_ref()?;
    match choice {
        cx::TextChoice::TextData(td) => extract_text_data_v(td),
        cx::TextChoice::RichTextBody(rich) => {
            use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main as a;
            let mut out = String::new();
            for p in &rich.paragraph {
                for ch in &p.paragraph_choice {
                    if let a::ParagraphChoice::Run(run) = ch {
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

fn extract_text_data_v(td: &cx::TextData) -> Option<String> {
    match td.text_data_choice.as_ref()? {
        cx::TextDataChoice::VXsdstring(s) => Some(s.clone()),
        cx::TextDataChoice::Sequence { v_xsdstring, .. } => v_xsdstring.clone(),
    }
}

struct ParsedSeriesData {
    categories: Vec<String>,
    categories_ref: Option<String>,
    values: Vec<f64>,
    values_ref: Option<String>,
    value_format: Option<String>,
}

fn parse_series_data(space: &cx::ChartSpace, series: &cx::Series) -> Option<ParsedSeriesData> {
    let data_id = series.data_id.as_ref().map(|d| d.val).unwrap_or(0);
    let data_block = space
        .chart_data
        .as_deref()
        .and_then(|cd| cd.data.iter().find(|d| d.id == data_id))
        .or_else(|| space.chart_data.as_deref().and_then(|cd| cd.data.first()))?;

    let mut categories: Vec<String> = Vec::new();
    let mut categories_ref: Option<String> = None;
    let mut values: Vec<f64> = Vec::new();
    let mut values_ref: Option<String> = None;
    let mut value_format: Option<String> = None;

    for choice in &data_block.data_choice {
        match choice {
            cx::DataChoice::StringDimension(sd) => {
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
                    Some(cx::StringDimensionChoice::StringLevel(lvl)) => vec![lvl.as_ref()],
                    None => Vec::new(),
                };
                if let Some(lvl) = levels.first() {
                    let n = lvl.pt_count as usize;
                    categories = vec![String::new(); n];
                    for pt in &lvl.chart_string_value {
                        let i = pt.index as usize;
                        if i < n {
                            categories[i] = pt.xml_content.clone().unwrap_or_default();
                        }
                    }
                }
            }
            cx::DataChoice::NumericDimension(nd) => {
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
                    Some(cx::NumericDimensionChoice::NumericLevel(lvl)) => vec![lvl.as_ref()],
                    None => Vec::new(),
                };
                if let Some(lvl) = levels.first() {
                    let n = lvl.pt_count as usize;
                    values = vec![0.0; n];
                    for pt in &lvl.numeric_value {
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

fn parse_series_name(series: &cx::Series) -> String {
    series
        .text
        .as_deref()
        .and_then(|t| t.text_choice.as_ref())
        .and_then(|c| match c {
            cx::TextChoice::TextData(td) => extract_text_data_v(td),
            cx::TextChoice::RichTextBody(_) => None,
        })
        .unwrap_or_default()
}

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
        point_fills: Vec::new(),
        point_explosions: Vec::new(),
        data_labels: None,
        axis_group: None,
        chart_type: None,
        marker_symbol: None,
        line_width_emu: None,
        line_dash: None,
        trendlines: Vec::new(),
        error_bars: None,
    }
}

fn series_has_binning(series: &cx::Series) -> bool {
    series
        .series_layout_properties
        .as_deref()
        .and_then(|lp| lp.series_layout_properties_choice.as_ref())
        .is_some_and(|c| matches!(c, cx::SeriesLayoutPropertiesChoice::Binning(_)))
}

pub(super) fn extract_chart_ex(space: &cx::ChartSpace, theme: Option<&Theme>) -> Option<Chart> {
    let chart = space.chart.as_ref();
    let plot_area = chart.plot_area.as_ref();
    let region = plot_area.plot_area_region.as_ref();
    let series_list = &region.series;
    let first = series_list.first()?;

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

    let primary_series: &cx::Series = if layout == "regionMap" {
        series_list
            .iter()
            .find(|s| !s.hidden.map(bool::from).unwrap_or(false))
            .unwrap_or(first)
    } else {
        first
    };

    let primary_data = parse_series_data(space, primary_series)?;

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

    let subtotal_indices: Vec<u32> = first
        .series_layout_properties
        .as_deref()
        .and_then(|lp| lp.subtotals.as_ref())
        .map(|sub| sub.unsigned_integer_type.iter().map(|i| i.val).collect())
        .unwrap_or_default();

    let title = extract_chart_ex_title(chart.chart_title.as_deref());

    let (rm_min, rm_mid, rm_max) = if layout == "regionMap" {
        extract_region_map_colors(primary_series, theme)
    } else {
        (None, None, None)
    };

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
        hole_size: None,
        first_slice_angle: None,
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
        cat_axis_label_rotation: None,
        val_axis_label_rotation: None,
        data_table: None,
        plot_area_fill: None,
        plot_area_border: None,
        legend_fill: None,
        legend_border: None,
        legend_font: None,
    })
}

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
        C::RgbColorModelHex(rgb) => resolve_chartex_srgb(&rgb.val),
        C::SchemeColor(sc) => resolve_chartex_scheme(&format!("{:?}", sc), theme),
        _ => None,
    }
}

fn mid_color_choice_hex(
    c: &cx::MidColorSolidColorFillPropertiesChoice,
    theme: Option<&Theme>,
) -> Option<String> {
    use cx::MidColorSolidColorFillPropertiesChoice as C;
    match c {
        C::RgbColorModelHex(rgb) => resolve_chartex_srgb(&rgb.val),
        C::SchemeColor(sc) => resolve_chartex_scheme(&format!("{:?}", sc), theme),
        _ => None,
    }
}

fn max_color_choice_hex(
    c: &cx::MaxColorSolidColorFillPropertiesChoice,
    theme: Option<&Theme>,
) -> Option<String> {
    use cx::MaxColorSolidColorFillPropertiesChoice as C;
    match c {
        C::RgbColorModelHex(rgb) => resolve_chartex_srgb(&rgb.val),
        C::SchemeColor(sc) => resolve_chartex_scheme(&format!("{:?}", sc), theme),
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

fn resolve_chartex_scheme(debug_block: &str, theme: Option<&Theme>) -> Option<String> {
    let base = theme_scheme_color(debug_block, theme)?;
    Some(apply_color_modifiers(&base, debug_block))
}
