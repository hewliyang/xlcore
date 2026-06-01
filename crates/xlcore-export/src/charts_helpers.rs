use crate::chart_colors::*;
use crate::schema::*;
use ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_chart as c;

pub(crate) fn built_in_unit_factor(b: &c::BuiltInUnitValues) -> f64 {
    match b {
        c::BuiltInUnitValues::Hundreds => 1e2,
        c::BuiltInUnitValues::Thousands => 1e3,
        c::BuiltInUnitValues::TenThousands => 1e4,
        c::BuiltInUnitValues::HundredThousands => 1e5,
        c::BuiltInUnitValues::Millions => 1e6,
        c::BuiltInUnitValues::TenMillions => 1e7,
        c::BuiltInUnitValues::HundredMillions => 1e8,
        c::BuiltInUnitValues::Billions => 1e9,
        c::BuiltInUnitValues::Trillions => 1e12,
    }
}

pub(crate) fn built_in_unit_default_label(b: &c::BuiltInUnitValues) -> &'static str {
    match b {
        c::BuiltInUnitValues::Hundreds => "Hundreds",
        c::BuiltInUnitValues::Thousands => "Thousands",
        c::BuiltInUnitValues::TenThousands => "Ten Thousands",
        c::BuiltInUnitValues::HundredThousands => "Hundred Thousands",
        c::BuiltInUnitValues::Millions => "Millions",
        c::BuiltInUnitValues::TenMillions => "Ten Millions",
        c::BuiltInUnitValues::HundredMillions => "Hundred Millions",
        c::BuiltInUnitValues::Billions => "Billions",
        c::BuiltInUnitValues::Trillions => "Trillions",
    }
}

pub(crate) fn extract_disp_units(du: Option<&c::DisplayUnits>) -> Option<(f64, Option<String>)> {
    let du = du?;
    let choice = du.display_units_choice.as_ref()?;
    let factor = match choice {
        c::DisplayUnitsChoice::BuiltInUnit(b) => {
            built_in_unit_factor(b.val.as_ref().unwrap_or(&c::BuiltInUnitValues::Hundreds))
        }
        c::DisplayUnitsChoice::CustomDisplayUnit(cu) => cu.val,
    };
    if factor <= 0.0 {
        return None;
    }
    let lbl_present = du.display_units_label.is_some();
    let explicit = du
        .display_units_label
        .as_deref()
        .and_then(extract_disp_units_lbl_text);
    let label = match explicit {
        Some(s) => Some(s),
        None if lbl_present => match choice {
            c::DisplayUnitsChoice::BuiltInUnit(b) => Some(
                built_in_unit_default_label(
                    b.val.as_ref().unwrap_or(&c::BuiltInUnitValues::Hundreds),
                )
                .to_string(),
            ),

            c::DisplayUnitsChoice::CustomDisplayUnit(_) => None,
        },
        None => None,
    };
    Some((factor, label))
}

