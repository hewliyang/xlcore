use crate::*;

#[test]
fn charts_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Units").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!A3", "South").unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!A4", "East").unwrap();
    wb.set_value("Sheet1!B4", 30.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("Sales".to_string()),
                kind: ChartKind::Column,
                title: Some("Units by Region".to_string()),
                legend_position: Some(ChartLegendPosition::Bottom),
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![ChartSeriesPatch {
                    name: None,
                    name_ref: Some("Sheet1!$B$1".to_string()),
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.sheet, "Sheet1");
    assert_eq!(info.kind, ChartKind::Column);
    assert_eq!(info.name, "Sales");
    assert!(!info.id.is_empty());

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 1);
    let chart = &charts[0];
    assert_eq!(chart.kind, ChartKind::Column);
    assert_eq!(chart.sheet, "Sheet1");
    assert_eq!(chart.title.as_deref(), Some("Units by Region"));
    assert_eq!(chart.legend_position, Some(ChartLegendPosition::Bottom));
    assert_eq!(chart.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));
    assert_eq!(chart.series.len(), 1);
    assert_eq!(chart.series[0].name_ref.as_deref(), Some("Sheet1!$B$1"));
    assert_eq!(chart.series[0].values_ref, "Sheet1!$B$2:$B$4");
    assert_eq!(chart.anchor.from_column, 3);
    assert_eq!(chart.anchor.to_row, 16);

    let removed = reopened.remove_chart("Sheet1", &chart.id).unwrap().unwrap();
    assert_eq!(removed.id, chart.id);
    assert!(reopened.charts(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.charts(None).unwrap().is_empty());
}

fn chart_xml(bytes: &[u8]) -> String {
    use std::io::{Cursor, Read};
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
    let name = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .find(|n| n.contains("/charts/chart") && n.ends_with(".xml") && !n.contains("/_rels/"))
        .expect("chart xml part");
    let mut file = zip.by_name(&name).unwrap();
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    out
}

fn inject_rounded_corners(bytes: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
    let mut buf = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let opts: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).unwrap();
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data).unwrap();
            if name.contains("/charts/chart") && name.ends_with(".xml") && !name.contains("/_rels/")
            {
                let s = String::from_utf8(data).unwrap();
                let s = s.replace("<c:chart>", "<c:roundedCorners val=\"1\"/><c:chart>");
                data = s.into_bytes();
            }
            writer.start_file(name, opts).unwrap();
            writer.write_all(&data).unwrap();
        }
        writer.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn update_chart_preserves_unmodeled_xml_and_stable_id() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("Sales".to_string()),
                kind: ChartKind::Column,
                title: Some("Old".to_string()),
                legend_position: Some(ChartLegendPosition::Bottom),
                categories_ref: Some("Sheet1!$A$2:$A$3".to_string()),
                series: vec![ChartSeriesPatch {
                    name_ref: Some("Sheet1!$B$1".to_string()),
                    values_ref: "Sheet1!$B$2:$B$3".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();

    let bytes = inject_rounded_corners(&wb.save_bytes().unwrap());
    assert!(chart_xml(&bytes).contains("roundedCorners"));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let updated = wb
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                title: Some("New".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(updated.id, info.id);
    assert_eq!(updated.title.as_deref(), Some("New"));
    assert_eq!(updated.legend_position, Some(ChartLegendPosition::Bottom));
    assert_eq!(updated.series.len(), 1);
    assert_eq!(updated.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$3"));

    let out = wb.save_bytes().unwrap();
    let xml = chart_xml(&out);
    assert!(
        xml.contains("roundedCorners"),
        "unmodeled XML must survive update"
    );
    assert!(xml.contains("New"));
    assert!(!xml.contains(">Old<"));

    let mut reopened = Workbook::open_bytes(out).unwrap();
    let charts = reopened.charts(Some("Sheet1")).unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0].id, info.id);
}

#[test]
fn update_chart_replaces_series_and_stacking() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!C2", 5.0).unwrap();
    wb.set_value("Sheet1!C3", 7.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$3".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();

    let updated = wb
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                stacking: Some(ChartStacking::Stacked),
                series: Some(vec![
                    ChartSeriesPatch {
                        values_ref: "Sheet1!$B$2:$B$3".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        values_ref: "Sheet1!$C$2:$C$3".to_string(),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(updated.series.len(), 2);
    assert_eq!(updated.stacking, Some(ChartStacking::Stacked));

    let err = wb
        .update_chart("Sheet1", "rIdMissing", ChartUpdate::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn chart_axis_patch_authors_and_round_trips() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 80.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$3".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: Some(ChartAxisPatch {
                    title: Some("Revenue".to_string()),
                    min: Some(0.0),
                    max: Some(100.0),
                    major_unit: Some(25.0),
                    major_gridlines: Some(true),
                    major_tick_mark: Some(TickMark::Outside),
                    tick_label_position: Some(TickLabelPosition::Low),
                    number_format: Some("#,##0".to_string()),
                    cross_between: Some(CrossBetween::Between),
                    reversed: Some(true),
                    ..Default::default()
                }),
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let chart = reopened
        .charts(Some("Sheet1"))
        .unwrap()
        .into_iter()
        .find(|c| c.id == info.id)
        .unwrap();
    let va = chart.value_axis.expect("value axis round-trips");
    assert_eq!(va.title.as_deref(), Some("Revenue"));
    assert_eq!(va.min, Some(0.0));
    assert_eq!(va.max, Some(100.0));
    assert_eq!(va.major_unit, Some(25.0));
    assert_eq!(va.major_gridlines, Some(true));
    assert_eq!(va.major_tick_mark, Some(TickMark::Outside));
    assert_eq!(va.tick_label_position, Some(TickLabelPosition::Low));
    assert_eq!(va.number_format.as_deref(), Some("#,##0"));
    assert_eq!(va.cross_between, Some(CrossBetween::Between));
    assert_eq!(va.reversed, Some(true));
    assert_eq!(chart.value_axis_title.as_deref(), Some("Revenue"));

    let updated = reopened
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                value_axis: Some(ChartAxisPatch {
                    max: Some(120.0),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let va = updated.value_axis.expect("value axis after update");
    assert_eq!(va.max, Some(120.0));
    assert_eq!(va.min, Some(0.0), "unspecified axis fields preserved");
    assert_eq!(va.major_unit, Some(25.0));
}

#[test]
fn chart_value_axis_display_units_round_trip() {
    use xlcore_types::{BuiltInUnit, DisplayUnits};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 3_000_000.0).unwrap();
    wb.set_value("Sheet1!B3", 8_000_000.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$3".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::A1("D2:K17".to_string()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: Some(ChartAxisPatch {
                    display_units: Some(DisplayUnits::Builtin(BuiltInUnit::Millions)),
                    ..Default::default()
                }),
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let chart = reopened
        .charts(Some("Sheet1"))
        .unwrap()
        .into_iter()
        .find(|c| c.id == info.id)
        .unwrap();
    let va = chart.value_axis.expect("value axis round-trips");
    assert_eq!(
        va.display_units,
        Some(DisplayUnits::Builtin(BuiltInUnit::Millions))
    );

    let updated = reopened
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                value_axis: Some(ChartAxisPatch {
                    display_units: Some(DisplayUnits::Custom(2500.0)),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        updated.value_axis.unwrap().display_units,
        Some(DisplayUnits::Custom(2500.0))
    );
}

#[test]
fn charts_supports_multiple_kinds() {
    let mut wb = Workbook::new().unwrap();
    let patch = |kind: ChartKind| ChartPatch {
        name: None,
        kind,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            name: Some("Series 1".to_string()),
            name_ref: None,
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            x_values_ref: matches!(kind, ChartKind::Scatter | ChartKind::Bubble)
                .then(|| "Sheet1!$A$2:$A$4".to_string()),
            bubble_sizes_ref: matches!(kind, ChartKind::Bubble)
                .then(|| "Sheet1!$C$2:$C$4".to_string()),
            color: None,
            data_labels: None,
            marker: None,
            line: None,
            smooth: None,
            data_points: None,
            kind: None,
            axis: None,
            invert_if_negative: None,
            trendline: None,
            error_bars: None,
        }],
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 1,
            from_row: 1,
            to_column: 5,
            to_row: 10,
            ..Default::default()
        }),
        category_axis_title: None,
        value_axis_title: None,
        category_axis: None,
        value_axis: None,
        stacking: None,
        gap_width: None,
        overlap: None,
        radar_style: None,
        hole_size: None,
        first_slice_angle: None,
        hi_low_lines: None,
        up_down_bars: None,
        drop_lines: None,
        disp_blanks_as: None,
        vary_colors: None,
        data_labels: None,
    };
    for kind in [
        ChartKind::Column,
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Pie,
        ChartKind::Area,
        ChartKind::Scatter,
        ChartKind::Bubble,
        ChartKind::Doughnut,
    ] {
        let info = wb.set_chart("Sheet1", patch(kind)).unwrap();
        assert_eq!(info.kind, kind);
    }
    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 8);
    let kinds: Vec<ChartKind> = charts.iter().map(|c| c.kind).collect();
    assert!(kinds.contains(&ChartKind::Column));
    assert!(kinds.contains(&ChartKind::Bar));
    assert!(kinds.contains(&ChartKind::Line));
    assert!(kinds.contains(&ChartKind::Pie));
    assert!(kinds.contains(&ChartKind::Area));
    assert!(kinds.contains(&ChartKind::Scatter));
    assert!(kinds.contains(&ChartKind::Bubble));
    assert!(kinds.contains(&ChartKind::Doughnut));
}

