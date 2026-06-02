use crate::schema::Cell;
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;
use std::cmp::Ordering;
use xlcore_io::parse_range;

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

pub fn compute_cells(
    pt: &x::PivotTableDefinition,
    cache_def: &x::PivotCacheDefinition,
    records: &x::PivotCacheRecords,
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

    let decoded: Vec<Vec<PVal>> = records
        .pivot_cache_record
        .iter()
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
    let data_fields: Vec<(usize, Option<x::DataConsolidateFunctionValues>, String)> = pt
        .data_fields
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
                    (fld, d.subtotal.clone(), name)
                })
                .collect()
        })
        .unwrap_or_default();

    if row_fields.is_empty() || col_fields.len() > 1 || data_fields.len() != 1 {
        return Vec::new();
    }

    let Some(((r1, c1), _)) = parse_range(pt.location.reference.as_str()) else {
        return Vec::new();
    };

    let (data_fld, ref data_func, ref data_name) = data_fields[0];

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
    let value_for = |row_key: &[String], col_key: Option<&[String]>| -> Option<f64> {
        let vals: Vec<PVal> = decoded
            .iter()
            .filter(|rec| matches(&row_fields, row_key, rec))
            .filter(|rec| {
                col_key
                    .map(|ck| matches(&col_fields, ck, rec))
                    .unwrap_or(true)
            })
            .map(|rec| rec.get(data_fld).cloned().unwrap_or(PVal::Blank))
            .collect();
        aggregate(&vals, data_func.as_ref())
    };

    let r = row_fields.len() as u32;
    let mut cells = Vec::new();

    if col_fields.is_empty() {
        let row_names: Vec<String> = row_fields.iter().map(|&f| field_names[f].clone()).collect();
        for (j, name) in row_names.iter().enumerate() {
            cells.push(text_cell(r1, c1 + j as u32, name.clone()));
        }
        cells.push(text_cell(r1, c1 + r, data_name.clone()));
        for (i, rk) in row_keys.iter().enumerate() {
            let rr = r1 + 1 + i as u32;
            for (j, lbl) in rk.iter().enumerate() {
                cells.push(text_cell(rr, c1 + j as u32, lbl.clone()));
            }
            if let Some(v) = value_for(rk, None) {
                cells.push(num_cell(rr, c1 + r, v));
            }
        }
        let gr = r1 + 1 + row_keys.len() as u32;
        cells.push(text_cell(gr, c1, "Grand Total".to_string()));
        if let Some(v) = value_for_all(&decoded, data_fld, data_func.as_ref()) {
            cells.push(num_cell(gr, c1 + r, v));
        }
        return cells;
    }

    let col_name = field_names[col_fields[0]].clone();
    let row_names: Vec<String> = row_fields.iter().map(|&f| field_names[f].clone()).collect();
    let m = col_keys.len() as u32;

    cells.push(text_cell(r1, c1, data_name.clone()));
    cells.push(text_cell(r1, c1 + r, col_name));

    for (j, name) in row_names.iter().enumerate() {
        cells.push(text_cell(r1 + 1, c1 + j as u32, name.clone()));
    }
    for (k, ck) in col_keys.iter().enumerate() {
        cells.push(text_cell(r1 + 1, c1 + r + k as u32, ck[0].clone()));
    }
    cells.push(text_cell(r1 + 1, c1 + r + m, "Grand Total".to_string()));

    for (i, rk) in row_keys.iter().enumerate() {
        let rr = r1 + 2 + i as u32;
        for (j, lbl) in rk.iter().enumerate() {
            cells.push(text_cell(rr, c1 + j as u32, lbl.clone()));
        }
        for (k, ck) in col_keys.iter().enumerate() {
            if let Some(v) = value_for(rk, Some(ck)) {
                cells.push(num_cell(rr, c1 + r + k as u32, v));
            }
        }
        if let Some(v) = value_for(rk, None) {
            cells.push(num_cell(rr, c1 + r + m, v));
        }
    }

    let gr = r1 + 2 + row_keys.len() as u32;
    cells.push(text_cell(gr, c1, "Grand Total".to_string()));
    for (k, ck) in col_keys.iter().enumerate() {
        let vals: Vec<PVal> = decoded
            .iter()
            .filter(|rec| matches(&col_fields, ck, rec))
            .map(|rec| rec.get(data_fld).cloned().unwrap_or(PVal::Blank))
            .collect();
        if let Some(v) = aggregate(&vals, data_func.as_ref()) {
            cells.push(num_cell(gr, c1 + r + k as u32, v));
        }
    }
    if let Some(v) = value_for_all(&decoded, data_fld, data_func.as_ref()) {
        cells.push(num_cell(gr, c1 + r + m, v));
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

        let cells = compute_cells(&pt, &cache_def, &records);

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
    }
}