pub(crate) fn extract_disp_units_lbl_text(lbl: &c::DisplayUnitsLabel) -> Option<String> {
    let txt = lbl.chart_text.as_ref()?;
    match txt.chart_text_choice.as_ref()? {
        c::ChartTextChoice::StringReference(sr) => sr.string_cache.as_ref().and_then(|sc| {
            sc.string_point
                .first()
                .map(|p| p.numeric_value.as_str().to_string())
        }),
        c::ChartTextChoice::RichText(rich) => {
            let mut s = String::new();
            for p in &rich.paragraph {
                for ch in &p.paragraph_choice {
                    if let ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::ParagraphChoice::Run(run) = ch {
                        s.push_str(run.text.as_str());
                    }
                }
            }
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        c::ChartTextChoice::StringLiteral(lit) => lit
            .string_point
            .first()
            .map(|p| p.numeric_value.as_str().to_string()),
    }
}

pub(crate) fn extract_data_labels(dl: Option<&c::DataLabels>) -> Option<DataLabels> {
    let dl = dl?;

    let point_overrides: Vec<PointDataLabel> = dl
        .data_label
        .iter()
        .filter_map(extract_point_data_label)
        .collect();
    let seq = match dl.data_labels_choice.as_ref() {
        Some(c::DataLabelsChoice::Delete(_)) => return None,
        Some(c::DataLabelsChoice::Sequence(s)) => Some(s.as_ref()),
        None => None,
    };
    let Some(seq) = seq else {
        if point_overrides.is_empty() {
            return None;
        }
        return Some(DataLabels {
            point_overrides,
            ..Default::default()
        });
    };

    let show_value = seq
        .show_value
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true.into()).into());
    let show_category = seq
        .show_category_name
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true.into()).into());
    let show_series_name = seq
        .show_series_name
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true.into()).into());
    let show_percent = seq
        .show_percent
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true.into()).into());
    if !show_value
        && !show_category
        && !show_series_name
        && !show_percent
        && point_overrides.is_empty()
    {
        return None;
    }
    let position = seq.data_label_position.as_ref().map(|p| {
        match p.val {
            c::DataLabelPositionValues::BestFit => "bestFit",
            c::DataLabelPositionValues::Bottom => "b",
            c::DataLabelPositionValues::Center => "ctr",
            c::DataLabelPositionValues::InsideBase => "inBase",
            c::DataLabelPositionValues::InsideEnd => "inEnd",
            c::DataLabelPositionValues::Left => "l",
            c::DataLabelPositionValues::OutsideEnd => "outEnd",
            c::DataLabelPositionValues::Right => "r",
            c::DataLabelPositionValues::Top => "t",
        }
        .to_string()
    });
    let separator = seq.separator.as_ref().map(|s| s.as_str().to_string());
    let num_fmt = seq
        .numbering_format
        .as_ref()
        .map(|nf| nf.format_code.as_str().to_string());
    Some(DataLabels {
        show_value,
        show_category,
        show_series_name,
        show_percent,
        position,
        separator,
        num_fmt,
        point_overrides,
    })
}

pub(crate) fn extract_point_data_label(dl: &c::DataLabel) -> Option<PointDataLabel> {
    let idx: u32 = dl.index.val;
    match dl.data_label_choice.as_ref() {
        Some(c::DataLabelChoice::Delete(_)) => Some(PointDataLabel {
            idx,
            delete: true,
            ..Default::default()
        }),
        Some(c::DataLabelChoice::Sequence(seq)) => {
            let position = seq.data_label_position.as_ref().map(|p| {
                match p.val {
                    c::DataLabelPositionValues::BestFit => "bestFit",
                    c::DataLabelPositionValues::Bottom => "b",
                    c::DataLabelPositionValues::Center => "ctr",
                    c::DataLabelPositionValues::InsideBase => "inBase",
                    c::DataLabelPositionValues::InsideEnd => "inEnd",
                    c::DataLabelPositionValues::Left => "l",
                    c::DataLabelPositionValues::OutsideEnd => "outEnd",
                    c::DataLabelPositionValues::Right => "r",
                    c::DataLabelPositionValues::Top => "t",
                }
                .to_string()
            });
            let num_fmt = seq
                .numbering_format
                .as_ref()
                .map(|nf| nf.format_code.as_str().to_string());
            let show_value = seq.show_value.as_ref().map(|b| b.val.unwrap_or(true.into()));
            let show_category = seq
                .show_category_name
                .as_ref()
                .map(|b| b.val.unwrap_or(true.into()));
            let show_series_name = seq.show_series_name.as_ref().map(|b| b.val.unwrap_or(true.into()));
            let show_percent = seq.show_percent.as_ref().map(|b| b.val.unwrap_or(true.into()));

            let text = seq.chart_text.as_deref().and_then(|txt| {
                match txt.chart_text_choice.as_ref()? {
                    c::ChartTextChoice::StringReference(sr) => sr.string_cache.as_ref().and_then(
                        |sc| sc.string_point.first().map(|p| p.numeric_value.as_str().to_string()),
                    ),
                    c::ChartTextChoice::RichText(rich) => {
                        let mut s = String::new();
                        for p in &rich.paragraph {
                            for ch in &p.paragraph_choice {
                                if let ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::ParagraphChoice::Run(run) = ch {
                                    s.push_str(run.text.as_str());
                                }
                            }
                        }
                        if s.is_empty() { None } else { Some(s) }
                    }
                    c::ChartTextChoice::StringLiteral(lit) => lit
                        .string_point
                        .first()
                        .map(|p| p.numeric_value.as_str().to_string()),
                }
            });

            if position.is_none()
                && num_fmt.is_none()
                && show_value.is_none()
                && show_category.is_none()
                && show_series_name.is_none()
                && show_percent.is_none()
                && text.is_none()
            {
                return None;
            }
            Some(PointDataLabel {
                idx,
                delete: false,
                text,
                position,
                num_fmt,
                show_value: show_value.map(bool::from),
                show_category: show_category.map(bool::from),
                show_series_name: show_series_name.map(bool::from),
                show_percent: show_percent.map(bool::from),
            })
        }
        None => None,
    }
}

