use crate::*;

fn seed(wb: &mut Workbook) {
    let cats = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon", "Zeta"];
    let vals = [120.0, -30.0, 50.0, -20.0, 80.0, 100.0];
    let parents = ["EU", "EU", "EU", "Asia", "Asia", "Asia"];
    for i in 0..6 {
        wb.set_value(&format!("Sheet1!A{}", i + 1), cats[i])
            .unwrap();
        wb.set_value(&format!("Sheet1!B{}", i + 1), vals[i])
            .unwrap();
        wb.set_value(&format!("Sheet1!C{}", i + 1), parents[i])
            .unwrap();
        wb.set_value(&format!("Sheet1!D{}", i + 1), cats[i])
            .unwrap();
        wb.set_value(&format!("Sheet1!E{}", i + 1), vals[i].abs())
            .unwrap();
    }
}

fn base_patch(kind: ChartExKind) -> ChartExPatch {
    ChartExPatch {
        kind,
        title: Some(format!("{kind:?}")),
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 7,
            from_row: 1,
            to_column: 14,
            to_row: 16,
            ..Default::default()
        }),
        categories_ref: Some("Sheet1!$A$1:$A$6".to_string()),
        series: vec![ChartExSeriesPatch {
            name: Some("Value".to_string()),
            values_ref: "Sheet1!$B$1:$B$6".to_string(),
            ..Default::default()
        }],
        legend_position: Some(ChartLegendPosition::Bottom),
        ..Default::default()
    }
}

fn chart_ex_xml(bytes: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut name = None;
    for i in 0..zip.len() {
        let n = zip.by_index(i).unwrap().name().to_string();
        if n.contains("extendedCharts/chart") && n.ends_with(".xml") && !n.contains("_rels") {
            name = Some(n);
            break;
        }
    }
    let name = name.expect("chartEx part");
    use std::io::Read;
    let mut s = String::new();
    zip.by_name(&name).unwrap().read_to_string(&mut s).unwrap();
    s
}

#[test]
fn chart_ex_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    seed(&mut wb);

    let mut patch = base_patch(ChartExKind::Waterfall);
    patch.subtotals = vec![5];
    patch.name = Some("WF".to_string());
    let info = wb.set_chart_ex("Sheet1", patch).unwrap();
    assert_eq!(info.kind, ChartExKind::Waterfall);
    assert_eq!(info.id, "rId1");
    assert_eq!(info.subtotals, vec![5]);

    let listed = wb.chart_exs(Some("Sheet1")).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].kind, ChartExKind::Waterfall);
    assert_eq!(listed[0].title.as_deref(), Some("Waterfall"));
    assert_eq!(
        listed[0].categories_ref.as_deref(),
        Some("Sheet1!$A$1:$A$6")
    );
    assert_eq!(listed[0].series[0].values_ref, "Sheet1!$B$1:$B$6");
    assert_eq!(listed[0].subtotals, vec![5]);
    assert_eq!(listed[0].legend_position, Some(ChartLegendPosition::Bottom));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_ex_xml(&bytes);
    assert!(xml.contains("cx:chartSpace"));
    assert!(xml.contains("layoutId=\"waterfall\""));
    assert!(xml.contains("cx:subtotals"));

    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.chart_exs(Some("Sheet1")).unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].kind, ChartExKind::Waterfall);
    assert_eq!(after[0].subtotals, vec![5]);

    let removed = reopened.remove_chart_ex("Sheet1", "rId1").unwrap();
    assert!(removed.is_some());
    assert!(reopened.chart_exs(Some("Sheet1")).unwrap().is_empty());
}