#[test]
fn charts_scatter_bubble_doughnut_color_and_axis_titles_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    for r in 2..=4 {
        wb.set_value(format!("Sheet1!A{r}").as_str(), (r as f64) * 1.5)
            .unwrap();
        wb.set_value(format!("Sheet1!B{r}").as_str(), (r as f64) * 2.0)
            .unwrap();
        wb.set_value(format!("Sheet1!C{r}").as_str(), (r as f64) * 5.0)
            .unwrap();
    }

    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("Sc".to_string()),
            kind: ChartKind::Scatter,
            title: Some("S".to_string()),
            legend_position: Some(ChartLegendPosition::Right),
            categories_ref: None,
            series: vec![ChartSeriesPatch {
                name: Some("P".to_string()),
                name_ref: None,
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                bubble_sizes_ref: None,
                color: Some("FF8800".to_string()),
                data_labels: None,
                marker: None,
                line: None,
                smooth: None,
                data_points: None,
                kind: None,
                axis: None,
                invert_if_negative: None,
                trendline: None,
                error_bars: None,
            }],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 4,
                from_row: 1,
                to_column: 12,
                to_row: 16,
                ..Default::default()
            }),
            category_axis_title: Some("X-Axis".to_string()),
            value_axis_title: Some("Y-Axis".to_string()),
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    )
    .unwrap();

    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("Bu".to_string()),
            kind: ChartKind::Bubble,
            title: None,
            legend_position: None,
            categories_ref: None,
            series: vec![ChartSeriesPatch {
                name_ref: Some("Sheet1!$B$1".to_string()),
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                bubble_sizes_ref: Some("Sheet1!$C$2:$C$4".to_string()),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 1,
                from_row: 18,
                to_column: 8,
                to_row: 30,
                ..Default::default()
            }),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    )
    .unwrap();

    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("Do".to_string()),
            kind: ChartKind::Doughnut,
            title: None,
            legend_position: Some(ChartLegendPosition::None),
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                color: Some("#00aacc".to_string()),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 9,
                from_row: 18,
                to_column: 16,
                to_row: 30,
                ..Default::default()
            }),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 3);

    let sc = charts
        .iter()
        .find(|c| c.kind == ChartKind::Scatter)
        .unwrap();
    assert_eq!(
        sc.series[0].x_values_ref.as_deref(),
        Some("Sheet1!$A$2:$A$4")
    );
    assert_eq!(sc.series[0].values_ref, "Sheet1!$B$2:$B$4");
    assert_eq!(sc.series[0].color.as_deref(), Some("FF8800"));
    assert_eq!(sc.category_axis_title.as_deref(), Some("X-Axis"));
    assert_eq!(sc.value_axis_title.as_deref(), Some("Y-Axis"));

    let bu = charts.iter().find(|c| c.kind == ChartKind::Bubble).unwrap();
    assert_eq!(
        bu.series[0].x_values_ref.as_deref(),
        Some("Sheet1!$A$2:$A$4")
    );
    assert_eq!(
        bu.series[0].bubble_sizes_ref.as_deref(),
        Some("Sheet1!$C$2:$C$4")
    );

    let dn = charts
        .iter()
        .find(|c| c.kind == ChartKind::Doughnut)
        .unwrap();
    assert_eq!(dn.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));
    assert_eq!(dn.series[0].color.as_deref(), Some("00AACC"));
}

#[test]
fn chart_series_color_accepts_argb_and_strips_alpha() {
    let mut wb = Workbook::new().unwrap();
    for r in 2..=4 {
        wb.set_value(format!("Sheet1!A{r}").as_str(), format!("c{r}").as_str())
            .unwrap();
        wb.set_value(format!("Sheet1!B{r}").as_str(), (r as f64) * 2.0)
            .unwrap();
    }
    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("C".to_string()),
            kind: ChartKind::Column,
            title: None,
            legend_position: None,
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                color: Some("FF1D4ED8".to_string()),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 4,
                from_row: 1,
                to_column: 12,
                to_row: 16,
                ..Default::default()
            }),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts[0].series[0].color.as_deref(), Some("1D4ED8"));
}

