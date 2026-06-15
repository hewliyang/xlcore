use std::collections::BTreeSet;
use xlcore_types::{AutoFilterCriteria, AutoFilterCustomCriterion, AutoFilterOperator};

pub fn compute_hidden_rows(
    first_data_row: u32,
    rows: &[Vec<String>],
    columns: &[(u32, &AutoFilterCriteria)],
) -> BTreeSet<u32> {
    let mut hidden = BTreeSet::new();
    for (i, row) in rows.iter().enumerate() {
        let row_num = first_data_row + i as u32;
        let mut visible = true;
        for (col_offset, criteria) in columns {
            let cell = row.get(*col_offset as usize).map(String::as_str).unwrap_or("");
            if !passes_column(cell, criteria, rows, *col_offset) {
                visible = false;
                break;
            }
        }
        if !visible {
            hidden.insert(row_num);
        }
    }
    hidden
}

fn passes_column(
    cell: &str,
    criteria: &AutoFilterCriteria,
    rows: &[Vec<String>],
    col_offset: u32,
) -> bool {
    match criteria {
        AutoFilterCriteria::Values { values, blank } => {
            if cell.is_empty() {
                blank.unwrap_or(false)
            } else {
                values.iter().any(|v| v == cell)
            }
        }
        AutoFilterCriteria::Custom {
            logical_and,
            criteria,
        } => passes_custom(cell, logical_and.unwrap_or(false), criteria),
        AutoFilterCriteria::Top10 { top, percent, val } => {
            passes_top10(cell, top.unwrap_or(true), percent.unwrap_or(false), *val, rows, col_offset)
        }
        AutoFilterCriteria::Unsupported { .. } => true,
    }
}

fn passes_custom(cell: &str, logical_and: bool, criteria: &[AutoFilterCustomCriterion]) -> bool {
    if criteria.is_empty() {
        return true;
    }
    let mut acc = logical_and;
    for crit in criteria {
        let r = matches_criterion(cell, crit);
        if logical_and {
            acc = acc && r;
        } else {
            acc = acc || r;
        }
    }
    acc
}

fn matches_criterion(cell: &str, crit: &AutoFilterCustomCriterion) -> bool {
    let value = &crit.value;
    match crit.operator {
        AutoFilterOperator::Equal => wildcard_match(value, cell),
        AutoFilterOperator::NotEqual => !wildcard_match(value, cell),
        AutoFilterOperator::GreaterThan
        | AutoFilterOperator::GreaterThanOrEqual
        | AutoFilterOperator::LessThan
        | AutoFilterOperator::LessThanOrEqual => {
            let ord = compare(cell, value);
            match crit.operator {
                AutoFilterOperator::GreaterThan => ord == std::cmp::Ordering::Greater,
                AutoFilterOperator::GreaterThanOrEqual => ord != std::cmp::Ordering::Less,
                AutoFilterOperator::LessThan => ord == std::cmp::Ordering::Less,
                AutoFilterOperator::LessThanOrEqual => ord != std::cmp::Ordering::Greater,
                _ => unreachable!(),
            }
        }
    }
}

fn compare(cell: &str, value: &str) -> std::cmp::Ordering {
    match (parse_number(cell), parse_number(value)) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        _ => cell.to_lowercase().cmp(&value.to_lowercase()),
    }
}

fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        t.parse::<f64>().ok()
    }
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') && !pattern.contains('?') {
        return pattern.eq_ignore_ascii_case(text);
    }
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    wildcard_rec(&p, &t)
}

fn wildcard_rec(p: &[char], t: &[char]) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    match p[0] {
        '*' => {
            if wildcard_rec(&p[1..], t) {
                return true;
            }
            !t.is_empty() && wildcard_rec(p, &t[1..])
        }
        '?' => !t.is_empty() && wildcard_rec(&p[1..], &t[1..]),
        c => !t.is_empty() && t[0] == c && wildcard_rec(&p[1..], &t[1..]),
    }
}

