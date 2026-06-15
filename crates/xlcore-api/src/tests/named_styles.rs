use crate::*;

fn styles_xml(wb: &Workbook) -> String {
    let name = wb
        .part_names()
        .unwrap()
        .into_iter()
        .find(|n| n.contains("styles"))
        .expect("styles part");
    wb.get_part_xml(&name).unwrap().expect("styles xml")
}

fn good_style() -> NamedStylePatch {
    NamedStylePatch {
        name: "Good".to_string(),
        builtin_id: Some(26),
        style: StylePatch {
            font: Some(FontPatch {
                color: Some("006100".to_string()),
                bold: Some(true),
                ..Default::default()
            }),
            fill: Some(FillPatch {
                color: Some("C6EFCE".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    }
}

#[test]
fn define_apply_and_round_trip_named_style() {
    let mut wb = Workbook::new().unwrap();
    let info = wb.set_named_style(good_style()).unwrap();
    assert_eq!(info.name, "Good");
    assert_eq!(info.builtin_id, Some(26));

    let listed = wb.named_styles().unwrap();
    assert!(listed.iter().any(|s| s.name == "Normal"));
    assert!(listed
        .iter()
        .any(|s| s.name == "Good" && s.builtin_id == Some(26)));

    wb.set_style(
        "Sheet1!A1",
        StylePatch {
            named_style: Some("Good".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();

    let listed = reopened.named_styles().unwrap();
    assert!(listed
        .iter()
        .any(|s| s.name == "Good" && s.builtin_id == Some(26)));

    let xml = styles_xml(&reopened);
    assert!(xml.contains("cellStyleXfs"));
    assert!(xml.contains("name=\"Good\""));
    assert!(xml.contains("builtinId=\"26\""));
    assert!(xml.contains("C6EFCE"));

    let a1 = reopened.get_cell("Sheet1!A1").unwrap();
    let style_idx = a1.style_index.expect("A1 styled");
    let part = reopened
        .doc
        .workbook_part()
        .unwrap()
        .clone()
        .workbook_styles_part(&mut reopened.doc)
        .unwrap();
    let sheet = part.root_element(&mut reopened.doc).unwrap();
    let xf = match &sheet.cell_formats.as_ref().unwrap().xml_children[style_idx as usize] {
        xlcore_io::spreadsheetml::CellFormatsChoice::CellFormat(xf) => xf,
        _ => panic!("expected cell format"),
    };
    let master = xf.format_id.expect("xfId set");
    let named_master = sheet
        .cell_styles
        .as_ref()
        .unwrap()
        .cell_style
        .iter()
        .find(|cs| cs.name.as_deref() == Some("Good"))
        .unwrap()
        .format_id;
    assert_eq!(master, named_master);
}

#[test]
fn set_named_style_is_in_place_update() {
    let mut wb = Workbook::new().unwrap();
    wb.set_named_style(good_style()).unwrap();
    wb.set_named_style(NamedStylePatch {
        name: "Good".to_string(),
        builtin_id: Some(26),
        style: StylePatch {
            fill: Some(FillPatch {
                color: Some("FFEB9C".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    })
    .unwrap();

    let count = wb
        .named_styles()
        .unwrap()
        .iter()
        .filter(|s| s.name == "Good")
        .count();
    assert_eq!(count, 1);

    let bytes = wb.save_bytes().unwrap();
    let reopened = Workbook::open_bytes(bytes).unwrap();
    let xml = styles_xml(&reopened);
    assert!(xml.contains("FFEB9C"));
}

#[test]
fn remove_named_style() {
    let mut wb = Workbook::new().unwrap();
    wb.set_named_style(good_style()).unwrap();
    let removed = wb.remove_named_style("Good").unwrap();
    assert_eq!(removed.unwrap().name, "Good");
    assert!(!wb.named_styles().unwrap().iter().any(|s| s.name == "Good"));
    assert!(wb.remove_named_style("Missing").unwrap().is_none());
    assert!(wb.remove_named_style("Normal").is_err());
}

#[test]
fn apply_unknown_named_style_errors() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_style(
            "Sheet1!A1",
            StylePatch {
                named_style: Some("Nope".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::UnsupportedStyle);
}