#[test]
fn chart_series_color_rejects_malformed_hex() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 1.0).unwrap();
    let err = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$2".to_string(),
                    color: Some("nothex".to_string()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 4,
                    from_row: 1,
                    to_column: 12,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn authored_parts_emit_xml_prolog_and_bound_root_prefix() {
    use std::io::Read;
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "header").unwrap();
    wb.set_value("Sheet1!A2", "row").unwrap();
    wb.create_sheet("Fresh").unwrap();
    wb.set_value("Fresh!B2", 42.0).unwrap();
    wb.set_style(
        "Sheet1!A1",
        StylePatch {
            font: Some(FontPatch {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    wb.set_comment(
        "Sheet1",
        "A1",
        CommentPatch {
            author: Some("a".into()),
            text: "hello".into(),
            ..Default::default()
        },
    )
    .unwrap();
    wb.add_threaded_note(
        "Sheet1",
        "A2",
        ThreadedNotePatch {
            author: Some("a".into()),
            text: "modern".into(),
            ..Default::default()
        },
    )
    .unwrap();
    wb.set_table(
        "Sheet1",
        TablePatch {
            name: "T".into(),
            reference: Some("Sheet1!A1:A2".into()),
            ..Default::default()
        },
    )
    .unwrap();
    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("C".into()),
            kind: ChartKind::Column,
            title: None,
            legend_position: None,
            categories_ref: None,
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$A$1:$A$2".into(),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 3,
                from_row: 1,
                to_column: 9,
                to_row: 12,
                ..Default::default()
            }),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let cursor = std::io::Cursor::new(&bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();

    for name in &names {
        if !name.ends_with(".xml") || name.contains(".rels") {
            continue;
        }
        let mut f = zip.by_name(name).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        assert!(
            buf.starts_with("<?xml "),
            "part {name} missing XML prolog; head: {:?}",
            &buf[..buf.len().min(120)]
        );
        let after_prolog = buf.splitn(2, "?>").nth(1).unwrap_or("").trim_start();
        let lt = after_prolog
            .find('<')
            .expect("no root element after prolog");
        let tag = &after_prolog[lt + 1..];
        let tag_end = tag
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(tag.len());
        let tag_name = &tag[..tag_end];
        if let Some(colon) = tag_name.find(':') {
            let prefix = &tag_name[..colon];
            let needle = format!("xmlns:{prefix}=");
            assert!(
                tag.contains(&needle),
                "root element <{tag_name}> in {name} uses unbound prefix {prefix:?}; root: {:?}",
                &tag[..tag.len().min(240)]
            );
        }
    }
}

#[test]
fn charts_stacking_roundtrips_for_bar_line_area() {
    let mut wb = Workbook::new().unwrap();
    let base = |kind: ChartKind, stacking: Option<ChartStacking>, row: u32| ChartPatch {
        name: None,
        kind,
        title: None,
        legend_position: None,
        categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
        series: vec![
            ChartSeriesPatch {
                name: Some("S1".to_string()),
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            },
            ChartSeriesPatch {
                name: Some("S2".to_string()),
                values_ref: "Sheet1!$C$2:$C$4".to_string(),
                ..Default::default()
            },
        ],
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 1,
            from_row: row,
            to_column: 8,
            to_row: row + 10,
            ..Default::default()
        }),
        category_axis_title: None,
        value_axis_title: None,
        category_axis: None,
        value_axis: None,
        stacking,
        gap_width: None,
        overlap: None,
        radar_style: None,
        hole_size: None,
        first_slice_angle: None,
        hi_low_lines: None,
        up_down_bars: None,
        drop_lines: None,
        disp_blanks_as: None,
        vary_colors: None,
        data_labels: None,
    };

    let col_stacked = wb
        .set_chart(
            "Sheet1",
            base(ChartKind::Column, Some(ChartStacking::Stacked), 1),
        )
        .unwrap();
    assert_eq!(col_stacked.stacking, Some(ChartStacking::Stacked));

    let bar_pct = wb
        .set_chart(
            "Sheet1",
            base(ChartKind::Bar, Some(ChartStacking::PercentStacked), 14),
        )
        .unwrap();
    assert_eq!(bar_pct.stacking, Some(ChartStacking::PercentStacked));

    let line_stacked = wb
        .set_chart(
            "Sheet1",
            base(ChartKind::Line, Some(ChartStacking::Stacked), 28),
        )
        .unwrap();
    assert_eq!(line_stacked.stacking, Some(ChartStacking::Stacked));

    let area_pct = wb
        .set_chart(
            "Sheet1",
            base(ChartKind::Area, Some(ChartStacking::PercentStacked), 42),
        )
        .unwrap();
    assert_eq!(area_pct.stacking, Some(ChartStacking::PercentStacked));

    let col_clustered = wb
        .set_chart(
            "Sheet1",
            base(ChartKind::Column, Some(ChartStacking::Clustered), 56),
        )
        .unwrap();
    assert_eq!(col_clustered.stacking, Some(ChartStacking::Clustered));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    let by_kind = |k: ChartKind, row: u32| -> ChartInfo {
        charts
            .iter()
            .find(|c| c.kind == k && c.anchor.from_row == row)
            .cloned()
            .unwrap_or_else(|| panic!("missing {:?} chart at row {row}", k))
    };
    assert_eq!(
        by_kind(ChartKind::Column, 1).stacking,
        Some(ChartStacking::Stacked)
    );
    assert_eq!(
        by_kind(ChartKind::Bar, 14).stacking,
        Some(ChartStacking::PercentStacked)
    );
    assert_eq!(
        by_kind(ChartKind::Line, 28).stacking,
        Some(ChartStacking::Stacked)
    );
    assert_eq!(
        by_kind(ChartKind::Area, 42).stacking,
        Some(ChartStacking::PercentStacked)
    );
    assert_eq!(
        by_kind(ChartKind::Column, 56).stacking,
        Some(ChartStacking::Clustered)
    );
}

#[test]
fn charts_stacking_on_pie_emits_warning_and_drops() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Pie,
                title: None,
                legend_position: None,
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 1,
                    from_row: 1,
                    to_column: 5,
                    to_row: 10,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: Some(ChartStacking::Stacked),
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.stacking, None);
    let warnings = wb.take_warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, ApiErrorCode::LossyOperation);
    assert!(warnings[0].message.contains("stacking"));
}

#[test]
fn charts_scatter_requires_x_values_and_rejects_bad_color() {
    let mut wb = Workbook::new().unwrap();
    let missing_x = wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: None,
            kind: ChartKind::Scatter,
            title: None,
            legend_position: None,
            categories_ref: None,
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    );
    assert!(missing_x.is_err());

    let bad_color = wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: None,
            kind: ChartKind::Column,
            title: None,
            legend_position: None,
            categories_ref: None,
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                color: Some("nope".to_string()),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: None,
        },
    );
    assert!(bad_color.is_err());
}

