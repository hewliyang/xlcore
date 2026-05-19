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

/// English label for a built-in display-unit value. Excel emits these as
/// the default `<c:dispUnitsLbl>` caption text when the workbook authored
/// `<c:dispUnitsLbl>` without an explicit `<c:tx>` child — the label band
/// still renders, but with the localized unit name. We hard-code the
/// en-US strings to match Excel desktop's default UI; full localization
/// would require a per-workbook locale + a translation table, which is
/// out of scope here.
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

/// Extract `<c:dispUnits>` into `(divisor, optional label text)`.
/// Returns `None` when the block is absent or carries no usable choice.
///
/// Label resolution priority (per ECMA-376 §21.2.2.46):
///   1. `<c:dispUnitsLbl><c:tx>...` explicit text — use as-is.
///   2. `<c:dispUnitsLbl>` present *without* `<c:tx>` AND the unit is a
///      built-in — fall back to the localized name of the built-in
///      ("Thousands", "Millions", …). Excel paints this default caption
///      even though the XML carries no text node. Without this fallback
///      we drop the entire "Thousands" caption on charts that scale to
///      thousands via `<c:builtInUnit val="thousands"/>` without an
///      explicit `<c:tx>` (e.g. AGS Metrics Model NWC line chart).
///   3. No `<c:dispUnitsLbl>` element at all — no caption.
pub(crate) fn extract_disp_units(du: Option<&c::DisplayUnits>) -> Option<(f64, Option<String>)> {
    let du = du?;
    let choice = du.display_units_choice.as_ref()?;
    let factor = match choice {
        c::DisplayUnitsChoice::CBuiltInUnit(b) => {
            built_in_unit_factor(b.val.as_ref().unwrap_or(&c::BuiltInUnitValues::Hundreds))
        }
        c::DisplayUnitsChoice::CCustUnit(cu) => cu.val,
    };
    if factor <= 0.0 {
        return None;
    }
    let lbl_present = du.c_disp_units_lbl.is_some();
    let explicit = du
        .c_disp_units_lbl
        .as_deref()
        .and_then(extract_disp_units_lbl_text);
    let label = match explicit {
        Some(s) => Some(s),
        None if lbl_present => match choice {
            c::DisplayUnitsChoice::CBuiltInUnit(b) => Some(
                built_in_unit_default_label(
                    b.val.as_ref().unwrap_or(&c::BuiltInUnitValues::Hundreds),
                )
                .to_string(),
            ),
            // CustUnit with no text — nothing sensible to default to.
            c::DisplayUnitsChoice::CCustUnit(_) => None,
        },
        None => None,
    };
    Some((factor, label))
}