pub(crate) fn common_series(
    order: &c::Order,
    tx: Option<&c::SeriesText>,
    sp_pr: Option<&c::ChartShapeProperties>,
    val: Option<&c::Values>,
    d_pts: &[c::DataPoint],
    theme: Option<&Theme>,
) -> ChartSeries {
    let (name, name_ref) = series_text_or_ref(tx);
    let (values, values_ref) = number_reference_values(val);

    let color = series_color_via_debug(sp_pr, theme)
        .or_else(|| line_color_via_debug(sp_pr, theme))
        .or_else(|| {
            let n = order.val % 6 + 1;
            Some(theme_accent_color(n, theme))
        });
    let point_colors = extract_point_colors(d_pts, values.len(), theme);
    ChartSeries {
        name: name.unwrap_or_default(),
        name_ref,
        color,
        values,
        values_ref,
        x_values: Vec::new(),
        x_values_ref: None,
        bubble_sizes: Vec::new(),
        bubble_sizes_ref: None,
        point_colors,
        data_labels: None,
        axis_group: None,
        chart_type: None,
        marker_symbol: None,
    }
}

pub(crate) fn common_series_scatter(
    order: &c::Order,
    tx: Option<&c::SeriesText>,
    sp_pr: Option<&c::ChartShapeProperties>,
    y_val: Option<&c::YValues>,
    d_pts: &[c::DataPoint],
    theme: Option<&Theme>,
) -> ChartSeries {
    let (name, name_ref) = series_text_or_ref(tx);
    let (values, values_ref) = y_values_values(y_val);
    let color = series_color_via_debug(sp_pr, theme)
        .or_else(|| line_color_via_debug(sp_pr, theme))
        .or_else(|| {
            let n = order.val % 6 + 1;
            Some(theme_accent_color(n, theme))
        });
    let point_colors = extract_point_colors(d_pts, values.len(), theme);
    ChartSeries {
        name: name.unwrap_or_default(),
        name_ref,
        color,
        values,
        values_ref,
        x_values: Vec::new(),
        x_values_ref: None,
        bubble_sizes: Vec::new(),
        bubble_sizes_ref: None,
        point_colors,
        data_labels: None,
        axis_group: None,
        chart_type: None,
        marker_symbol: None,
    }
}

pub(crate) fn extract_point_colors(
    d_pts: &[c::DataPoint],
    values_len: usize,
    theme: Option<&Theme>,
) -> Vec<String> {
    if d_pts.is_empty() {
        return Vec::new();
    }
    let max_idx = d_pts
        .iter()
        .map(|dp| dp.index.val as usize)
        .max()
        .unwrap_or(0);
    let len = values_len.max(max_idx + 1);
    let mut out = vec![String::new(); len];
    let mut any = false;
    for dp in d_pts {
        let idx = dp.index.val as usize;
        if idx >= len {
            continue;
        }
        let Some(sp) = dp.chart_shape_properties.as_deref() else {
            continue;
        };
        if let Some(c) = series_color_via_debug(Some(sp), theme) {
            out[idx] = c;
            any = true;
        } else if shape_has_no_fill(sp) {
            out[idx] = "none".to_string();
            any = true;
        }
    }
    if any {
        out
    } else {
        Vec::new()
    }
}