#[test]
fn charts_data_labels_roundtrip() {
    use xlcore_types::{ChartDataLabelPosition, ChartDataLabels};
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("WithLabels".to_string()),
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![ChartSeriesPatch {
                    name: Some("S1".to_string()),
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 1,
                    from_row: 1,
                    to_column: 8,
                    to_row: 12,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: Some(ChartDataLabels {
                    show_value: Some(true),
                    show_category_name: Some(false),
                    show_series_name: Some(false),
                    show_percent: None,
                    show_legend_key: Some(false),
                    position: Some(ChartDataLabelPosition::OutsideEnd),
                    separator: Some(", ".to_string()),
                    number_format: Some("0.0%".to_string()),
                    per_point: vec![],
                }),
            },
        )
        .unwrap();
    let dl = info.data_labels.as_ref().expect("data_labels echoed");
    assert_eq!(dl.show_value, Some(true));
    assert_eq!(dl.position, Some(ChartDataLabelPosition::OutsideEnd));

    let bytes = wb.save_bytes().unwrap();
    let mut wb2 = Workbook::open_bytes(bytes).unwrap();
    let charts = wb2.charts(Some("Sheet1")).unwrap();
    assert_eq!(charts.len(), 1);
    let dl = charts[0]
        .data_labels
        .as_ref()
        .expect("data_labels survives reopen");
    assert_eq!(dl.show_value, Some(true));
    assert_eq!(dl.show_category_name, Some(false));
    assert_eq!(dl.position, Some(ChartDataLabelPosition::OutsideEnd));
    assert_eq!(dl.separator.as_deref(), Some(", "));
    assert_eq!(dl.number_format.as_deref(), Some("0.0%"));
}

#[test]
fn charts_data_labels_pie_show_percent_roundtrip() {
    use xlcore_types::{ChartDataLabelPosition, ChartDataLabels};
    let mut wb = Workbook::new().unwrap();
    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("Pie".to_string()),
            kind: ChartKind::Pie,
            title: None,
            legend_position: None,
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            }],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 6,
                to_row: 10,
                ..Default::default()
            }),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: Some(ChartDataLabels {
                show_percent: Some(true),
                show_category_name: Some(true),
                position: Some(ChartDataLabelPosition::Center),
                ..Default::default()
            }),
        },
    )
    .unwrap();
    let bytes = wb.save_bytes().unwrap();
    let mut wb2 = Workbook::open_bytes(bytes).unwrap();
    let charts = wb2.charts(Some("Sheet1")).unwrap();
    let dl = charts[0].data_labels.as_ref().unwrap();
    assert_eq!(dl.show_percent, Some(true));
    assert_eq!(dl.show_category_name, Some(true));
    assert_eq!(dl.position, Some(ChartDataLabelPosition::Center));
}