/// Pull the text out of a `<c:dispUnitsLbl>`'s inner `<c:tx>` — same
/// shape as a chart `<c:title>` so we mirror `extract_title`'s body.
pub(crate) fn extract_disp_units_lbl_text(lbl: &c::DisplayUnitsLabel) -> Option<String> {
    let txt = lbl.chart_text.as_ref()?;
    match txt.chart_text_choice.as_ref()? {
        c::ChartTextChoice::CStrRef(sr) => sr.string_cache.as_ref().and_then(|sc| {
            sc.c_pt
                .first()
                .map(|p| p.numeric_value.as_str().to_string())
        }),
        c::ChartTextChoice::CRich(rich) => {
            let mut s = String::new();
            for p in &rich.a_p {
                for ch in &p.paragraph_choice {
                    if let ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::ParagraphChoice::AR(run) = ch {
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
        c::ChartTextChoice::CStrLit(lit) => lit
            .c_pt
            .first()
            .map(|p| p.numeric_value.as_str().to_string()),
    }
}

/// Convert an OOXML `<c:dLbls>` block into our flat `DataLabels` shape.
/// Returns `None` when the block is fully absent or carries `<c:delete
/// val="1"/>` (Excel's "labels suppressed" marker), or when no show*
/// flag is enabled — there's nothing to render in that case.
pub(crate) fn extract_data_labels(dl: Option<&c::DataLabels>) -> Option<DataLabels> {
    let dl = dl?;
    // Per-point overrides first — they may carry the only renderable
    // content even when the parent block has zero show* flags (e.g.
    // pies that label only one slice).
    let point_overrides: Vec<PointDataLabel> = dl
        .c_d_lbl
        .iter()
        .filter_map(extract_point_data_label)
        .collect();
    let seq = match dl.data_labels_choice.as_ref() {
        Some(c::DataLabelsChoice::CDelete(_)) => return None,
        Some(c::DataLabelsChoice::Sequence(s)) => Some(s.as_ref()),
        None => None,
    };
    let Some(seq) = seq else {
        // No parent sequence at all; surface the block only if some
        // point override exists so the renderer has something to paint.
        if point_overrides.is_empty() {
            return None;
        }
        return Some(DataLabels {
            point_overrides,
            ..Default::default()
        });
    };
    // OOXML CT_Boolean: element absent ⇒ false; element present with no
    // val attr ⇒ true (CT_Boolean default, e.g. ECMA-376 §21.2.2.3); element
    // present with val="0"/"false" ⇒ false; val="1"/"true" ⇒ true.
    let show_value = seq
        .show_value
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    let show_category = seq
        .show_category_name
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    let show_series_name = seq
        .show_series_name
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
    let show_percent = seq
        .show_percent
        .as_ref()
        .is_some_and(|b| b.val.unwrap_or(true));
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

/// Map one `<c:dLbl>` (per-data-point override inside `<c:dLbls>`) to
/// our `PointDataLabel`. `<c:idx val="N"/>` is required by the schema
/// but defensively treated as 0 when missing.
pub(crate) fn extract_point_data_label(dl: &c::DataLabel) -> Option<PointDataLabel> {
    let idx: u32 = dl.index.as_ref().map(|i| i.val).unwrap_or(0);
    match dl.data_label_choice.as_ref() {
        Some(c::DataLabelChoice::CDelete(_)) => Some(PointDataLabel {
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
            let show_value = seq.show_value.as_ref().map(|b| b.val.unwrap_or(true));
            let show_category = seq
                .show_category_name
                .as_ref()
                .map(|b| b.val.unwrap_or(true));
            let show_series_name = seq.show_series_name.as_ref().map(|b| b.val.unwrap_or(true));
            let show_percent = seq.show_percent.as_ref().map(|b| b.val.unwrap_or(true));
            // Literal text via `<c:tx>` — same shape as a chart title /
            // dispUnitsLbl, so we mirror `extract_disp_units_lbl_text`'s
            // body inline. Flattens rich-text runs into a plain String.
            let text = seq.chart_text.as_deref().and_then(|txt| {
                match txt.chart_text_choice.as_ref()? {
                    c::ChartTextChoice::CStrRef(sr) => sr.string_cache.as_ref().and_then(
                        |sc| sc.c_pt.first().map(|p| p.numeric_value.as_str().to_string()),
                    ),
                    c::ChartTextChoice::CRich(rich) => {
                        let mut s = String::new();
                        for p in &rich.a_p {
                            for ch in &p.paragraph_choice {
                                if let ooxmlsdk::schemas::schemas_openxmlformats_org_drawingml_2006_main::ParagraphChoice::AR(run) = ch {
                                    s.push_str(run.text.as_str());
                                }
                            }
                        }
                        if s.is_empty() { None } else { Some(s) }
                    }
                    c::ChartTextChoice::CStrLit(lit) => lit
                        .c_pt
                        .first()
                        .map(|p| p.numeric_value.as_str().to_string()),
                }
            });
            // Skip overrides that don't actually override anything
            // (some authors write `<c:dLbl><c:idx/></c:dLbl>` as a
            // no-op) so the wire stays tight.
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
                show_value,
                show_category,
                show_series_name,
                show_percent,
            })
        }
        None => None,
    }
}

/// Common per-series extraction shared by bar/line/area/pie. Reads name,
/// color, and y-values from the standard `c:tx` / `c:spPr` / `c:val` slots.
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
    // Series color resolution order:
    //   1. `<c:spPr><a:solidFill>` (the shape fill — typical for
    //      bar/area/pie series and any bar with an outline+fill).
    //   2. `<c:spPr><a:ln><a:solidFill>` (the *outline* color —
    //      typical for line series authored outline-only, e.g.
    //      `<a:ln><a:solidFill><a:srgbClr val="002060"/></a:ln>`
    //      on chart32.xml's Technology line). Without this fallback
    //      every outline-only series falls through to the theme
    //      accent and renders in the wrong color.
    //   3. Theme accent cycle keyed on series order (matches
    //      Excel's auto-assignment for unstyled series).
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

/// Same as `common_series` but takes the scatter-only `YValues` shape.
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

/// Build a `point_colors` Vec from `<c:dPt>` children. Returns an empty
/// Vec when no data point carries an explicit fill (the common case),
/// so the renderer can cheaply fall back to its per-slice palette /
/// series color. When at least one `<c:dPt>` does have a fill, the
/// returned Vec is sized to `max(values_len, max_dpt_idx + 1)` and
/// indexed by the data point's `c:idx` value; missing entries are
/// empty strings. We can't always trust `values_len` because pie/line
/// series load their numbers from `<c:numRef>` formulas that are only
/// resolved post-sheet-extract — at chart-extract time `values` is
/// commonly empty even though the dPts are present.
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
            // Explicit `<a:noFill/>` at the fill level — transparent.
            // The sentinel "none" is never a valid CSS hex so the
            // renderer can branch on it cleanly.
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

/// Read string/number values out of a CategoryAxisData slot (used by both
/// `c:cat` on bar/line/area/pie and `c:xVal` on scatter).
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
        c::CategoryAxisDataChoice::CStrRef(sr) => (
            string_cache_values(&sr.string_cache),
            Some(sr.formula.as_str().to_string()),
            None,
        ),
        c::CategoryAxisDataChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    nc.c_pt
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
        c::CategoryAxisDataChoice::CStrLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            None,
        ),
        c::CategoryAxisDataChoice::CNumLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            lit.format_code.as_ref().map(|s| s.as_str().to_string()),
        ),
        _ => (Vec::new(), None, None),
    }
}