#[test]
fn chart_ex_update_anchor_persists() {
    let mut wb = Workbook::new().unwrap();
    seed(&mut wb);
    wb.set_chart_ex("Sheet1", base_patch(ChartExKind::Waterfall))
        .unwrap();

    let before = wb.chart_exs(Some("Sheet1")).unwrap()[0].anchor.clone();
    let moved = ChartAnchor {
        from_column: before.from_column + 3,
        from_row: before.from_row + 5,
        to_column: before.to_column + 3,
        to_row: before.to_row + 5,
        ..before.clone()
    };
    wb.update_chart_ex(
        "Sheet1",
        "rId1",
        ChartExUpdate {
            anchor: Some(AnchorSpec::Cells(moved.clone())),
            ..Default::default()
        },
    )
    .unwrap();

    let after = wb.chart_exs(Some("Sheet1")).unwrap()[0].anchor.clone();
    assert_eq!(after.from_column, moved.from_column);
    assert_eq!(after.to_row, moved.to_row);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let roundtrip = reopened.chart_exs(Some("Sheet1")).unwrap()[0].anchor.clone();
    assert_eq!(roundtrip.from_column, moved.from_column);
    assert_eq!(roundtrip.from_row, moved.from_row);
    assert_eq!(roundtrip.to_column, moved.to_column);
    assert_eq!(roundtrip.to_row, moved.to_row);
}

#[test]
fn chart_ex_all_kinds_author_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    seed(&mut wb);

    let kinds = [
        ChartExKind::Waterfall,
        ChartExKind::Funnel,
        ChartExKind::Treemap,
        ChartExKind::Sunburst,
        ChartExKind::Histogram,
        ChartExKind::Pareto,
        ChartExKind::RegionMap,
    ];
    for kind in kinds {
        let mut patch = base_patch(kind);
        if matches!(kind, ChartExKind::Treemap | ChartExKind::Sunburst) {
            patch.categories_ref = Some("Sheet1!$C$1:$D$6".to_string());
            patch.series[0].values_ref = "Sheet1!$E$1:$E$6".to_string();
        }
        if kind == ChartExKind::Histogram {
            patch.categories_ref = None;
            patch.bin_count = Some(5);
        }
        wb.set_chart_ex("Sheet1", patch).unwrap();
    }

    let mut box_patch = base_patch(ChartExKind::BoxWhisker);
    box_patch.series = vec![
        ChartExSeriesPatch {
            name: Some("A".to_string()),
            values_ref: "Sheet1!$B$1:$B$6".to_string(),
            ..Default::default()
        },
        ChartExSeriesPatch {
            name: Some("B".to_string()),
            values_ref: "Sheet1!$E$1:$E$6".to_string(),
            ..Default::default()
        },
    ];
    box_patch.quartile_method = Some(ChartExQuartileMethod::Exclusive);
    wb.set_chart_ex("Sheet1", box_patch).unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let listed = reopened.chart_exs(Some("Sheet1")).unwrap();
    assert_eq!(listed.len(), 8);

    let find = |k: ChartExKind| listed.iter().find(|c| c.kind == k).unwrap();
    assert_eq!(find(ChartExKind::Histogram).bin_count, Some(5));
    let bw = find(ChartExKind::BoxWhisker);
    assert_eq!(bw.quartile_method, Some(ChartExQuartileMethod::Exclusive));
    assert_eq!(bw.series.len(), 2);
    assert_eq!(
        find(ChartExKind::Treemap).categories_ref.as_deref(),
        Some("Sheet1!$C$1:$D$6")
    );
}

#[test]
fn chart_ex_rejects_empty_series() {
    let mut wb = Workbook::new().unwrap();
    seed(&mut wb);
    let mut patch = base_patch(ChartExKind::Funnel);
    patch.series.clear();
    let err = wb.set_chart_ex("Sheet1", patch).unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn chart_ex_rejects_multi_series_non_box() {
    let mut wb = Workbook::new().unwrap();
    seed(&mut wb);
    let mut patch = base_patch(ChartExKind::Waterfall);
    patch.series.push(ChartExSeriesPatch {
        values_ref: "Sheet1!$E$1:$E$6".to_string(),
        ..Default::default()
    });
    let err = wb.set_chart_ex("Sheet1", patch).unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}