#[test]
fn charts_per_series_data_labels_override_chart_level() {
    use xlcore_types::{ChartDataLabelPosition, ChartDataLabels};
    let mut wb = Workbook::new().unwrap();
    wb.set_chart(
        "Sheet1",
        ChartPatch {
            name: Some("PerSeries".to_string()),
            kind: ChartKind::Column,
            title: None,
            legend_position: None,
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![
                ChartSeriesPatch {
                    name: Some("A".to_string()),
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    data_labels: Some(ChartDataLabels {
                        show_value: Some(true),
                        position: Some(ChartDataLabelPosition::OutsideEnd),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ChartSeriesPatch {
                    name: Some("B".to_string()),
                    values_ref: "Sheet1!$C$2:$C$4".to_string(),
                    ..Default::default()
                },
            ],
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 8,
                to_row: 12,
                ..Default::default()
            }),
            category_axis_title: None,
            value_axis_title: None,
            category_axis: None,
            value_axis: None,
            stacking: None,
            gap_width: None,
            overlap: None,
            radar_style: None,
            hole_size: None,
            first_slice_angle: None,
            hi_low_lines: None,
            up_down_bars: None,
            drop_lines: None,
            disp_blanks_as: None,
            vary_colors: None,
            data_labels: Some(ChartDataLabels {
                show_series_name: Some(true),
                position: Some(ChartDataLabelPosition::Center),
                ..Default::default()
            }),
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut wb2 = Workbook::open_bytes(bytes).unwrap();
    let charts = wb2.charts(Some("Sheet1")).unwrap();
    assert_eq!(charts.len(), 1);
    let info = &charts[0];
    assert_eq!(info.series.len(), 2);

    let s0 = info.series[0]
        .data_labels
        .as_ref()
        .expect("series 0 has dl");
    assert_eq!(s0.show_value, Some(true));
    assert_eq!(s0.position, Some(ChartDataLabelPosition::OutsideEnd));

    assert!(
        info.series[1].data_labels.is_none(),
        "series without explicit dl should not echo chart-level dl",
    );

    let chart_dl = info.data_labels.as_ref().expect("chart-level dl preserved");
    assert_eq!(chart_dl.show_series_name, Some(true));
    assert_eq!(chart_dl.position, Some(ChartDataLabelPosition::Center));
}

#[test]
fn charts_per_point_data_labels_roundtrip_and_update() {
    use xlcore_types::{ChartDataLabel, ChartDataLabelPosition, ChartDataLabels};
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("PerPoint".to_string()),
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: Some("Sheet1!$A$2:$A$5".to_string()),
                series: vec![ChartSeriesPatch {
                    name: Some("S1".to_string()),
                    values_ref: "Sheet1!$B$2:$B$5".to_string(),
                    data_labels: Some(ChartDataLabels {
                        show_value: Some(true),
                        per_point: vec![
                            ChartDataLabel {
                                index: 1,
                                delete: true,
                                ..Default::default()
                            },
                            ChartDataLabel {
                                index: 3,
                                show_value: Some(true),
                                show_category_name: Some(true),
                                position: Some(ChartDataLabelPosition::InsideEnd),
                                number_format: Some("0.00".to_string()),
                                ..Default::default()
                            },
                        ],
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 1,
                    from_row: 1,
                    to_column: 8,
                    to_row: 12,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("<c:dLbl>"), "per-point dLbl emitted");
    assert!(
        xml.contains("<c:idx val=\"1\" /><c:delete val=\"1\" />"),
        "delete flag emitted for point 1",
    );
    assert!(xml.contains("<c:dLblPos val=\"inEnd\" />"), "per-point position");

    let mut wb2 = Workbook::open_bytes(bytes).unwrap();
    let charts = wb2.charts(Some("Sheet1")).unwrap();
    let dl = charts[0].series[0]
        .data_labels
        .as_ref()
        .expect("series dl survives reopen");
    assert_eq!(dl.per_point.len(), 2);
    let p1 = dl.per_point.iter().find(|p| p.index == 1).unwrap();
    assert!(p1.delete);
    let p3 = dl.per_point.iter().find(|p| p.index == 3).unwrap();
    assert!(!p3.delete);
    assert_eq!(p3.show_value, Some(true));
    assert_eq!(p3.show_category_name, Some(true));
    assert_eq!(p3.position, Some(ChartDataLabelPosition::InsideEnd));
    assert_eq!(p3.number_format.as_deref(), Some("0.00"));

    let updated = wb2
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                series: Some(vec![ChartSeriesPatch {
                    name: Some("S1".to_string()),
                    values_ref: "Sheet1!$B$2:$B$5".to_string(),
                    data_labels: Some(ChartDataLabels {
                        show_value: Some(true),
                        per_point: vec![ChartDataLabel {
                            index: 0,
                            show_percent: Some(true),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        )
        .unwrap();
    let dl = updated.series[0].data_labels.as_ref().unwrap();
    assert_eq!(dl.per_point.len(), 1);
    assert_eq!(dl.per_point[0].index, 0);
    assert_eq!(dl.per_point[0].show_percent, Some(true));
}

#[test]
fn chart_gap_width_overlap_roundtrip_and_update() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$3".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: Some(219),
                overlap: Some(-27),
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.gap_width, Some(219));
    assert_eq!(info.overlap, Some(-27));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("gapWidth"));
    assert!(xml.contains("overlap"));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].gap_width, Some(219));
    assert_eq!(read[0].overlap, Some(-27));

    let updated = wb
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                gap_width: Some(50),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.gap_width, Some(50));
    assert_eq!(updated.overlap, Some(-27), "unspecified overlap preserved");

    let err = wb
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                gap_width: Some(999),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn chart_series_marker_roundtrip_and_validation() {
    use xlcore_types::{ChartMarker, MarkerStyle};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Line,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    marker: Some(ChartMarker {
                        style: Some(MarkerStyle::Diamond),
                        size: Some(9),
                    }),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(
        info.series[0].marker,
        Some(ChartMarker {
            style: Some(MarkerStyle::Diamond),
            size: Some(9)
        })
    );

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:marker"));
    assert!(xml.contains("diamond"));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(
        read[0].series[0].marker,
        Some(ChartMarker {
            style: Some(MarkerStyle::Diamond),
            size: Some(9)
        })
    );

    let err = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Line,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    marker: Some(ChartMarker {
                        style: None,
                        size: Some(100),
                    }),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn chart_series_smooth_roundtrips_and_sets_scatter_style() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A2", 1.0).unwrap();
    wb.set_value("Sheet1!A3", 2.0).unwrap();
    wb.set_value("Sheet1!A4", 3.0).unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Scatter,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                    smooth: Some(true),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series[0].smooth, Some(true));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("smoothMarker"));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].series[0].smooth, Some(true));
}

#[test]
fn chart_data_point_fills_roundtrip_and_validation() {
    use xlcore_types::ChartDataPoint;

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", -5.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let points = vec![
        ChartDataPoint {
            index: 0,
            fill: Some("FF0000".to_string()),
            explosion: None,
        },
        ChartDataPoint {
            index: 1,
            fill: Some("none".to_string()),
            explosion: None,
        },
    ];
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    data_points: Some(points.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(
        info.series[0].data_points.as_deref(),
        Some(points.as_slice())
    );

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:dPt"));
    assert!(xml.contains("FF0000"));
    assert!(xml.contains("noFill"));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(
        read[0].series[0].data_points.as_deref(),
        Some(points.as_slice())
    );

    let err = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    data_points: Some(vec![ChartDataPoint {
                        index: 0,
                        fill: Some("nope".to_string()),
                        explosion: None,
                    }]),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn chart_data_point_explosion_roundtrip() {
    use xlcore_types::ChartDataPoint;

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 30.0).unwrap();
    wb.set_value("Sheet1!B3", 50.0).unwrap();
    wb.set_value("Sheet1!B4", 20.0).unwrap();

    let points = vec![ChartDataPoint {
        index: 0,
        fill: None,
        explosion: Some(25),
    }];
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Pie,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    data_points: Some(points.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(
        info.series[0].data_points.as_deref(),
        Some(points.as_slice())
    );

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:explosion"));
    assert!(xml.contains("val=\"25\""));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(
        read[0].series[0].data_points.as_deref(),
        Some(points.as_slice())
    );
}

#[test]
fn combo_chart_secondary_axis_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    for (i, (region, units, margin)) in [
        ("North", 100.0, 0.12),
        ("South", 200.0, 0.18),
        ("East", 150.0, 0.09),
    ]
    .iter()
    .enumerate()
    {
        let r = i + 2;
        wb.set_value(&format!("Sheet1!A{r}"), *region).unwrap();
        wb.set_value(&format!("Sheet1!B{r}"), *units).unwrap();
        wb.set_value(&format!("Sheet1!C{r}"), *margin).unwrap();
    }
    wb.set_value("Sheet1!B1", "Units").unwrap();
    wb.set_value("Sheet1!C1", "Margin").unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("Combo".to_string()),
                kind: ChartKind::Column,
                title: Some("Units vs Margin".to_string()),
                legend_position: Some(ChartLegendPosition::Bottom),
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$B$1".to_string()),
                        values_ref: "Sheet1!$B$2:$B$4".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$C$1".to_string()),
                        values_ref: "Sheet1!$C$2:$C$4".to_string(),
                        kind: Some(ChartKind::Line),
                        axis: Some(ChartAxisGroup::Secondary),
                        ..Default::default()
                    },
                ],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series.len(), 2);
    assert_eq!(info.series[1].kind, Some(ChartKind::Line));
    assert_eq!(info.series[1].axis, Some(ChartAxisGroup::Secondary));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 1);
    let chart = &charts[0];
    assert_eq!(chart.kind, ChartKind::Column);
    assert_eq!(chart.series.len(), 2);
    assert_eq!(chart.series[0].kind, Some(ChartKind::Column));
    assert_eq!(chart.series[0].axis, None);
    assert_eq!(chart.series[1].kind, Some(ChartKind::Line));
    assert_eq!(chart.series[1].axis, Some(ChartAxisGroup::Secondary));

    let err = reopened
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Pie,
                title: None,
                legend_position: None,
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    axis: Some(ChartAxisGroup::Secondary),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidChart);
}

