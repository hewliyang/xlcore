use crate::*;

#[test]
fn conditional_format_add_list_remove_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 5.0).unwrap();
    workbook.set_value("Sheet1!A2", 10.0).unwrap();

    workbook
        .set_conditional_format("Sheet1", "A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::CellIs,
                operator: Some(CfOperator::GreaterThan),
                formula1: Some("7".into()),
                dxf: Some(StylePatch {
                    fill: Some(FillPatch {
                        color: Some("#FFEB3B".into()),
                    }),
                    font: Some(FontPatch {
                        bold: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    workbook
        .set_conditional_format("Sheet1", "A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::Expression,
                formula1: Some("MOD(ROW(),2)=0".into()),
                ..Default::default()
            },
        )
        .unwrap();

    let listed = workbook.conditional_formats("Sheet1").unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].kind, CfRuleKind::CellIs);
    assert_eq!(listed[0].operator, Some(CfOperator::GreaterThan));
    assert_eq!(listed[0].formula1.as_deref(), Some("7"));
    assert!(listed[0].dxf_id.is_some());
    assert_eq!(listed[1].kind, CfRuleKind::Expression);
    assert!(listed[1].priority > listed[0].priority);

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.conditional_formats("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(after[0].formula1.as_deref(), Some("7"));
    assert!(after[0].dxf_id.is_some());

    let removed = reopened.clear_conditional_formats("Sheet1", "A1:A10").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(reopened.conditional_formats("Sheet1").unwrap().is_empty());
}

#[test]
fn conditional_format_rejects_missing_formula() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_conditional_format("Sheet1", "A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::CellIs,
                operator: Some(CfOperator::Equal),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidConditionalFormat);
}

#[test]
fn conditional_format_color_scale_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_conditional_format("Sheet1", "A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::ColorScale,
                color_scale: Some(ColorScalePatch {
                    values: vec![
                        CfValueObject {
                            kind: CfValueObjectKind::Min,
                            value: None,
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Percentile,
                            value: Some("50".into()),
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Max,
                            value: None,
                        },
                    ],
                    colors: vec!["#F8696B".into(), "#FFEB84".into(), "#63BE7B".into()],
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rules = reopened.conditional_formats("Sheet1").unwrap();
    assert_eq!(rules.len(), 1);
    let cs = rules[0].color_scale.as_ref().expect("color_scale");
    assert_eq!(cs.values.len(), 3);
    assert_eq!(cs.values[0].kind, CfValueObjectKind::Min);
    assert_eq!(cs.values[1].kind, CfValueObjectKind::Percentile);
    assert_eq!(cs.values[1].value.as_deref(), Some("50"));
    assert_eq!(cs.colors.len(), 3);
    assert!(cs.colors[0].to_uppercase().ends_with("F8696B"));
}

#[test]
fn conditional_format_data_bar_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_conditional_format("Sheet1", "B1:B20",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::DataBar,
                data_bar: Some(DataBarPatch {
                    min: None,
                    max: None,
                    color: "#638EC6".into(),
                    min_length: Some(10),
                    max_length: Some(90),
                    show_value: Some(true),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rules = reopened.conditional_formats("Sheet1").unwrap();
    let db = rules[0].data_bar.as_ref().expect("data_bar");
    assert_eq!(db.min.as_ref().unwrap().kind, CfValueObjectKind::Min);
    assert_eq!(db.max.as_ref().unwrap().kind, CfValueObjectKind::Max);
    assert_eq!(db.min_length, Some(10));
    assert_eq!(db.max_length, Some(90));
    assert_eq!(db.show_value, Some(true));
    assert!(db.color.to_uppercase().ends_with("638EC6"));
}

#[test]
fn conditional_format_icon_set_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_conditional_format("Sheet1", "C1:C30",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::IconSet,
                icon_set: Some(IconSetPatch {
                    icon_set: CfIconSetKind::FourTrafficLights,
                    values: vec![
                        CfValueObject {
                            kind: CfValueObjectKind::Percent,
                            value: Some("0".into()),
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Percent,
                            value: Some("25".into()),
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Percent,
                            value: Some("50".into()),
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Percent,
                            value: Some("75".into()),
                        },
                    ],
                    show_value: Some(false),
                    percent: Some(true),
                    reverse: Some(true),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rules = reopened.conditional_formats("Sheet1").unwrap();
    let is = rules[0].icon_set.as_ref().expect("icon_set");
    assert_eq!(is.icon_set, CfIconSetKind::FourTrafficLights);
    assert_eq!(is.values.len(), 4);
    assert_eq!(is.show_value, Some(false));
    assert_eq!(is.percent, Some(true));
    assert_eq!(is.reverse, Some(true));
}

#[test]
fn conditional_format_color_scale_rejects_mismatched_lengths() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_conditional_format("Sheet1", "A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::ColorScale,
                color_scale: Some(ColorScalePatch {
                    values: vec![
                        CfValueObject {
                            kind: CfValueObjectKind::Min,
                            value: None,
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Max,
                            value: None,
                        },
                    ],
                    colors: vec!["#FF0000".into()],
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidConditionalFormat);
}

#[test]
fn conditional_format_icon_set_rejects_wrong_arity() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_conditional_format("Sheet1", "A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::IconSet,
                icon_set: Some(IconSetPatch {
                    icon_set: CfIconSetKind::ThreeTrafficLights1,
                    values: vec![
                        CfValueObject {
                            kind: CfValueObjectKind::Percent,
                            value: Some("0".into()),
                        },
                        CfValueObject {
                            kind: CfValueObjectKind::Percent,
                            value: Some("50".into()),
                        },
                    ],
                    show_value: None,
                    percent: None,
                    reverse: None,
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidConditionalFormat);
}
