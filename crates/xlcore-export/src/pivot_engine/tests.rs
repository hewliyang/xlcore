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
    let t = ordered_unique(
        vec![
            vec![PVal::Text("Widget".into())],
            vec![PVal::Text("gadget".into())],
            vec![PVal::Text("Widget".into())],
            vec![PVal::Num(2.0)],
        ],
        &[None],
    );
    let labels: Vec<String> = t.iter().map(|x| x[0].label()).collect();
    assert_eq!(labels, vec!["2", "gadget", "Widget"]);
}

#[test]
fn axis_honors_pivot_field_item_order() {
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
            pivot_field: vec![pivot_field_items(&[1, 0]), x::PivotField::default()],
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

    assert_eq!(find(&cells, 2, 1).value.as_deref(), Some("South"));
    assert_eq!(find(&cells, 2, 2).value.as_deref(), Some("75"));
    assert_eq!(find(&cells, 3, 1).value.as_deref(), Some("North"));
    assert_eq!(find(&cells, 3, 2).value.as_deref(), Some("150"));
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