#[test]
fn radar_chart_build_roundtrip_and_update() {
    let mut wb = Workbook::new().unwrap();
    for (i, (cat, v1, v2)) in [
        ("Speed", 8.0, 6.0),
        ("Power", 5.0, 9.0),
        ("Range", 7.0, 4.0),
    ]
    .iter()
    .enumerate()
    {
        let row = i + 2;
        wb.set_value(&format!("Sheet1!A{row}"), *cat).unwrap();
        wb.set_value(&format!("Sheet1!B{row}"), *v1).unwrap();
        wb.set_value(&format!("Sheet1!C{row}"), *v2).unwrap();
    }
    wb.set_value("Sheet1!B1", "Car A").unwrap();
    wb.set_value("Sheet1!C1", "Car B").unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("Radar".to_string()),
                kind: ChartKind::Radar,
                title: Some("Comparison".to_string()),
                legend_position: Some(ChartLegendPosition::Right),
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$B$1".to_string()),
                        values_ref: "Sheet1!$B$2:$B$4".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$C$1".to_string()),
                        values_ref: "Sheet1!$C$2:$C$4".to_string(),
                        ..Default::default()
                    },
                ],
                anchor: AnchorSpec::A1("Sheet1!E1:M16".to_string()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: Some(RadarStyle::Marker),
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.kind, ChartKind::Radar);
    assert_eq!(info.radar_style, Some(RadarStyle::Marker));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 1);
    let chart = &charts[0];
    assert_eq!(chart.kind, ChartKind::Radar);
    assert_eq!(chart.radar_style, Some(RadarStyle::Marker));
    assert_eq!(chart.series.len(), 2);
    assert_eq!(chart.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));

    reopened
        .update_chart(
            "Sheet1",
            &chart.id,
            ChartUpdate {
                radar_style: Some(RadarStyle::Filled),
                hole_size: None,
                first_slice_angle: None,
                ..Default::default()
            },
        )
        .unwrap();
    let chart = reopened.charts(None).unwrap().into_iter().next().unwrap();
    assert_eq!(chart.radar_style, Some(RadarStyle::Filled));
    assert_eq!(chart.series.len(), 2);
}

#[test]
fn doughnut_hole_size_and_first_slice_angle_roundtrip_and_update() {
    let mut wb = Workbook::new().unwrap();
    for (i, (cat, v)) in [("A", 30.0), ("B", 20.0), ("C", 50.0)].iter().enumerate() {
        let row = i + 2;
        wb.set_value(&format!("Sheet1!A{row}"), *cat).unwrap();
        wb.set_value(&format!("Sheet1!B{row}"), *v).unwrap();
    }
    wb.set_value("Sheet1!B1", "Share").unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("Donut".to_string()),
                kind: ChartKind::Doughnut,
                title: None,
                legend_position: None,
                categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                series: vec![ChartSeriesPatch {
                    name_ref: Some("Sheet1!$B$1".to_string()),
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::A1("Sheet1!D1:K16".to_string()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: Some(70),
                first_slice_angle: Some(90),
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.hole_size, Some(70));
    assert_eq!(info.first_slice_angle, Some(90));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let chart = reopened.charts(None).unwrap().into_iter().next().unwrap();
    assert_eq!(chart.kind, ChartKind::Doughnut);
    assert_eq!(chart.hole_size, Some(70));
    assert_eq!(chart.first_slice_angle, Some(90));

    reopened
        .update_chart(
            "Sheet1",
            &chart.id,
            ChartUpdate {
                hole_size: Some(25),
                first_slice_angle: Some(180),
                ..Default::default()
            },
        )
        .unwrap();
    let chart = reopened.charts(None).unwrap().into_iter().next().unwrap();
    assert_eq!(chart.hole_size, Some(25));
    assert_eq!(chart.first_slice_angle, Some(180));
    assert_eq!(chart.series.len(), 1);
}

#[test]
fn doughnut_hole_size_out_of_range_rejected() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B1", "Share").unwrap();
    wb.set_value("Sheet1!B2", 1.0).unwrap();
    let err = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Doughnut,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::A1("Sheet1!D1:K16".to_string()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: Some(5),
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap_err();
    assert!(err.to_string().contains("hole_size"));
}

#[test]
fn stock_chart_build_roundtrip_and_update() {
    let mut wb = Workbook::new().unwrap();
    let rows = [
        ("Mon", 12.0, 15.0, 10.0, 14.0),
        ("Tue", 14.0, 16.0, 13.0, 13.0),
        ("Wed", 13.0, 14.0, 11.0, 12.0),
        ("Thu", 12.0, 18.0, 12.0, 17.0),
    ];
    for (i, (cat, o, h, l, c)) in rows.iter().enumerate() {
        let row = i + 2;
        wb.set_value(&format!("Sheet1!A{row}"), *cat).unwrap();
        wb.set_value(&format!("Sheet1!B{row}"), *o).unwrap();
        wb.set_value(&format!("Sheet1!C{row}"), *h).unwrap();
        wb.set_value(&format!("Sheet1!D{row}"), *l).unwrap();
        wb.set_value(&format!("Sheet1!E{row}"), *c).unwrap();
    }
    for (col, name) in [("B", "Open"), ("C", "High"), ("D", "Low"), ("E", "Close")] {
        wb.set_value(&format!("Sheet1!{col}1"), name).unwrap();
    }

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: Some("Stock".to_string()),
                kind: ChartKind::Stock,
                title: Some("OHLC".to_string()),
                legend_position: None,
                categories_ref: Some("Sheet1!$A$2:$A$5".to_string()),
                series: vec![
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$B$1".to_string()),
                        values_ref: "Sheet1!$B$2:$B$5".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$C$1".to_string()),
                        values_ref: "Sheet1!$C$2:$C$5".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$D$1".to_string()),
                        values_ref: "Sheet1!$D$2:$D$5".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        name_ref: Some("Sheet1!$E$1".to_string()),
                        values_ref: "Sheet1!$E$2:$E$5".to_string(),
                        ..Default::default()
                    },
                ],
                anchor: AnchorSpec::A1("Sheet1!G1:O16".to_string()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.kind, ChartKind::Stock);
    assert_eq!(info.hi_low_lines, Some(true));
    assert_eq!(info.up_down_bars, Some(true));
    assert_eq!(info.drop_lines, Some(false));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 1);
    let chart = &charts[0];
    assert_eq!(chart.kind, ChartKind::Stock);
    assert_eq!(chart.series.len(), 4);
    assert_eq!(chart.hi_low_lines, Some(true));
    assert_eq!(chart.up_down_bars, Some(true));
    assert_eq!(chart.drop_lines, Some(false));
    assert_eq!(chart.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$5"));

    reopened
        .update_chart(
            "Sheet1",
            &chart.id,
            ChartUpdate {
                up_down_bars: Some(false),
                drop_lines: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    let chart = reopened.charts(None).unwrap().into_iter().next().unwrap();
    assert_eq!(chart.kind, ChartKind::Stock);
    assert_eq!(chart.hi_low_lines, Some(true));
    assert_eq!(chart.up_down_bars, Some(false));
    assert_eq!(chart.drop_lines, Some(true));
    assert_eq!(chart.series.len(), 4);
}

#[test]
fn stock_chart_requires_three_to_six_series() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 1.0).unwrap();
    wb.set_value("Sheet1!C2", 2.0).unwrap();
    let err = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Stock,
                series: vec![
                    ChartSeriesPatch {
                        values_ref: "Sheet1!$B$2".to_string(),
                        ..Default::default()
                    },
                    ChartSeriesPatch {
                        values_ref: "Sheet1!$C$2".to_string(),
                        ..Default::default()
                    },
                ],
                title: None,
                legend_position: None,
                categories_ref: None,
                anchor: AnchorSpec::A1("Sheet1!E1:K16".to_string()),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap_err();
    assert!(err.to_string().contains("3..=6 series"));
}

#[test]
fn chart_series_line_roundtrips_and_updates() {
    use xlcore_types::{ChartLine, ChartUpdate, LineDash};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let line = ChartLine {
        width_emu: Some(38100),
        dash: Some(LineDash::Dash),
        none: None,
    };
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Line,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    line: Some(line.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series[0].line, Some(line.clone()));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("<a:ln"));
    assert!(xml.contains("w=\"38100\""));
    assert!(xml.contains("prstDash"));
    assert!(xml.contains("\"dash\""));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let id = wb.charts(Some("Sheet1")).unwrap()[0].id.clone();
    assert_eq!(
        wb.charts(Some("Sheet1")).unwrap()[0].series[0].line,
        Some(line)
    );

    let updated = ChartLine {
        width_emu: Some(12700),
        dash: Some(LineDash::SystemDot),
        none: None,
    };
    wb.update_chart(
        "Sheet1",
        id,
        ChartUpdate {
            series: Some(vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                line: Some(updated.clone()),
                ..Default::default()
            }]),
            ..Default::default()
        },
    )
    .unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].series[0].line, Some(updated));
}

