use crate::*;

#[test]
fn pivots_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let rows = [
        ("North", "Widget", 100.0),
        ("North", "Gadget", 50.0),
        ("South", "Widget", 75.0),
        ("South", "Gadget", 25.0),
        ("North", "Widget", 30.0),
        ("South", "Gadget", 60.0),
    ];
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Product").unwrap();
    wb.set_value("Sheet1!C1", "Amount").unwrap();
    for (i, (region, product, amount)) in rows.iter().enumerate() {
        let r = i as u32 + 2;
        wb.set_value(format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(format!("Sheet1!B{r}"), *product).unwrap();
        wb.set_value(format!("Sheet1!C{r}"), *amount).unwrap();
    }
    wb.create_sheet("Pivot").unwrap();

    let info = wb
        .set_pivot(
            "Pivot",
            PivotPatch {
                anchor_cell: "Pivot!A1".to_string(),
                source_ref: "Sheet1!A1:C7".to_string(),
                name: Some("SalesPivot".to_string()),
                row_fields: vec!["Region".to_string()],
                column_fields: vec!["Product".to_string()],
                filter_fields: vec![],
                data_fields: vec![PivotDataField {
                    field: "Amount".to_string(),
                    aggregation: PivotAggregation::Sum,
                    name: None,
                    number_format: Some("0.00%".to_string()),
                }],
                hidden_items: None,
            },
        )
        .unwrap();
    assert_eq!(info.name, "SalesPivot");
    assert_eq!(info.row_fields, vec!["Region".to_string()]);
    assert_eq!(info.column_fields, vec!["Product".to_string()]);
    assert_eq!(info.data_fields.len(), 1);
    assert_eq!(info.data_fields[0].field, "Amount");
    assert_eq!(info.anchor_cell, "A1");
    assert!(!info.id.is_empty());

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let pivots = reopened.pivots(None).unwrap();
    assert_eq!(pivots.len(), 1);
    let p = &pivots[0];
    assert_eq!(p.sheet, "Pivot");
    assert_eq!(p.name, "SalesPivot");
    assert_eq!(p.source_ref, "Sheet1!A1:C7");
    assert_eq!(p.row_fields, vec!["Region".to_string()]);
    assert_eq!(p.data_fields[0].aggregation, PivotAggregation::Sum);
    assert_eq!(p.data_fields[0].number_format.as_deref(), Some("0.00%"));

    let removed = reopened.remove_pivot("Pivot", &p.id).unwrap().unwrap();
    assert_eq!(removed.id, p.id);
    assert!(reopened.pivots(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.pivots(None).unwrap().is_empty());
}

#[test]
fn pivot_update_merges_partial_and_keeps_unset_fields() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Product").unwrap();
    wb.set_value("Sheet1!C1", "Amount").unwrap();
    let rows = [
        ("North", "Widget", 100.0),
        ("South", "Widget", 75.0),
        ("North", "Gadget", 50.0),
    ];
    for (i, (region, product, amount)) in rows.iter().enumerate() {
        let r = i as u32 + 2;
        wb.set_value(format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(format!("Sheet1!B{r}"), *product).unwrap();
        wb.set_value(format!("Sheet1!C{r}"), *amount).unwrap();
    }
    wb.create_sheet("Pivot").unwrap();

    let info = wb
        .set_pivot(
            "Pivot",
            PivotPatch {
                anchor_cell: "Pivot!A1".to_string(),
                source_ref: "Sheet1!A1:C4".to_string(),
                name: Some("SalesPivot".to_string()),
                row_fields: vec!["Region".to_string()],
                column_fields: vec!["Product".to_string()],
                filter_fields: vec![],
                data_fields: vec![PivotDataField {
                    field: "Amount".to_string(),
                    aggregation: PivotAggregation::Sum,
                    name: None,
                    number_format: None,
                }],
                hidden_items: None,
            },
        )
        .unwrap();

    let updated = wb
        .update_pivot(
            "Pivot",
            &info.id,
            PivotUpdate {
                row_fields: Some(vec!["Product".to_string()]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.name, "SalesPivot");
    assert_eq!(updated.row_fields, vec!["Product".to_string()]);
    assert_eq!(updated.column_fields, vec!["Product".to_string()]);
    assert_eq!(updated.source_ref, "Sheet1!A1:C4");
    assert_eq!(updated.data_fields[0].field, "Amount");

    assert_eq!(wb.pivots(Some("Pivot")).unwrap().len(), 1);

    let err = wb
        .update_pivot("Pivot", "missing-id", PivotUpdate::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::Other);
}

#[test]
fn pivot_update_roundtrips_multi_data_field_with_column() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Channel").unwrap();
    wb.set_value("Sheet1!C1", "Units").unwrap();
    wb.set_value("Sheet1!D1", "Revenue").unwrap();
    let rows = [
        ("North", "Online", 3.0, 100.0),
        ("North", "Retail", 5.0, 250.0),
        ("South", "Online", 2.0, 75.0),
        ("South", "Retail", 4.0, 180.0),
    ];
    for (i, (region, channel, units, revenue)) in rows.iter().enumerate() {
        let r = i as u32 + 2;
        wb.set_value(format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(format!("Sheet1!B{r}"), *channel).unwrap();
        wb.set_value(format!("Sheet1!C{r}"), *units).unwrap();
        wb.set_value(format!("Sheet1!D{r}"), *revenue).unwrap();
    }
    wb.create_sheet("Pivot").unwrap();

    let info = wb
        .set_pivot(
            "Pivot",
            PivotPatch {
                anchor_cell: "Pivot!A1".to_string(),
                source_ref: "Sheet1!A1:D5".to_string(),
                name: Some("SalesPivot".to_string()),
                row_fields: vec!["Region".to_string()],
                column_fields: vec!["Channel".to_string()],
                filter_fields: vec![],
                data_fields: vec![
                    PivotDataField {
                        field: "Revenue".to_string(),
                        aggregation: PivotAggregation::Sum,
                        name: Some("Total Revenue".to_string()),
                        number_format: None,
                    },
                    PivotDataField {
                        field: "Units".to_string(),
                        aggregation: PivotAggregation::Sum,
                        name: Some("Total Units".to_string()),
                        number_format: None,
                    },
                ],
                hidden_items: None,
            },
        )
        .unwrap();

    assert_eq!(info.column_fields, vec!["Channel".to_string()]);

    let updated = wb
        .update_pivot(
            "Pivot",
            &info.id,
            PivotUpdate {
                hidden_items: Some(vec![PivotFieldFilter {
                    field: "Channel".to_string(),
                    hide: vec!["Retail".to_string()],
                }]),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(updated.column_fields, vec!["Channel".to_string()]);
    assert_eq!(updated.data_fields.len(), 2);
    assert_eq!(
        updated.hidden_items,
        Some(vec![PivotFieldFilter {
            field: "Channel".to_string(),
            hide: vec!["Retail".to_string()],
        }])
    );
    assert_eq!(wb.pivots(Some("Pivot")).unwrap().len(), 1);
}

#[test]
fn pivot_requires_data_field_and_axis() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Amount").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();

    let no_data = wb.set_pivot(
        "Sheet1",
        PivotPatch {
            anchor_cell: "Sheet1!D1".to_string(),
            source_ref: "Sheet1!A1:B2".to_string(),
            name: None,
            row_fields: vec!["Region".to_string()],
            column_fields: vec![],
            filter_fields: vec![],
            data_fields: vec![],
            hidden_items: None,
        },
    );
    assert_eq!(no_data.unwrap_err().code, ApiErrorCode::InvalidPivot);

    let no_axis = wb.set_pivot(
        "Sheet1",
        PivotPatch {
            anchor_cell: "Sheet1!D1".to_string(),
            source_ref: "Sheet1!A1:B2".to_string(),
            name: None,
            row_fields: vec![],
            column_fields: vec![],
            filter_fields: vec![],
            data_fields: vec![PivotDataField {
                field: "Amount".to_string(),
                aggregation: PivotAggregation::Sum,
                name: None,
                number_format: None,
            }],
            hidden_items: None,
        },
    );
    assert_eq!(no_axis.unwrap_err().code, ApiErrorCode::InvalidPivot);
}

#[test]
fn pivot_preview_aggregates_without_writing_parts() {
    let mut wb = Workbook::new().unwrap();
    let rows = [
        ("North", "Widget", 100.0),
        ("North", "Gadget", 50.0),
        ("South", "Widget", 75.0),
        ("South", "Gadget", 25.0),
        ("North", "Widget", 30.0),
        ("South", "Gadget", 60.0),
    ];
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Product").unwrap();
    wb.set_value("Sheet1!C1", "Amount").unwrap();
    for (i, (region, product, amount)) in rows.iter().enumerate() {
        let r = i as u32 + 2;
        wb.set_value(format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(format!("Sheet1!B{r}"), *product).unwrap();
        wb.set_value(format!("Sheet1!C{r}"), *amount).unwrap();
    }

    let grid = wb
        .pivot_preview(
            "Sheet1",
            PivotPatch {
                anchor_cell: "Sheet1!E1".to_string(),
                source_ref: "Sheet1!A1:C7".to_string(),
                name: None,
                row_fields: vec!["Region".to_string()],
                column_fields: vec![],
                filter_fields: vec![],
                data_fields: vec![PivotDataField {
                    field: "Amount".to_string(),
                    aggregation: PivotAggregation::Sum,
                    name: Some("Sum of Amount".to_string()),
                    number_format: None,
                }],
                hidden_items: None,
            },
        )
        .unwrap();

    let at = |row: u32, col: u32| -> &PivotGridCell {
        grid.cells
            .iter()
            .find(|c| c.row == row && c.col == col)
            .unwrap_or_else(|| panic!("no cell at ({row},{col})"))
    };

    assert_eq!(grid.rows, 4);
    assert_eq!(grid.cols, 2);
    assert_eq!(at(0, 0).value.as_deref(), Some("Region"));
    assert_eq!(at(0, 0).role, PivotCellRole::Header);
    assert_eq!(at(0, 1).value.as_deref(), Some("Sum of Amount"));
    assert_eq!(at(1, 0).value.as_deref(), Some("North"));
    assert_eq!(at(1, 0).role, PivotCellRole::Label);
    assert_eq!(at(1, 1).value.as_deref(), Some("180"));
    assert_eq!(at(1, 1).role, PivotCellRole::Value);
    assert_eq!(at(2, 1).value.as_deref(), Some("160"));
    assert_eq!(at(3, 0).value.as_deref(), Some("Grand Total"));
    assert_eq!(at(3, 0).role, PivotCellRole::TotalLabel);
    assert_eq!(at(3, 1).value.as_deref(), Some("340"));
    assert_eq!(at(3, 1).role, PivotCellRole::TotalValue);

    assert!(wb.pivots(None).unwrap().is_empty());
}

#[test]
fn pivot_hidden_items_filter_and_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let rows = [
        ("North", "Widget", 100.0),
        ("North", "Gadget", 50.0),
        ("South", "Widget", 75.0),
        ("South", "Gadget", 25.0),
    ];
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Product").unwrap();
    wb.set_value("Sheet1!C1", "Amount").unwrap();
    for (i, (region, product, amount)) in rows.iter().enumerate() {
        let r = i as u32 + 2;
        wb.set_value(format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(format!("Sheet1!B{r}"), *product).unwrap();
        wb.set_value(format!("Sheet1!C{r}"), *amount).unwrap();
    }

    let patch = PivotPatch {
        anchor_cell: "Sheet1!E1".to_string(),
        source_ref: "Sheet1!A1:C5".to_string(),
        name: Some("P".to_string()),
        row_fields: vec!["Region".to_string()],
        column_fields: vec![],
        filter_fields: vec![],
        data_fields: vec![PivotDataField {
            field: "Amount".to_string(),
            aggregation: PivotAggregation::Sum,
            name: Some("Sum of Amount".to_string()),
            number_format: None,
        }],
        hidden_items: Some(vec![PivotFieldFilter {
            field: "Region".to_string(),
            hide: vec!["South".to_string()],
        }]),
    };

    let grid = wb.pivot_preview("Sheet1", patch.clone()).unwrap();
    assert!(!grid
        .cells
        .iter()
        .any(|c| c.value.as_deref() == Some("South")));
    let north = grid
        .cells
        .iter()
        .find(|c| c.value.as_deref() == Some("North"))
        .unwrap();
    let total = grid
        .cells
        .iter()
        .find(|c| c.row == north.row && c.col == 1)
        .unwrap();
    assert_eq!(total.value.as_deref(), Some("150"));

    wb.create_sheet("Pivot").unwrap();
    let mut p2 = patch;
    p2.anchor_cell = "Pivot!A1".to_string();
    wb.set_pivot("Pivot", p2).unwrap();
    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let pivots = reopened.pivots(None).unwrap();
    assert_eq!(pivots.len(), 1);
    assert_eq!(
        pivots[0].hidden_items,
        Some(vec![PivotFieldFilter {
            field: "Region".to_string(),
            hide: vec!["South".to_string()],
        }])
    );
}

fn cache_def_part_count(wb: &Workbook) -> usize {
    wb.part_names()
        .unwrap()
        .iter()
        .filter(|n| n.contains("pivotCacheDefinition") && n.ends_with(".xml"))
        .count()
}

fn make_pivot_workbook() -> (Workbook, PivotPatch) {
    let mut wb = Workbook::new().unwrap();
    let rows = [
        ("North", "Widget", 100.0),
        ("South", "Widget", 75.0),
        ("North", "Gadget", 50.0),
        ("South", "Gadget", 25.0),
    ];
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Product").unwrap();
    wb.set_value("Sheet1!C1", "Amount").unwrap();
    for (i, (region, product, amount)) in rows.iter().enumerate() {
        let r = i as u32 + 2;
        wb.set_value(format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(format!("Sheet1!B{r}"), *product).unwrap();
        wb.set_value(format!("Sheet1!C{r}"), *amount).unwrap();
    }
    wb.create_sheet("Pivot").unwrap();
    let patch = PivotPatch {
        anchor_cell: "Pivot!A1".to_string(),
        source_ref: "Sheet1!A1:C5".to_string(),
        name: Some("P".to_string()),
        row_fields: vec!["Region".to_string()],
        column_fields: vec!["Product".to_string()],
        filter_fields: vec![],
        data_fields: vec![PivotDataField {
            field: "Amount".to_string(),
            aggregation: PivotAggregation::Sum,
            name: None,
            number_format: None,
        }],
        hidden_items: None,
    };
    (wb, patch)
}

#[test]
fn remove_pivot_drops_orphaned_cache() {
    let (mut wb, patch) = make_pivot_workbook();
    let info = wb.set_pivot("Pivot", patch).unwrap();
    assert_eq!(cache_def_part_count(&wb), 1);
    wb.remove_pivot("Pivot", &info.id).unwrap();
    assert_eq!(
        cache_def_part_count(&wb),
        0,
        "removing the only pivot must drop its cache"
    );
    let bytes = wb.save_bytes().unwrap();
    let names = Workbook::open_bytes(bytes).unwrap().part_names().unwrap();
    assert!(!names.iter().any(|n| n.contains("pivotCache")));
}

#[test]
fn update_pivot_does_not_leak_caches() {
    let (mut wb, patch) = make_pivot_workbook();
    let info = wb.set_pivot("Pivot", patch).unwrap();
    for _ in 0..5 {
        let cur = wb.pivots(Some("Pivot")).unwrap().pop().unwrap();
        wb.update_pivot(
            "Pivot",
            &cur.id,
            PivotUpdate {
                row_fields: Some(vec!["Product".to_string()]),
                ..Default::default()
            },
        )
        .unwrap();
        let cur = wb.pivots(Some("Pivot")).unwrap().pop().unwrap();
        wb.update_pivot(
            "Pivot",
            &cur.id,
            PivotUpdate {
                row_fields: Some(vec!["Region".to_string()]),
                ..Default::default()
            },
        )
        .unwrap();
    }
    let _ = info;
    assert_eq!(wb.pivots(Some("Pivot")).unwrap().len(), 1);
    assert_eq!(
        cache_def_part_count(&wb),
        1,
        "each update reuses a single cache slot, not leak one per edit"
    );
}