pub(crate) fn ax_data_values(
    cat: Option<&c::CategoryAxisData>,
) -> (Vec<String>, Option<String>, Option<String>) {
    let Some(cat) = cat else {
        return (Vec::new(), None, None);
    };
    let Some(choice) = cat.category_axis_data_choice.as_ref() else {
        return (Vec::new(), None, None);
    };
    match choice {
        c::CategoryAxisDataChoice::StringReference(sr) => (
            string_cache_values(&sr.string_cache),
            Some(sr.formula.as_str().to_string()),
            None,
        ),
        c::CategoryAxisDataChoice::MultiLevelStringReference(mlsr) => {
            (Vec::new(), Some(mlsr.formula.as_str().to_string()), None)
        }
        c::CategoryAxisDataChoice::NumberReference(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    nc.numeric_point
                        .iter()
                        .map(|p| p.numeric_value.as_str().to_string())
                        .collect()
                })
                .unwrap_or_default();
            (
                vals,
                Some(nr.formula.as_str().to_string()),
                nr.numbering_cache
                    .as_ref()
                    .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
            )
        }
        c::CategoryAxisDataChoice::StringLiteral(lit) => (
            lit.string_point
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            None,
        ),
        c::CategoryAxisDataChoice::NumberLiteral(lit) => (
            lit.numeric_point
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            lit.format_code.as_ref().map(|s| s.as_str().to_string()),
        ),
    }
}

pub(crate) fn values_format(v: Option<&c::Values>) -> Option<String> {
    let v = v?;
    match v.values_choice.as_ref()? {
        c::ValuesChoice::NumberReference(nr) => nr
            .numbering_cache
            .as_ref()
            .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
        c::ValuesChoice::NumberLiteral(lit) => lit.format_code.as_ref().map(|s| s.as_str().to_string()),
    }
}

pub(crate) fn y_values_values(v: Option<&c::YValues>) -> (Vec<f64>, Option<String>) {
    let Some(v) = v else {
        return (Vec::new(), None);
    };
    let Some(choice) = v.y_values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::YValuesChoice::NumberReference(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .numeric_point
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::YValuesChoice::NumberLiteral(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .numeric_point
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
    }
}

pub(crate) fn bubble_size_values(v: Option<&c::BubbleSize>) -> (Vec<f64>, Option<String>) {
    let Some(v) = v else {
        return (Vec::new(), None);
    };
    let Some(choice) = v.bubble_size_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::BubbleSizeChoice::NumberReference(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .numeric_point
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::BubbleSizeChoice::NumberLiteral(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .numeric_point
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
    }
}

pub(crate) fn y_values_format(v: Option<&c::YValues>) -> Option<String> {
    let v = v?;
    match v.y_values_choice.as_ref()? {
        c::YValuesChoice::NumberReference(nr) => nr
            .numbering_cache
            .as_ref()
            .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
        c::YValuesChoice::NumberLiteral(lit) => lit.format_code.as_ref().map(|s| s.as_str().to_string()),
    }
}

pub(crate) fn x_axis_values(
    x: Option<&c::XValues>,
) -> (Vec<String>, Option<String>, Option<String>) {
    let Some(x) = x else {
        return (Vec::new(), None, None);
    };
    let Some(choice) = x.x_values_choice.as_ref() else {
        return (Vec::new(), None, None);
    };
    match choice {
        c::XValuesChoice::StringReference(sr) => (
            string_cache_values(&sr.string_cache),
            Some(sr.formula.as_str().to_string()),
            None,
        ),
        c::XValuesChoice::NumberReference(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    nc.numeric_point
                        .iter()
                        .map(|p| p.numeric_value.as_str().to_string())
                        .collect()
                })
                .unwrap_or_default();
            (
                vals,
                Some(nr.formula.as_str().to_string()),
                nr.numbering_cache
                    .as_ref()
                    .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
            )
        }
        c::XValuesChoice::StringLiteral(lit) => (
            lit.string_point
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            None,
        ),
        c::XValuesChoice::NumberLiteral(lit) => (
            lit.numeric_point
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            lit.format_code.as_ref().map(|s| s.as_str().to_string()),
        ),
        _ => (Vec::new(), None, None),
    }
}