#[test]
fn disp_blanks_as_builds_reads_updates() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Line,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    name: Some("S".to_string()),
                    values_ref: "Sheet1!$B$2:$B$3".to_string(),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: Some(DispBlanksAs::Span),
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.disp_blanks_as, Some(DispBlanksAs::Span));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("<c:dispBlanksAs val=\"span\" />"), "{xml}");

    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let read = reopened.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].disp_blanks_as, Some(DispBlanksAs::Span));

    reopened
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                disp_blanks_as: Some(DispBlanksAs::Zero),
                vary_colors: None,
                ..Default::default()
            },
        )
        .unwrap();
    let updated = reopened.charts(Some("Sheet1")).unwrap();
    assert_eq!(updated[0].disp_blanks_as, Some(DispBlanksAs::Zero));
    let xml2 = chart_xml(&reopened.save_bytes().unwrap());
    assert!(xml2.contains("<c:dispBlanksAs val=\"zero\" />"), "{xml2}");
}

#[test]
fn vary_colors_and_invert_if_negative_roundtrip_and_update() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", -20.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    invert_if_negative: Some(true),
                    ..Default::default()
                }],
                anchor: AnchorSpec::Cells(ChartAnchor {
                    from_column: 3,
                    from_row: 1,
                    to_column: 10,
                    to_row: 16,
                    ..Default::default()
                }),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: Some(true),
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.vary_colors, Some(true));
    assert_eq!(info.series[0].invert_if_negative, Some(true));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:varyColors"), "{xml}");
    assert!(xml.contains("c:invertIfNegative"), "{xml}");

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].vary_colors, Some(true));
    assert_eq!(read[0].series[0].invert_if_negative, Some(true));

    let updated = wb
        .update_chart(
            "Sheet1",
            &info.id,
            ChartUpdate {
                vary_colors: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.vary_colors, Some(false));
    assert_eq!(
        updated.series[0].invert_if_negative,
        Some(true),
        "invert preserved across update"
    );

    let xml2 = chart_xml(&wb.save_bytes().unwrap());
    assert!(
        xml2.contains("<c:varyColors val=\"false\" />") || xml2.contains("varyColors val=\"0\""),
        "{xml2}"
    );
}

#[test]
fn chart_trendline_roundtrips_and_updates() {
    use xlcore_types::{ChartTrendline, TrendlineKind};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 12.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let trend = ChartTrendline {
        kind: TrendlineKind::Linear,
        name: Some("Fit".to_string()),
        polynomial_order: None,
        period: None,
        forward: Some(2.0),
        backward: Some(1.0),
        intercept: Some(0.0),
        display_equation: Some(true),
        display_r_squared: Some(true),
    };
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    trendline: Some(trend.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series[0].trendline.as_ref(), Some(&trend));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:trendline"));
    assert!(xml.contains("c:trendlineType val=\"linear\""));
    assert!(xml.contains("c:dispEq"));
    assert!(xml.contains("c:dispRSqr"));
    assert!(xml.contains("c:forward val=\"2\""));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].series[0].trendline.as_ref(), Some(&trend));

    let id = read[0].id.clone();
    let poly = ChartTrendline {
        kind: TrendlineKind::Polynomial,
        name: None,
        polynomial_order: Some(3),
        period: None,
        forward: None,
        backward: None,
        intercept: None,
        display_equation: None,
        display_r_squared: Some(true),
    };
    let updated = wb
        .update_chart(
            "Sheet1",
            &id,
            ChartUpdate {
                series: Some(vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    trendline: Some(poly.clone()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.series[0].trendline.as_ref(), Some(&poly));

    let xml2 = chart_xml(&wb.save_bytes().unwrap());
    assert!(xml2.contains("c:trendlineType val=\"poly\""));
    assert!(xml2.contains("c:order val=\"3\""));
}

#[test]
fn chart_trendline_moving_average_on_scatter_roundtrips() {
    use xlcore_types::{ChartTrendline, TrendlineKind};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A2", 1.0).unwrap();
    wb.set_value("Sheet1!A3", 2.0).unwrap();
    wb.set_value("Sheet1!A4", 3.0).unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 12.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let trend = ChartTrendline {
        kind: TrendlineKind::MovingAverage,
        name: None,
        polynomial_order: None,
        period: Some(2),
        forward: None,
        backward: None,
        intercept: None,
        display_equation: None,
        display_r_squared: None,
    };
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Scatter,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                    trendline: Some(trend.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series[0].trendline.as_ref(), Some(&trend));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:trendlineType val=\"movingAvg\""));
    assert!(xml.contains("c:period val=\"2\""));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].series[0].trendline.as_ref(), Some(&trend));
}

#[test]
fn chart_trendline_rejected_on_pie_and_bad_params() {
    use xlcore_types::{ChartTrendline, TrendlineKind};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 12.0).unwrap();

    let mk = |kind: ChartKind, trend: ChartTrendline| ChartPatch {
        name: None,
        kind,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$B$2:$B$3".to_string(),
            trendline: Some(trend),
            ..Default::default()
        }],
        anchor: AnchorSpec::default(),
        category_axis_title: None,
        value_axis_title: None,
        category_axis: None,
        value_axis: None,
        stacking: None,
        gap_width: None,
        overlap: None,
        radar_style: None,
        hole_size: None,
        first_slice_angle: None,
        hi_low_lines: None,
        up_down_bars: None,
        drop_lines: None,
        disp_blanks_as: None,
        vary_colors: None,
        data_labels: None,
    };

    let linear = ChartTrendline {
        kind: TrendlineKind::Linear,
        name: None,
        polynomial_order: None,
        period: None,
        forward: None,
        backward: None,
        intercept: None,
        display_equation: None,
        display_r_squared: None,
    };
    assert!(wb
        .set_chart("Sheet1", mk(ChartKind::Pie, linear.clone()))
        .is_err());

    let bad_order = ChartTrendline {
        kind: TrendlineKind::Polynomial,
        polynomial_order: Some(7),
        ..linear.clone()
    };
    assert!(wb
        .set_chart("Sheet1", mk(ChartKind::Column, bad_order))
        .is_err());

    let bad_period = ChartTrendline {
        kind: TrendlineKind::MovingAverage,
        period: Some(1),
        ..linear
    };
    assert!(wb
        .set_chart("Sheet1", mk(ChartKind::Column, bad_period))
        .is_err());
}

