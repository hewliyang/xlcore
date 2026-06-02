use crate::schema::{Cell, CellFormat, Color, Fill, Font, Styles};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use std::cmp::Ordering;
use xlcore_io::parse_range;

#[derive(Clone, Copy)]
pub struct PivotStyleIndices {
    header: u32,
    total_label: u32,
    total_value: u32,
}

impl PivotStyleIndices {
    pub fn header(&self) -> u32 {
        self.header
    }
    pub fn total_label(&self) -> u32 {
        self.total_label
    }
    pub fn total_value(&self) -> u32 {
        self.total_value
    }
}

fn register_styles(styles: &mut Styles) -> PivotStyleIndices {
    if styles.fonts.is_empty() {
        styles.fonts.push(Font {
            name: Some(styles.default_font.clone()),
            size: Some(styles.default_font_size),
            ..Default::default()
        });
    }
    if styles.fills.is_empty() {
        styles.fills.push(Fill {
            pattern_type: Some("none".to_string()),
            ..Default::default()
        });
    }
    if styles.cell_xfs.is_empty() {
        styles.cell_xfs.push(CellFormat::default());
    }
    let bold_white = Font {
        name: Some(styles.default_font.clone()),
        size: Some(styles.default_font_size),
        bold: true,
        color: Some(Color {
            rgb: Some("FFFFFFFF".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let bold_dark = Font {
        name: Some(styles.default_font.clone()),
        size: Some(styles.default_font_size),
        bold: true,
        ..Default::default()
    };
    let white_font = styles.fonts.len() as u32;
    styles.fonts.push(bold_white);
    let dark_font = styles.fonts.len() as u32;
    styles.fonts.push(bold_dark);

    let header_fill = styles.fills.len() as u32;
    styles.fills.push(Fill {
        pattern_type: Some("solid".to_string()),
        fg_color: Some(Color {
            rgb: Some("FF4472C4".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    });

    let header = styles.cell_xfs.len() as u32;
    styles.cell_xfs.push(CellFormat {
        font_id: Some(white_font),
        fill_id: Some(header_fill),
        ..Default::default()
    });
    let total_label = styles.cell_xfs.len() as u32;
    styles.cell_xfs.push(CellFormat {
        font_id: Some(dark_font),
        ..Default::default()
    });
    let total_value = styles.cell_xfs.len() as u32;
    styles.cell_xfs.push(CellFormat {
        font_id: Some(dark_font),
        horizontal_alignment: Some("right".to_string()),
        ..Default::default()
    });

    PivotStyleIndices {
        header,
        total_label,
        total_value,
    }
}

#[derive(Clone, PartialEq)]
enum PVal {
    Num(f64),
    Text(String),
    Bool(bool),
    Blank,
}

impl PVal {
    fn label(&self) -> String {
        match self {
            PVal::Num(n) => fmt_num(*n),
            PVal::Text(s) => s.clone(),
            PVal::Bool(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
            PVal::Blank => String::new(),
        }
    }

    fn as_num(&self) -> Option<f64> {
        match self {
            PVal::Num(n) => Some(*n),
            PVal::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn is_blank(&self) -> bool {
        matches!(self, PVal::Blank)
    }
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn cmp_pval(a: &PVal, b: &PVal) -> Ordering {
    fn rank(v: &PVal) -> u8 {
        match v {
            PVal::Num(_) | PVal::Bool(_) => 0,
            PVal::Text(_) => 1,
            PVal::Blank => 2,
        }
    }
    match rank(a).cmp(&rank(b)) {
        Ordering::Equal => match (a.as_num(), b.as_num()) {
            (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
            _ => a.label().to_lowercase().cmp(&b.label().to_lowercase()),
        },
        other => other,
    }
}

fn unique_sorted(tuples: Vec<Vec<PVal>>) -> Vec<Vec<PVal>> {
    let mut out: Vec<Vec<PVal>> = Vec::new();
    for t in tuples {
        if !out.iter().any(|e| e == &t) {
            out.push(t);
        }
    }
    out.sort_by(|a, b| {
        for (x, y) in a.iter().zip(b) {
            match cmp_pval(x, y) {
                Ordering::Equal => continue,
                other => return other,
            }
        }
        Ordering::Equal
    });
    out
}

fn shared_item_value(choice: &x::SharedItemsChoice) -> PVal {
    match choice {
        x::SharedItemsChoice::NumberItem(b) => PVal::Num(b.val),
        x::SharedItemsChoice::StringItem(b) => PVal::Text(b.val.clone()),
        x::SharedItemsChoice::BooleanItem(b) => PVal::Bool(b.val.into()),
        x::SharedItemsChoice::DateTimeItem(_) | x::SharedItemsChoice::ErrorItem(_) => PVal::Blank,
        x::SharedItemsChoice::MissingItem(_) => PVal::Blank,
    }
}

fn record_value(choice: &x::PivotCacheRecordChoice, items: &[PVal]) -> PVal {
    match choice {
        x::PivotCacheRecordChoice::FieldItem(b) => {
            items.get(b.val as usize).cloned().unwrap_or(PVal::Blank)
        }
        x::PivotCacheRecordChoice::NumberItem(b) => PVal::Num(b.val),
        x::PivotCacheRecordChoice::StringItem(b) => PVal::Text(b.val.clone()),
        x::PivotCacheRecordChoice::BooleanItem(b) => PVal::Bool(b.val.into()),
        _ => PVal::Blank,
    }
}

fn aggregate(vals: &[PVal], func: Option<&x::DataConsolidateFunctionValues>) -> Option<f64> {
    use x::DataConsolidateFunctionValues as F;
    let nums: Vec<f64> = vals.iter().filter_map(|v| v.as_num()).collect();
    let non_blank = vals.iter().filter(|v| !v.is_blank()).count();
    match func.unwrap_or(&F::Sum) {
        F::Count => Some(non_blank as f64),
        F::CountNumbers => Some(nums.len() as f64),
        F::Average => {
            if nums.is_empty() {
                None
            } else {
                Some(nums.iter().sum::<f64>() / nums.len() as f64)
            }
        }
        F::Maximum => nums.iter().cloned().reduce(f64::max),
        F::Minimum => nums.iter().cloned().reduce(f64::min),
        F::Product => Some(nums.iter().product()),
        F::StandardDeviation => std_dev(&nums, true),
        F::StandardDeviationP => std_dev(&nums, false),
        F::Variance => variance(&nums, true),
        F::VarianceP => variance(&nums, false),
        F::Sum => Some(nums.iter().sum()),
    }
}

fn variance(nums: &[f64], sample: bool) -> Option<f64> {
    let n = nums.len();
    let denom = if sample { n.checked_sub(1)? } else { n };
    if denom == 0 {
        return None;
    }
    let mean = nums.iter().sum::<f64>() / n as f64;
    let ss: f64 = nums.iter().map(|v| (v - mean).powi(2)).sum();
    Some(ss / denom as f64)
}

fn std_dev(nums: &[f64], sample: bool) -> Option<f64> {
    variance(nums, sample).map(f64::sqrt)
}

fn text_cell(r: u32, c: u32, text: String) -> Cell {
    Cell {
        r,
        c,
        kind: "str".to_string(),
        value: Some(text),
        formula: None,
        style_index: None,
        runs: Vec::new(),
    }
}

fn num_cell(r: u32, c: u32, n: f64) -> Cell {
    Cell {
        r,
        c,
        kind: "n".to_string(),
        value: Some(fmt_num(n)),
        formula: None,
        style_index: None,
        runs: Vec::new(),
    }
}

fn styled(mut c: Cell, idx: u32) -> Cell {
    c.style_index = Some(idx);
    c
}

fn opt_styled(mut c: Cell, idx: Option<u32>) -> Cell {
    c.style_index = idx;
    c
}

fn intern_xf(styles: &mut Styles, cf: CellFormat) -> u32 {
    if let Some(i) = styles.cell_xfs.iter().position(|e| *e == cf) {
        i as u32
    } else {
        let i = styles.cell_xfs.len() as u32;
        styles.cell_xfs.push(cf);
        i
    }
}

pub fn compute_cells(
    pt: &x::PivotTableDefinition,
    cache_def: &x::PivotCacheDefinition,
    records: &x::PivotCacheRecords,
    styles: &mut Styles,
    style_memo: &mut Option<PivotStyleIndices>,
) -> Vec<Cell> {
    let field_names: Vec<String> = cache_def
        .cache_fields
        .cache_field
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let field_items: Vec<Vec<PVal>> = cache_def
        .cache_fields
        .cache_field
        .iter()
        .map(|f| {
            f.shared_items
                .as_ref()
                .map(|si| {
                    si.shared_items_choice
                        .iter()
                        .map(shared_item_value)
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let hidden_items: Vec<std::collections::HashSet<u32>> = pt
        .pivot_fields
        .as_ref()
        .map(|pf| {
            pf.pivot_field
                .iter()
                .map(|f| {
                    f.items
                        .as_ref()
                        .map(|items| {
                            items
                                .item
                                .iter()
                                .filter(|it| it.hidden.map(Into::into).unwrap_or(false))
                                .filter_map(|it| it.index.map(Into::into))
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();

    let page_filters: std::collections::HashMap<usize, u32> = pt
        .page_fields
        .as_ref()
        .map(|pf| {
            pf.page_field
                .iter()
                .filter_map(|f| {
                    let fld = usize::try_from(f.field).ok()?;
                    let item: u32 = f.item.map(Into::into)?;
                    let shared = pt
                        .pivot_fields
                        .as_ref()?
                        .pivot_field
                        .get(fld)?
                        .items
                        .as_ref()?
                        .item
                        .get(item as usize)?
                        .index
                        .map(Into::into)?;
                    Some((fld, shared))
                })
                .collect()
        })
        .unwrap_or_default();

    let record_hidden = |rec: &x::PivotCacheRecord| -> bool {
        rec.pivot_cache_record_choice
            .iter()
            .enumerate()
            .any(|(i, choice)| {
                if let x::PivotCacheRecordChoice::FieldItem(b) = choice {
                    let is_hidden = hidden_items
                        .get(i)
                        .map(|h| h.contains(&b.val))
                        .unwrap_or(false);
                    let page_excluded =
                        page_filters.get(&i).map(|want| *want != b.val).unwrap_or(false);
                    is_hidden || page_excluded
                } else {
                    false
                }
            })
    };

    let decoded: Vec<Vec<PVal>> = records
        .pivot_cache_record
        .iter()
        .filter(|rec| !record_hidden(rec))
        .map(|rec| {
            rec.pivot_cache_record_choice
                .iter()
                .enumerate()
                .map(|(i, choice)| {
                    record_value(
                        choice,
                        field_items.get(i).map(|v| v.as_slice()).unwrap_or(&[]),
                    )
                })
                .collect()
        })
        .collect();

    let row_fields: Vec<usize> = pt
        .row_fields
        .as_ref()
        .map(|rf| {
            rf.field
                .iter()
                .map(|f| f.index)
                .filter(|i| *i >= 0)
                .map(|i| i as usize)
                .collect()
        })
        .unwrap_or_default();
    let col_fields: Vec<usize> = pt
        .column_fields
        .as_ref()
        .map(|cf| {
            cf.field
                .iter()
                .map(|f| f.index)
                .filter(|i| *i >= 0)
                .map(|i| i as usize)
                .collect()
        })
        .unwrap_or_default();
    let data_fields: Vec<(usize, Option<x::DataConsolidateFunctionValues>, String, Option<u32>)> =
        pt.data_fields
            .as_ref()
            .map(|df| {
                df.data_field
                    .iter()
                    .map(|d| {
                        let fld = d.field as usize;
                        let name = d
                            .name
                            .clone()
                            .unwrap_or_else(|| field_names.get(fld).cloned().unwrap_or_default());
                        (fld, d.subtotal.clone(), name, d.number_format_id)
                    })
                    .collect()
            })
            .unwrap_or_default();

    if row_fields.is_empty()
        || data_fields.is_empty()
        || col_fields.len() > 2
        || (col_fields.len() == 2 && data_fields.len() > 1)
    {
        return Vec::new();
    }

    let Some(((r1, c1), _)) = parse_range(pt.location.reference.as_str()) else {
        return Vec::new();
    };

    let st = *style_memo.get_or_insert_with(|| register_styles(styles));

    let field_xfs: Vec<(Option<u32>, u32)> = data_fields
        .iter()
        .map(|(_, _, _, num_fmt)| match num_fmt {
            Some(id) => {
                let val = intern_xf(
                    styles,
                    CellFormat {
                        num_fmt_id: Some(*id),
                        horizontal_alignment: Some("right".to_string()),
                        ..Default::default()
                    },
                );
                let mut tcf = styles.cell_xfs[st.total_value as usize].clone();
                tcf.num_fmt_id = Some(*id);
                let tot = intern_xf(styles, tcf);
                (Some(val), tot)
            }
            None => (None, st.total_value),
        })
        .collect();

    let tuple = |fields: &[usize], rec: &[PVal]| -> Vec<PVal> {
        fields
            .iter()
            .map(|&f| rec.get(f).cloned().unwrap_or(PVal::Blank))
            .collect()
    };
    let to_labels = |t: &[PVal]| -> Vec<String> { t.iter().map(PVal::label).collect() };

    let row_keys: Vec<Vec<String>> =
        unique_sorted(decoded.iter().map(|rec| tuple(&row_fields, rec)).collect())
            .iter()
            .map(|t| to_labels(t))
            .collect();
    let col_keys: Vec<Vec<String>> = if col_fields.is_empty() {
        Vec::new()
    } else {
        unique_sorted(decoded.iter().map(|rec| tuple(&col_fields, rec)).collect())
            .iter()
            .map(|t| to_labels(t))
            .collect()
    };

    let matches = |fields: &[usize], key: &[String], rec: &[PVal]| -> bool {
        fields
            .iter()
            .zip(key)
            .all(|(&f, want)| rec.get(f).cloned().unwrap_or(PVal::Blank).label() == *want)
    };
    let value_for = |row_key: &[String],
                     col_key: Option<&[String]>,
                     fld: usize,
                     func: Option<&x::DataConsolidateFunctionValues>|
     -> Option<f64> {
        let vals: Vec<PVal> = decoded
            .iter()
            .filter(|rec| matches(&row_fields, row_key, rec))
            .filter(|rec| {
                col_key
                    .map(|ck| matches(&col_fields, ck, rec))
                    .unwrap_or(true)
            })
            .map(|rec| rec.get(fld).cloned().unwrap_or(PVal::Blank))
            .collect();
        aggregate(&vals, func)
    };

    let r = row_fields.len() as u32;
    let mut cells = Vec::new();

    if col_fields.is_empty() {
        let row_names: Vec<String> = row_fields.iter().map(|&f| field_names[f].clone()).collect();
        for (j, name) in row_names.iter().enumerate() {
            cells.push(styled(text_cell(r1, c1 + j as u32, name.clone()), st.header));
        }
        for (d, (_, _, name, _)) in data_fields.iter().enumerate() {
            cells.push(styled(
                text_cell(r1, c1 + r + d as u32, name.clone()),
                st.header,
            ));
        }
        for (i, rk) in row_keys.iter().enumerate() {
            let rr = r1 + 1 + i as u32;
            for (j, lbl) in rk.iter().enumerate() {
                cells.push(text_cell(rr, c1 + j as u32, lbl.clone()));
            }
            for (d, (fld, func, _, _)) in data_fields.iter().enumerate() {
                if let Some(v) = value_for(rk, None, *fld, func.as_ref()) {
                    cells.push(opt_styled(num_cell(rr, c1 + r + d as u32, v), field_xfs[d].0));
                }
            }
        }
        let gr = r1 + 1 + row_keys.len() as u32;
        cells.push(styled(text_cell(gr, c1, "Grand Total".to_string()), st.total_label));
        for (d, (fld, func, _, _)) in data_fields.iter().enumerate() {
            if let Some(v) = value_for_all(&decoded, *fld, func.as_ref()) {
                cells.push(styled(num_cell(gr, c1 + r + d as u32, v), field_xfs[d].1));
            }
        }
        return cells;
    }

    if col_fields.len() == 2 {
        let (data_fld, ref data_func, ref data_name, _) = data_fields[0];
        let (value_xf, total_xf) = field_xfs[0];
        let outer_name = field_names[col_fields[0]].clone();
        let inner_name = field_names[col_fields[1]].clone();
        let row_names: Vec<String> =
            row_fields.iter().map(|&f| field_names[f].clone()).collect();
        let base = c1 + r;

        enum Slot {
            Leaf(String, String),
            Sub(String),
            Grand,
        }
        let mut slots: Vec<Slot> = Vec::new();
        let mut i = 0;
        while i < col_keys.len() {
            let outer = col_keys[i][0].clone();
            let mut j = i;
            while j < col_keys.len() && col_keys[j][0] == outer {
                slots.push(Slot::Leaf(outer.clone(), col_keys[j][1].clone()));
                j += 1;
            }
            slots.push(Slot::Sub(outer.clone()));
            i = j;
        }
        slots.push(Slot::Grand);

        let value_cols = |row_key: Option<&[String]>, cflds: &[usize], clabels: &[String]| {
            let vals: Vec<PVal> = decoded
                .iter()
                .filter(|rec| row_key.map(|rk| matches(&row_fields, rk, rec)).unwrap_or(true))
                .filter(|rec| matches(cflds, clabels, rec))
                .map(|rec| rec.get(data_fld).cloned().unwrap_or(PVal::Blank))
                .collect();
            aggregate(&vals, data_func.as_ref())
        };

        cells.push(styled(text_cell(r1, c1, data_name.clone()), st.header));
        cells.push(styled(text_cell(r1, base, outer_name), st.header));
        cells.push(styled(text_cell(r1, base + 1, inner_name), st.header));
        for (jx, name) in row_names.iter().enumerate() {
            cells.push(styled(text_cell(r1 + 2, c1 + jx as u32, name.clone()), st.header));
        }

        let mut prev_outer: Option<String> = None;
        for (sidx, slot) in slots.iter().enumerate() {
            let col = base + sidx as u32;
            match slot {
                Slot::Leaf(outer, inner) => {
                    let outer_lbl = if prev_outer.as_deref() != Some(outer.as_str()) {
                        prev_outer = Some(outer.clone());
                        outer.clone()
                    } else {
                        String::new()
                    };
                    cells.push(styled(text_cell(r1 + 1, col, outer_lbl), st.header));
                    cells.push(styled(text_cell(r1 + 2, col, inner.clone()), st.header));
                }
                Slot::Sub(outer) => {
                    cells.push(styled(text_cell(r1 + 1, col, format!("{outer} Total")), st.header));
                    cells.push(styled(text_cell(r1 + 2, col, String::new()), st.header));
                }
                Slot::Grand => {
                    cells.push(styled(text_cell(r1 + 1, col, "Grand Total".to_string()), st.header));
                    cells.push(styled(text_cell(r1 + 2, col, String::new()), st.header));
                }
            }
        }

        for (ri, rk) in row_keys.iter().enumerate() {
            let rr = r1 + 3 + ri as u32;
            for (jx, lbl) in rk.iter().enumerate() {
                cells.push(text_cell(rr, c1 + jx as u32, lbl.clone()));
            }
            for (sidx, slot) in slots.iter().enumerate() {
                let col = base + sidx as u32;
                let (v, xf) = match slot {
                    Slot::Leaf(o, inn) => (
                        value_cols(Some(rk), &col_fields, &[o.clone(), inn.clone()]),
                        value_xf,
                    ),
                    Slot::Sub(o) => {
                        (value_cols(Some(rk), &col_fields[..1], &[o.clone()]), Some(total_xf))
                    }
                    Slot::Grand => (value_cols(Some(rk), &[], &[]), Some(total_xf)),
                };
                if let Some(v) = v {
                    cells.push(opt_styled(num_cell(rr, col, v), xf));
                }
            }
        }

        let gr = r1 + 3 + row_keys.len() as u32;
        cells.push(styled(text_cell(gr, c1, "Grand Total".to_string()), st.total_label));
        for (sidx, slot) in slots.iter().enumerate() {
            let col = base + sidx as u32;
            let v = match slot {
                Slot::Leaf(o, inn) => value_cols(None, &col_fields, &[o.clone(), inn.clone()]),
                Slot::Sub(o) => value_cols(None, &col_fields[..1], &[o.clone()]),
                Slot::Grand => value_cols(None, &[], &[]),
            };
            if let Some(v) = v {
                cells.push(styled(num_cell(gr, col, v), total_xf));
            }
        }
        return cells;
    }

    if data_fields.len() > 1 {
        let col_name = field_names[col_fields[0]].clone();
        let row_names: Vec<String> =
            row_fields.iter().map(|&f| field_names[f].clone()).collect();
        let dcount = data_fields.len() as u32;
        let base = c1 + r;
        let m = col_keys.len() as u32;

        cells.push(styled(text_cell(r1, base, col_name), st.header));
        for (j, name) in row_names.iter().enumerate() {
            cells.push(styled(
                text_cell(r1 + 2, c1 + j as u32, name.clone()),
                st.header,
            ));
        }
        for (k, ck) in col_keys.iter().enumerate() {
            let gc = base + k as u32 * dcount;
            cells.push(styled(text_cell(r1 + 1, gc, ck[0].clone()), st.header));
            for (d, (_, _, name, _)) in data_fields.iter().enumerate() {
                cells.push(styled(
                    text_cell(r1 + 2, gc + d as u32, name.clone()),
                    st.header,
                ));
            }
        }
        for (d, (_, _, name, _)) in data_fields.iter().enumerate() {
            cells.push(styled(
                text_cell(r1 + 1, base + m * dcount + d as u32, format!("Total {name}")),
                st.header,
            ));
        }

        for (i, rk) in row_keys.iter().enumerate() {
            let rr = r1 + 3 + i as u32;
            for (j, lbl) in rk.iter().enumerate() {
                cells.push(text_cell(rr, c1 + j as u32, lbl.clone()));
            }
            for (k, ck) in col_keys.iter().enumerate() {
                let gc = base + k as u32 * dcount;
                for (d, (fld, func, _, _)) in data_fields.iter().enumerate() {
                    if let Some(v) = value_for(rk, Some(ck), *fld, func.as_ref()) {
                        cells.push(opt_styled(num_cell(rr, gc + d as u32, v), field_xfs[d].0));
                    }
                }
            }
            for (d, (fld, func, _, _)) in data_fields.iter().enumerate() {
                if let Some(v) = value_for(rk, None, *fld, func.as_ref()) {
                    cells.push(styled(
                        num_cell(rr, base + m * dcount + d as u32, v),
                        field_xfs[d].1,
                    ));
                }
            }
        }

        let gr = r1 + 3 + row_keys.len() as u32;
        cells.push(styled(
            text_cell(gr, c1, "Grand Total".to_string()),
            st.total_label,
        ));
        for (k, ck) in col_keys.iter().enumerate() {
            let gc = base + k as u32 * dcount;
            for (d, (fld, func, _, _)) in data_fields.iter().enumerate() {
                let vals: Vec<PVal> = decoded
                    .iter()
                    .filter(|rec| matches(&col_fields, ck, rec))
                    .map(|rec| rec.get(*fld).cloned().unwrap_or(PVal::Blank))
                    .collect();
                if let Some(v) = aggregate(&vals, func.as_ref()) {
                    cells.push(styled(num_cell(gr, gc + d as u32, v), field_xfs[d].1));
                }
            }
        }
        for (d, (fld, func, _, _)) in data_fields.iter().enumerate() {
            if let Some(v) = value_for_all(&decoded, *fld, func.as_ref()) {
                cells.push(styled(
                    num_cell(gr, base + m * dcount + d as u32, v),
                    field_xfs[d].1,
                ));
            }
        }
        return cells;
    }

    let (data_fld, ref data_func, ref data_name, _) = data_fields[0];
    let (value_xf, total_xf) = field_xfs[0];

    let col_name = field_names[col_fields[0]].clone();
    let row_names: Vec<String> = row_fields.iter().map(|&f| field_names[f].clone()).collect();
    let m = col_keys.len() as u32;

    cells.push(styled(text_cell(r1, c1, data_name.clone()), st.header));
    cells.push(styled(text_cell(r1, c1 + r, col_name), st.header));

    for (j, name) in row_names.iter().enumerate() {
        cells.push(styled(text_cell(r1 + 1, c1 + j as u32, name.clone()), st.header));
    }
    for (k, ck) in col_keys.iter().enumerate() {
        cells.push(styled(text_cell(r1 + 1, c1 + r + k as u32, ck[0].clone()), st.header));
    }
    cells.push(styled(
        text_cell(r1 + 1, c1 + r + m, "Grand Total".to_string()),
        st.header,
    ));

    for (i, rk) in row_keys.iter().enumerate() {
        let rr = r1 + 2 + i as u32;
        for (j, lbl) in rk.iter().enumerate() {
            cells.push(text_cell(rr, c1 + j as u32, lbl.clone()));
        }
        for (k, ck) in col_keys.iter().enumerate() {
            if let Some(v) = value_for(rk, Some(ck), data_fld, data_func.as_ref()) {
                cells.push(opt_styled(num_cell(rr, c1 + r + k as u32, v), value_xf));
            }
        }
        if let Some(v) = value_for(rk, None, data_fld, data_func.as_ref()) {
            cells.push(styled(num_cell(rr, c1 + r + m, v), total_xf));
        }
    }

    let gr = r1 + 2 + row_keys.len() as u32;
    cells.push(styled(text_cell(gr, c1, "Grand Total".to_string()), st.total_label));
    for (k, ck) in col_keys.iter().enumerate() {
        let vals: Vec<PVal> = decoded
            .iter()
            .filter(|rec| matches(&col_fields, ck, rec))
            .map(|rec| rec.get(data_fld).cloned().unwrap_or(PVal::Blank))
            .collect();
        if let Some(v) = aggregate(&vals, data_func.as_ref()) {
            cells.push(styled(num_cell(gr, c1 + r + k as u32, v), total_xf));
        }
    }
    if let Some(v) = value_for_all(&decoded, data_fld, data_func.as_ref()) {
        cells.push(styled(num_cell(gr, c1 + r + m, v), total_xf));
    }

    cells
}

fn value_for_all(
    decoded: &[Vec<PVal>],
    data_fld: usize,
    func: Option<&x::DataConsolidateFunctionValues>,
) -> Option<f64> {
    let vals: Vec<PVal> = decoded
        .iter()
        .map(|rec| rec.get(data_fld).cloned().unwrap_or(PVal::Blank))
        .collect();
    aggregate(&vals, func)
}

#[cfg(test)]
mod tests {
    use super::*;
    use x::DataConsolidateFunctionValues as F;

    fn nums(v: &[f64]) -> Vec<PVal> {
        v.iter().map(|n| PVal::Num(*n)).collect()
    }

    #[test]
    fn aggregations_match_excel() {
        let v = nums(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert_eq!(aggregate(&v, Some(&F::Sum)), Some(40.0));
        assert_eq!(aggregate(&v, Some(&F::Average)), Some(5.0));
        assert_eq!(aggregate(&v, Some(&F::Count)), Some(8.0));
        assert_eq!(aggregate(&v, Some(&F::CountNumbers)), Some(8.0));
        assert_eq!(aggregate(&v, Some(&F::Maximum)), Some(9.0));
        assert_eq!(aggregate(&v, Some(&F::Minimum)), Some(2.0));
        assert_eq!(aggregate(&v, Some(&F::VarianceP)), Some(4.0));
        assert_eq!(aggregate(&v, Some(&F::StandardDeviationP)), Some(2.0));
        let var_s = aggregate(&v, Some(&F::Variance)).unwrap();
        assert!((var_s - 32.0 / 7.0).abs() < 1e-9);
    }

    #[test]
    fn count_versus_count_numbers() {
        let v = vec![
            PVal::Num(1.0),
            PVal::Text("x".into()),
            PVal::Blank,
            PVal::Num(2.0),
        ];
        assert_eq!(aggregate(&v, Some(&F::Count)), Some(3.0));
        assert_eq!(aggregate(&v, Some(&F::CountNumbers)), Some(2.0));
    }

    #[test]
    fn sort_orders_numbers_then_text_case_insensitively() {
        let t = unique_sorted(vec![
            vec![PVal::Text("Widget".into())],
            vec![PVal::Text("gadget".into())],
            vec![PVal::Text("Widget".into())],
            vec![PVal::Num(2.0)],
        ]);
        let labels: Vec<String> = t.iter().map(|x| x[0].label()).collect();
        assert_eq!(labels, vec!["2", "gadget", "Widget"]);
    }

    fn s_field(name: &str, vals: &[&str]) -> x::CacheField {
        x::CacheField {
            name: name.to_string(),
            shared_items: Some(x::SharedItems {
                count: Some(vals.len() as u32),
                shared_items_choice: vals
                    .iter()
                    .map(|v| {
                        x::SharedItemsChoice::StringItem(Box::new(x::StringItem {
                            val: v.to_string(),
                            ..Default::default()
                        }))
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn n_field(name: &str, vals: &[f64]) -> x::CacheField {
        x::CacheField {
            name: name.to_string(),
            shared_items: Some(x::SharedItems {
                count: Some(vals.len() as u32),
                shared_items_choice: vals
                    .iter()
                    .map(|v| {
                        x::SharedItemsChoice::NumberItem(Box::new(x::NumberItem {
                            val: *v,
                            ..Default::default()
                        }))
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn rec(idx: &[u32]) -> x::PivotCacheRecord {
        x::PivotCacheRecord {
            pivot_cache_record_choice: idx
                .iter()
                .map(|i| x::PivotCacheRecordChoice::FieldItem(Box::new(x::FieldItem { val: *i })))
                .collect(),
        }
    }

    fn find<'a>(cells: &'a [Cell], r: u32, c: u32) -> &'a Cell {
        cells
            .iter()
            .find(|x| x.r == r && x.c == c)
            .unwrap_or_else(|| panic!("no cell at ({r},{c})"))
    }

    #[test]
    fn computes_multiple_data_fields_no_columns() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(2),
                cache_field: vec![
                    s_field("Region", &["North", "South"]),
                    n_field("Amount", &[100.0, 50.0, 75.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![rec(&[0, 0]), rec(&[0, 1]), rec(&[1, 2])],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:C4".to_string(),
                first_header_row: 1,
                first_data_row: 1,
                first_data_column: 1,
                ..Default::default()
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(2),
                data_field: vec![
                    x::DataField {
                        name: Some("Sum of Amount".to_string()),
                        field: 1,
                        subtotal: Some(F::Sum),
                        ..Default::default()
                    },
                    x::DataField {
                        name: Some("Count of Amount".to_string()),
                        field: 1,
                        subtotal: Some(F::Count),
                        ..Default::default()
                    },
                ],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);

        assert_eq!(find(&cells, 1, 1).value.as_deref(), Some("Region"));
        assert_eq!(find(&cells, 1, 2).value.as_deref(), Some("Sum of Amount"));
        assert_eq!(find(&cells, 1, 3).value.as_deref(), Some("Count of Amount"));

        assert_eq!(find(&cells, 2, 1).value.as_deref(), Some("North"));
        assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("150"));
        assert_eq!(find(&cells, 2, 3).value.as_deref(), Some("2"));

        assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("South"));
        assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("75"));
        assert_eq!(find(&cells, 3, 3).value.as_deref(), Some("1"));

        assert_eq!(find(&cells, 4, 1).value.as_deref(), Some("Grand Total"));
        assert_eq!(find(&cells, 4, 2).value.as_deref(), Some("225"));
        assert_eq!(find(&cells, 4, 3).value.as_deref(), Some("3"));
    }

    #[test]
    fn data_field_num_fmt_propagates_to_value_and_total_cells() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(2),
                cache_field: vec![
                    s_field("Region", &["North", "South"]),
                    n_field("Amount", &[100.0, 50.0, 75.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![rec(&[0, 0]), rec(&[0, 1]), rec(&[1, 2])],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:B4".to_string(),
                first_header_row: 1,
                first_data_row: 1,
                first_data_column: 1,
                ..Default::default()
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(1),
                data_field: vec![x::DataField {
                    name: Some("Sum of Amount".to_string()),
                    field: 1,
                    subtotal: Some(F::Sum),
                    number_format_id: Some(44),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);

        let value = find(&cells, 2, 2);
        let vxf = &styles.cell_xfs[value.style_index.unwrap() as usize];
        assert_eq!(vxf.num_fmt_id, Some(44));

        let total = find(&cells, 4, 2);
        let txf = &styles.cell_xfs[total.style_index.unwrap() as usize];
        assert_eq!(txf.num_fmt_id, Some(44));
        assert!(styles.fonts[txf.font_id.unwrap() as usize].bold);
    }

    fn pivot_field_hiding(indices: &[u32]) -> x::PivotField {
        x::PivotField {
            items: Some(x::Items {
                item: indices
                    .iter()
                    .map(|i| x::Item {
                        index: Some(*i),
                        hidden: Some(true.into()),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn hidden_items_excluded_from_keys_and_totals() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(2),
                cache_field: vec![
                    s_field("Region", &["North", "South"]),
                    n_field("Amount", &[100.0, 50.0, 75.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![rec(&[0, 0]), rec(&[0, 1]), rec(&[1, 2])],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:B4".to_string(),
                first_header_row: 1,
                first_data_row: 1,
                first_data_column: 1,
                ..Default::default()
            }),
            pivot_fields: Some(x::PivotFields {
                count: Some(2),
                pivot_field: vec![pivot_field_hiding(&[1]), x::PivotField::default()],
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(1),
                data_field: vec![x::DataField {
                    name: Some("Sum of Amount".to_string()),
                    field: 1,
                    subtotal: Some(F::Sum),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);

        assert_eq!(find(&cells, 2, 1).value.as_deref(), Some("North"));
        assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("150"));
        assert!(!cells.iter().any(|c| c.value.as_deref() == Some("South")));
        assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("Grand Total"));
        assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("150"));
    }

    fn pivot_field_items(indices: &[u32]) -> x::PivotField {
        x::PivotField {
            items: Some(x::Items {
                item: indices
                    .iter()
                    .map(|i| x::Item {
                        index: Some(*i),
                        ..Default::default()
                    })
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn page_field_single_select_filters_records() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(3),
                cache_field: vec![
                    s_field("Region", &["North", "South"]),
                    s_field("Product", &["Widget", "Gadget"]),
                    n_field("Amount", &[100.0, 50.0, 75.0, 25.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![
                rec(&[0, 0, 0]),
                rec(&[0, 1, 1]),
                rec(&[1, 0, 2]),
                rec(&[1, 1, 3]),
            ],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:B4".to_string(),
                first_header_row: 1,
                first_data_row: 1,
                first_data_column: 1,
                ..Default::default()
            }),
            pivot_fields: Some(x::PivotFields {
                count: Some(3),
                pivot_field: vec![
                    pivot_field_items(&[0, 1]),
                    pivot_field_items(&[0, 1]),
                    x::PivotField::default(),
                ],
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            page_fields: Some(x::PageFields {
                count: Some(1),
                page_field: vec![x::PageField {
                    field: 1,
                    item: Some(1u32.into()),
                    ..Default::default()
                }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(1),
                data_field: vec![x::DataField {
                    name: Some("Sum of Amount".to_string()),
                    field: 2,
                    subtotal: Some(F::Sum),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);

        assert_eq!(find(&cells, 2, 1).value.as_deref(), Some("North"));
        assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("50"));
        assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("South"));
        assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("25"));
        assert_eq!(find(&cells, 4, 1).value.as_deref(), Some("Grand Total"));
        assert_eq!(find(&cells, 4, 2).value.as_deref(), Some("75"));
    }

    #[test]
    fn computes_multiple_data_fields_with_column_field() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(3),
                cache_field: vec![
                    s_field("Region", &["North", "South"]),
                    s_field("Product", &["Widget", "Gadget"]),
                    n_field("Amount", &[100.0, 50.0, 75.0, 25.0, 30.0, 60.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![
                rec(&[0, 0, 0]),
                rec(&[0, 1, 1]),
                rec(&[1, 0, 2]),
                rec(&[1, 1, 3]),
                rec(&[0, 0, 4]),
                rec(&[1, 1, 5]),
            ],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:G6".to_string(),
                first_header_row: 1,
                first_data_row: 2,
                first_data_column: 1,
                ..Default::default()
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            column_fields: Some(x::ColumnFields {
                count: Some(2),
                field: vec![x::Field { index: 1 }, x::Field { index: -2 }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(2),
                data_field: vec![
                    x::DataField {
                        name: Some("Sum of Amount".to_string()),
                        field: 2,
                        subtotal: Some(F::Sum),
                        ..Default::default()
                    },
                    x::DataField {
                        name: Some("Count of Amount".to_string()),
                        field: 2,
                        subtotal: Some(F::Count),
                        ..Default::default()
                    },
                ],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);
        let st = memo.unwrap();

        assert_eq!(find(&cells, 1, 2).value.as_deref(), Some("Product"));
        assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("Gadget"));
        assert_eq!(find(&cells, 2, 4).value.as_deref(), Some("Widget"));
        assert_eq!(
            find(&cells, 2, 6).value.as_deref(),
            Some("Total Sum of Amount")
        );
        assert_eq!(
            find(&cells, 2, 7).value.as_deref(),
            Some("Total Count of Amount")
        );
        assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("Region"));
        assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("Sum of Amount"));
        assert_eq!(find(&cells, 3, 3).value.as_deref(), Some("Count of Amount"));

        assert_eq!(find(&cells, 4, 1).value.as_deref(), Some("North"));
        assert_eq!(find(&cells, 4, 2).value.as_deref(), Some("50"));
        assert_eq!(find(&cells, 4, 3).value.as_deref(), Some("1"));
        assert_eq!(find(&cells, 4, 4).value.as_deref(), Some("130"));
        assert_eq!(find(&cells, 4, 5).value.as_deref(), Some("2"));
        assert_eq!(find(&cells, 4, 6).value.as_deref(), Some("180"));
        assert_eq!(find(&cells, 4, 7).value.as_deref(), Some("3"));

        assert_eq!(find(&cells, 5, 1).value.as_deref(), Some("South"));
        assert_eq!(find(&cells, 5, 6).value.as_deref(), Some("160"));
        assert_eq!(find(&cells, 5, 7).value.as_deref(), Some("3"));

        assert_eq!(find(&cells, 6, 1).value.as_deref(), Some("Grand Total"));
        assert_eq!(find(&cells, 6, 2).value.as_deref(), Some("135"));
        assert_eq!(find(&cells, 6, 3).value.as_deref(), Some("3"));
        assert_eq!(find(&cells, 6, 6).value.as_deref(), Some("340"));
        assert_eq!(find(&cells, 6, 7).value.as_deref(), Some("6"));

        assert_eq!(find(&cells, 1, 2).style_index, Some(st.header));
        assert_eq!(find(&cells, 4, 6).style_index, Some(st.total_value));
        assert_eq!(find(&cells, 4, 2).style_index, None);
        assert_eq!(find(&cells, 6, 1).style_index, Some(st.total_label));
    }

    #[test]
    fn computes_nested_column_fields() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(4),
                cache_field: vec![
                    n_field("Year", &[2020.0, 2021.0]),
                    s_field("Region", &["North", "South"]),
                    s_field("Product", &["Widget", "Gadget"]),
                    n_field("Amount", &[100.0, 50.0, 75.0, 25.0, 30.0, 60.0, 40.0, 20.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![
                rec(&[0, 0, 0, 0]),
                rec(&[0, 0, 1, 1]),
                rec(&[0, 1, 0, 2]),
                rec(&[0, 1, 1, 3]),
                rec(&[1, 0, 0, 4]),
                rec(&[1, 0, 1, 5]),
                rec(&[1, 1, 0, 6]),
                rec(&[1, 1, 1, 7]),
            ],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:H6".to_string(),
                first_header_row: 1,
                first_data_row: 3,
                first_data_column: 1,
                ..Default::default()
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            column_fields: Some(x::ColumnFields {
                count: Some(2),
                field: vec![x::Field { index: 1 }, x::Field { index: 2 }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(1),
                data_field: vec![x::DataField {
                    name: Some("Sum of Amount".to_string()),
                    field: 3,
                    subtotal: Some(F::Sum),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);
        let st = memo.unwrap();

        assert_eq!(find(&cells, 1, 1).value.as_deref(), Some("Sum of Amount"));
        assert_eq!(find(&cells, 1, 2).value.as_deref(), Some("Region"));
        assert_eq!(find(&cells, 1, 3).value.as_deref(), Some("Product"));

        assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("North"));
        assert_eq!(find(&cells, 2, 4).value.as_deref(), Some("North Total"));
        assert_eq!(find(&cells, 2, 5).value.as_deref(), Some("South"));
        assert_eq!(find(&cells, 2, 7).value.as_deref(), Some("South Total"));
        assert_eq!(find(&cells, 2, 8).value.as_deref(), Some("Grand Total"));

        assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("Year"));
        assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("Gadget"));
        assert_eq!(find(&cells, 3, 3).value.as_deref(), Some("Widget"));

        assert_eq!(find(&cells, 4, 1).value.as_deref(), Some("2020"));
        assert_eq!(find(&cells, 4, 2).value.as_deref(), Some("50"));
        assert_eq!(find(&cells, 4, 3).value.as_deref(), Some("100"));
        assert_eq!(find(&cells, 4, 4).value.as_deref(), Some("150"));
        assert_eq!(find(&cells, 4, 7).value.as_deref(), Some("100"));
        assert_eq!(find(&cells, 4, 8).value.as_deref(), Some("250"));

        assert_eq!(find(&cells, 6, 1).value.as_deref(), Some("Grand Total"));
        assert_eq!(find(&cells, 6, 4).value.as_deref(), Some("240"));
        assert_eq!(find(&cells, 6, 8).value.as_deref(), Some("400"));

        assert_eq!(find(&cells, 1, 1).style_index, Some(st.header));
        assert_eq!(find(&cells, 4, 2).style_index, None);
        assert_eq!(find(&cells, 4, 4).style_index, Some(st.total_value));
        assert_eq!(find(&cells, 4, 8).style_index, Some(st.total_value));
        assert_eq!(find(&cells, 6, 1).style_index, Some(st.total_label));
    }

    #[test]
    fn computes_two_axis_grid_with_totals() {
        let cache_def = x::PivotCacheDefinition {
            cache_fields: Box::new(x::CacheFields {
                count: Some(3),
                cache_field: vec![
                    s_field("Region", &["North", "South"]),
                    s_field("Product", &["Widget", "Gadget"]),
                    n_field("Amount", &[100.0, 50.0, 75.0, 85.0]),
                ],
            }),
            ..Default::default()
        };
        let records = x::PivotCacheRecords {
            pivot_cache_record: vec![
                rec(&[0, 0, 0]),
                rec(&[0, 1, 1]),
                rec(&[1, 0, 2]),
                rec(&[1, 1, 3]),
            ],
            ..Default::default()
        };
        let pt = x::PivotTableDefinition {
            location: Box::new(x::Location {
                reference: "A1:D5".to_string(),
                first_header_row: 1,
                first_data_row: 2,
                first_data_column: 1,
                ..Default::default()
            }),
            row_fields: Some(x::RowFields {
                count: Some(1),
                field: vec![x::Field { index: 0 }],
            }),
            column_fields: Some(x::ColumnFields {
                count: Some(1),
                field: vec![x::Field { index: 1 }],
            }),
            data_fields: Some(x::DataFields {
                count: Some(1),
                data_field: vec![x::DataField {
                    name: Some("Sum of Amount".to_string()),
                    field: 2,
                    subtotal: Some(F::Sum),
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };

        let mut styles = Styles::default();
        let mut memo = None;
        let cells = compute_cells(&pt, &cache_def, &records, &mut styles, &mut memo);

        assert_eq!(find(&cells, 1, 1).value.as_deref(), Some("Sum of Amount"));
        assert_eq!(find(&cells, 1, 2).value.as_deref(), Some("Product"));
        assert_eq!(find(&cells, 2, 1).value.as_deref(), Some("Region"));
        assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("Gadget"));
        assert_eq!(find(&cells, 2, 3).value.as_deref(), Some("Widget"));
        assert_eq!(find(&cells, 2, 4).value.as_deref(), Some("Grand Total"));

        assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("North"));
        assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("50"));
        assert_eq!(find(&cells, 3, 3).value.as_deref(), Some("100"));
        assert_eq!(find(&cells, 3, 4).value.as_deref(), Some("150"));

        assert_eq!(find(&cells, 4, 1).value.as_deref(), Some("South"));
        assert_eq!(find(&cells, 4, 4).value.as_deref(), Some("160"));

        assert_eq!(find(&cells, 5, 1).value.as_deref(), Some("Grand Total"));
        assert_eq!(find(&cells, 5, 2).value.as_deref(), Some("135"));
        assert_eq!(find(&cells, 5, 3).value.as_deref(), Some("175"));
        assert_eq!(find(&cells, 5, 4).value.as_deref(), Some("310"));

        let st = memo.unwrap();
        assert_eq!(find(&cells, 1, 1).style_index, Some(st.header));
        assert_eq!(find(&cells, 2, 4).style_index, Some(st.header));
        assert_eq!(find(&cells, 3, 1).style_index, None);
        assert_eq!(find(&cells, 3, 2).style_index, None);
        assert_eq!(find(&cells, 3, 4).style_index, Some(st.total_value));
        assert_eq!(find(&cells, 5, 1).style_index, Some(st.total_label));
        assert_eq!(find(&cells, 5, 4).style_index, Some(st.total_value));

        let header_xf = &styles.cell_xfs[st.header as usize];
        let fill = &styles.fills[header_xf.fill_id.unwrap() as usize];
        assert_eq!(fill.pattern_type.as_deref(), Some("solid"));
        assert!(styles.fonts[header_xf.font_id.unwrap() as usize].bold);
        assert!(styles.fonts[styles.cell_xfs[st.total_value as usize].font_id.unwrap() as usize].bold);
    }
}