pub(crate) fn scatter_x_values(x: Option<&c::XValues>) -> (Vec<f64>, Option<String>) {
    let Some(x) = x else {
        return (Vec::new(), None);
    };
    let Some(choice) = x.x_values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::XValuesChoice::NumberReference(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .numeric_point
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::XValuesChoice::NumberLiteral(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .numeric_point
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
        _ => (Vec::new(), None),
    }
}

pub(crate) fn series_text_or_ref(t: Option<&c::SeriesText>) -> (Option<String>, Option<String>) {
    let Some(t) = t else {
        return (None, None);
    };
    let Some(choice) = t.series_text_choice.as_ref() else {
        return (None, None);
    };
    match choice {
        c::SeriesTextChoice::StringReference(sr) => {
            let cached = sr.string_cache.as_ref().and_then(|sc| {
                sc.string_point
                    .first()
                    .map(|p| p.numeric_value.as_str().to_string())
            });
            (cached, Some(sr.formula.as_str().to_string()))
        }
        c::SeriesTextChoice::NumericValue(v) => (Some(v.as_str().to_string()), None),
    }
}

pub(crate) fn number_reference_values(v: Option<&c::Values>) -> (Vec<f64>, Option<String>) {
    let Some(v) = v else {
        return (Vec::new(), None);
    };
    let Some(choice) = v.values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::ValuesChoice::NumberReference(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .numeric_point
                        .iter()
                        .filter_map(|p| {
                            p.numeric_value
                                .as_str()
                                .parse::<f64>()
                                .ok()
                                .map(|v| (p.index, v))
                        })
                        .collect();
                    indexed.sort_by_key(|(i, _)| *i);
                    indexed.into_iter().map(|(_, v)| v).collect()
                })
                .unwrap_or_default();
            (vals, Some(nr.formula.as_str().to_string()))
        }
        c::ValuesChoice::NumberLiteral(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .numeric_point
                .iter()
                .filter_map(|p| {
                    p.numeric_value
                        .as_str()
                        .parse::<f64>()
                        .ok()
                        .map(|v| (p.index, v))
                })
                .collect();
            indexed.sort_by_key(|(i, _)| *i);
            (indexed.into_iter().map(|(_, v)| v).collect(), None)
        }
    }
}

pub(crate) fn string_cache_values(sc: &Option<Box<c::StringCache>>) -> Vec<String> {
    let Some(sc) = sc else {
        return Vec::new();
    };
    let mut indexed: Vec<(u32, String)> = sc
        .string_point
        .iter()
        .map(|p| (p.index, p.numeric_value.as_str().to_string()))
        .collect();
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, v)| v).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_unit_labels_match_excel_ui() {
        assert_eq!(
            built_in_unit_default_label(&c::BuiltInUnitValues::Thousands),
            "Thousands",
        );
        assert_eq!(
            built_in_unit_default_label(&c::BuiltInUnitValues::Millions),
            "Millions",
        );
        assert_eq!(
            built_in_unit_default_label(&c::BuiltInUnitValues::TenThousands),
            "Ten Thousands",
        );
        assert_eq!(
            built_in_unit_default_label(&c::BuiltInUnitValues::Billions),
            "Billions",
        );
    }

    #[test]
    fn built_in_unit_factors_powers_of_ten() {
        assert_eq!(built_in_unit_factor(&c::BuiltInUnitValues::Thousands), 1e3);
        assert_eq!(built_in_unit_factor(&c::BuiltInUnitValues::Millions), 1e6);
        assert_eq!(built_in_unit_factor(&c::BuiltInUnitValues::Billions), 1e9);
    }
}