#[test]
fn chart_error_bars_fixed_roundtrips_and_updates() {
    use xlcore_types::{ChartErrorBarType, ChartErrorBars, ChartErrorValueType};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 12.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();

    let eb = ChartErrorBars {
        direction: None,
        bar_type: ChartErrorBarType::Both,
        value_type: ChartErrorValueType::FixedValue,
        value: Some(1.5),
        no_end_cap: Some(true),
        plus_ref: None,
        minus_ref: None,
        plus_values: None,
        minus_values: None,
    };
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Column,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    error_bars: Some(eb.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series[0].error_bars.as_ref(), Some(&eb));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:errBars"));
    assert!(xml.contains("c:errBarType val=\"both\""));
    assert!(xml.contains("c:errValType val=\"fixedVal\""));
    assert!(xml.contains("c:val val=\"1.5\""));
    assert!(xml.contains("c:noEndCap val=\"1\""));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].series[0].error_bars.as_ref(), Some(&eb));

    let id = read[0].id.clone();
    let pct = ChartErrorBars {
        direction: None,
        bar_type: ChartErrorBarType::Plus,
        value_type: ChartErrorValueType::Percentage,
        value: Some(5.0),
        no_end_cap: None,
        plus_ref: None,
        minus_ref: None,
        plus_values: None,
        minus_values: None,
    };
    let updated = wb
        .update_chart(
            "Sheet1",
            &id,
            ChartUpdate {
                series: Some(vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    error_bars: Some(pct.clone()),
                    ..Default::default()
                }]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.series[0].error_bars.as_ref(), Some(&pct));

    let xml2 = chart_xml(&wb.save_bytes().unwrap());
    assert!(xml2.contains("c:errBarType val=\"plus\""));
    assert!(xml2.contains("c:errValType val=\"percentage\""));
    assert!(!xml2.contains("c:noEndCap"));
}

#[test]
fn chart_error_bars_custom_on_scatter_roundtrips() {
    use xlcore_types::{
        ChartErrorBarType, ChartErrorBars, ChartErrorDirection, ChartErrorValueType,
    };

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A2", 1.0).unwrap();
    wb.set_value("Sheet1!A3", 2.0).unwrap();
    wb.set_value("Sheet1!A4", 3.0).unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 12.0).unwrap();
    wb.set_value("Sheet1!B4", 15.0).unwrap();
    wb.set_value("Sheet1!C2", 0.5).unwrap();
    wb.set_value("Sheet1!C3", 0.4).unwrap();
    wb.set_value("Sheet1!C4", 0.3).unwrap();

    let eb = ChartErrorBars {
        direction: Some(ChartErrorDirection::Y),
        bar_type: ChartErrorBarType::Both,
        value_type: ChartErrorValueType::Custom,
        value: None,
        no_end_cap: None,
        plus_ref: Some("Sheet1!$C$2:$C$4".to_string()),
        minus_ref: None,
        plus_values: None,
        minus_values: Some(vec![0.1, 0.2, 0.3]),
    };
    let info = wb
        .set_chart(
            "Sheet1",
            ChartPatch {
                name: None,
                kind: ChartKind::Scatter,
                title: None,
                legend_position: None,
                categories_ref: None,
                series: vec![ChartSeriesPatch {
                    values_ref: "Sheet1!$B$2:$B$4".to_string(),
                    x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
                    error_bars: Some(eb.clone()),
                    ..Default::default()
                }],
                anchor: AnchorSpec::default(),
                category_axis_title: None,
                value_axis_title: None,
                category_axis: None,
                value_axis: None,
                stacking: None,
                gap_width: None,
                overlap: None,
                radar_style: None,
                hole_size: None,
                first_slice_angle: None,
                hi_low_lines: None,
                up_down_bars: None,
                drop_lines: None,
                disp_blanks_as: None,
                vary_colors: None,
                data_labels: None,
            },
        )
        .unwrap();
    assert_eq!(info.series[0].error_bars.as_ref(), Some(&eb));

    let bytes = wb.save_bytes().unwrap();
    let xml = chart_xml(&bytes);
    assert!(xml.contains("c:errDir val=\"y\""));
    assert!(xml.contains("c:errValType val=\"cust\""));
    assert!(xml.contains("c:plus"));
    assert!(xml.contains("Sheet1!$C$2:$C$4"));
    assert!(xml.contains("c:minus"));
    assert!(xml.contains("c:numLit"));

    let mut wb = Workbook::open_bytes(bytes).unwrap();
    let read = wb.charts(Some("Sheet1")).unwrap();
    assert_eq!(read[0].series[0].error_bars.as_ref(), Some(&eb));
}

#[test]
fn chart_error_bars_rejected_on_pie_and_bad_params() {
    use xlcore_types::{ChartErrorBarType, ChartErrorBars, ChartErrorValueType};

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!B3", 12.0).unwrap();

    let mk = |kind: ChartKind, eb: ChartErrorBars| ChartPatch {
        name: None,
        kind,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$B$2:$B$3".to_string(),
            error_bars: Some(eb),
            ..Default::default()
        }],
        anchor: AnchorSpec::default(),
        category_axis_title: None,
        value_axis_title: None,
        category_axis: None,
        value_axis: None,
        stacking: None,
        gap_width: None,
        overlap: None,
        radar_style: None,
        hole_size: None,
        first_slice_angle: None,
        hi_low_lines: None,
        up_down_bars: None,
        drop_lines: None,
        disp_blanks_as: None,
        vary_colors: None,
        data_labels: None,
    };

    let fixed = ChartErrorBars {
        direction: None,
        bar_type: ChartErrorBarType::Both,
        value_type: ChartErrorValueType::FixedValue,
        value: Some(1.0),
        no_end_cap: None,
        plus_ref: None,
        minus_ref: None,
        plus_values: None,
        minus_values: None,
    };
    assert!(wb
        .set_chart("Sheet1", mk(ChartKind::Pie, fixed.clone()))
        .is_err());

    let no_value = ChartErrorBars {
        value: None,
        ..fixed.clone()
    };
    assert!(wb
        .set_chart("Sheet1", mk(ChartKind::Column, no_value))
        .is_err());

    let custom_empty = ChartErrorBars {
        value_type: ChartErrorValueType::Custom,
        value: None,
        ..fixed
    };
    assert!(wb
        .set_chart("Sheet1", mk(ChartKind::Column, custom_empty))
        .is_err());
}