pub(crate) fn values_format(v: Option<&c::Values>) -> Option<String> {
    let v = v?;
    match v.values_choice.as_ref()? {
        c::ValuesChoice::CNumRef(nr) => nr
            .numbering_cache
            .as_ref()
            .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
        c::ValuesChoice::CNumLit(lit) => lit.format_code.as_ref().map(|s| s.as_str().to_string()),
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
        c::YValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
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
        c::YValuesChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
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

/// Same shape as `y_values_values` for the `<c:bubbleSize>` element
/// (`CT_NumDataSource` again, just a different enum tag).
pub(crate) fn bubble_size_values(v: Option<&c::BubbleSize>) -> (Vec<f64>, Option<String>) {
    let Some(v) = v else {
        return (Vec::new(), None);
    };
    let Some(choice) = v.bubble_size_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::BubbleSizeChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
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
        c::BubbleSizeChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
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
        c::YValuesChoice::CNumRef(nr) => nr
            .numbering_cache
            .as_ref()
            .and_then(|nc| nc.format_code.as_ref().map(|s| s.as_str().to_string())),
        c::YValuesChoice::CNumLit(lit) => lit.format_code.as_ref().map(|s| s.as_str().to_string()),
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
        c::XValuesChoice::CStrRef(sr) => (
            string_cache_values(&sr.string_cache),
            Some(sr.formula.as_str().to_string()),
            None,
        ),
        c::XValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    nc.c_pt
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
        c::XValuesChoice::CStrLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            None,
        ),
        c::XValuesChoice::CNumLit(lit) => (
            lit.c_pt
                .iter()
                .map(|p| p.numeric_value.as_str().to_string())
                .collect(),
            None,
            lit.format_code.as_ref().map(|s| s.as_str().to_string()),
        ),
        _ => (Vec::new(), None, None),
    }
}

/// Numeric x-values for a scatter series. Returns parsed f64s when
/// available, plus the underlying formula ref.
pub(crate) fn scatter_x_values(x: Option<&c::XValues>) -> (Vec<f64>, Option<String>) {
    let Some(x) = x else {
        return (Vec::new(), None);
    };
    let Some(choice) = x.x_values_choice.as_ref() else {
        return (Vec::new(), None);
    };
    match choice {
        c::XValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
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
        c::XValuesChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
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
        c::SeriesTextChoice::CStrRef(sr) => {
            let cached = sr.string_cache.as_ref().and_then(|sc| {
                sc.c_pt
                    .first()
                    .map(|p| p.numeric_value.as_str().to_string())
            });
            (cached, Some(sr.formula.as_str().to_string()))
        }
        c::SeriesTextChoice::CV(v) => (Some(v.as_str().to_string()), None),
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
        c::ValuesChoice::CNumRef(nr) => {
            let vals = nr
                .numbering_cache
                .as_ref()
                .map(|nc| {
                    let mut indexed: Vec<(u32, f64)> = nc
                        .c_pt
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
        c::ValuesChoice::CNumLit(lit) => {
            let mut indexed: Vec<(u32, f64)> = lit
                .c_pt
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
        .c_pt
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
        // English fallback caption used by extract_disp_units when the
        // workbook authored `<c:dispUnitsLbl>` without an explicit
        // `<c:tx>` child. Regression guard for the AGS NWC line chart,
        // where `<c:builtInUnit val="thousands"/>` + empty
        // `<c:dispUnitsLbl>` should still paint "Thousands" on the
        // y-axis.
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
        // Sanity: the factor must match the label's order of magnitude.
        assert_eq!(built_in_unit_factor(&c::BuiltInUnitValues::Thousands), 1e3);
        assert_eq!(built_in_unit_factor(&c::BuiltInUnitValues::Millions), 1e6);
        assert_eq!(built_in_unit_factor(&c::BuiltInUnitValues::Billions), 1e9);
    }
}