fn passes_top10(
    cell: &str,
    top: bool,
    percent: bool,
    val: f64,
    rows: &[Vec<String>],
    col_offset: u32,
) -> bool {
    let cell_num = match parse_number(cell) {
        Some(n) => n,
        None => return false,
    };
    let mut nums: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get(col_offset as usize))
        .filter_map(|s| parse_number(s))
        .collect();
    if nums.is_empty() {
        return false;
    }
    let count = nums.len();
    let keep = if percent {
        ((val / 100.0) * count as f64).ceil() as usize
    } else {
        val as usize
    };
    let keep = keep.clamp(1, count);
    if top {
        nums.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        cell_num >= nums[keep - 1]
    } else {
        nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        cell_num <= nums[keep - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn values_filter_keeps_matching() {
        let rows = vec![row(&["apple"]), row(&["banana"]), row(&[""]), row(&["cherry"])];
        let crit = AutoFilterCriteria::Values {
            values: vec!["apple".into(), "cherry".into()],
            blank: None,
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([3, 4]));
    }

    #[test]
    fn values_filter_blank_included() {
        let rows = vec![row(&["apple"]), row(&[""]), row(&["banana"])];
        let crit = AutoFilterCriteria::Values {
            values: vec!["apple".into()],
            blank: Some(true),
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([4]));
    }

    #[test]
    fn custom_numeric_compare_and() {
        let rows = vec![row(&["5"]), row(&["15"]), row(&["25"]), row(&["35"])];
        let crit = AutoFilterCriteria::Custom {
            logical_and: Some(true),
            criteria: vec![
                AutoFilterCustomCriterion {
                    operator: AutoFilterOperator::GreaterThan,
                    value: "10".into(),
                },
                AutoFilterCustomCriterion {
                    operator: AutoFilterOperator::LessThanOrEqual,
                    value: "25".into(),
                },
            ],
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([2, 5]));
    }

    #[test]
    fn custom_or() {
        let rows = vec![row(&["5"]), row(&["15"]), row(&["25"])];
        let crit = AutoFilterCriteria::Custom {
            logical_and: Some(false),
            criteria: vec![
                AutoFilterCustomCriterion {
                    operator: AutoFilterOperator::LessThan,
                    value: "10".into(),
                },
                AutoFilterCustomCriterion {
                    operator: AutoFilterOperator::GreaterThan,
                    value: "20".into(),
                },
            ],
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([3]));
    }

    #[test]
    fn custom_wildcard_equal() {
        let rows = vec![row(&["apple"]), row(&["apricot"]), row(&["banana"])];
        let crit = AutoFilterCriteria::Custom {
            logical_and: Some(true),
            criteria: vec![AutoFilterCustomCriterion {
                operator: AutoFilterOperator::Equal,
                value: "ap*".into(),
            }],
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([4]));
    }

    #[test]
    fn custom_wildcard_notequal_question() {
        let rows = vec![row(&["cat"]), row(&["cot"]), row(&["cart"])];
        let crit = AutoFilterCriteria::Custom {
            logical_and: Some(true),
            criteria: vec![AutoFilterCustomCriterion {
                operator: AutoFilterOperator::NotEqual,
                value: "c?t".into(),
            }],
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([2, 3]));
    }

    #[test]
    fn top10_top_count() {
        let rows = vec![row(&["10"]), row(&["50"]), row(&["30"]), row(&["20"]), row(&["40"])];
        let crit = AutoFilterCriteria::Top10 {
            top: Some(true),
            percent: Some(false),
            val: 2.0,
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([2, 4, 5]));
    }

    #[test]
    fn top10_bottom_count() {
        let rows = vec![row(&["10"]), row(&["50"]), row(&["30"]), row(&["20"]), row(&["40"])];
        let crit = AutoFilterCriteria::Top10 {
            top: Some(false),
            percent: Some(false),
            val: 2.0,
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &crit)]);
        assert_eq!(hidden, BTreeSet::from([3, 4, 6]));
    }

    #[test]
    fn multi_column_and() {
        let rows = vec![
            row(&["apple", "5"]),
            row(&["apple", "15"]),
            row(&["banana", "15"]),
        ];
        let values = AutoFilterCriteria::Values {
            values: vec!["apple".into()],
            blank: None,
        };
        let custom = AutoFilterCriteria::Custom {
            logical_and: Some(true),
            criteria: vec![AutoFilterCustomCriterion {
                operator: AutoFilterOperator::GreaterThan,
                value: "10".into(),
            }],
        };
        let hidden = compute_hidden_rows(2, &rows, &[(0, &values), (1, &custom)]);
        assert_eq!(hidden, BTreeSet::from([2, 4]));
    }
}
