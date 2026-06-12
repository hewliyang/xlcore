use crate::*;

#[test]
fn hyperlinks_add_list_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "anthropic").unwrap();
    wb.set_value("Sheet1!B2", "internal").unwrap();

    let info = wb
        .set_hyperlink("Sheet1", "A1",
            HyperlinkPatch {
                target: Some("https://anthropic.com".to_string()),
                tooltip: Some("home".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.reference, "A1:A1");
    assert_eq!(info.target.as_deref(), Some("https://anthropic.com"));

    wb.set_hyperlink("Sheet1", "B2:C3",
        HyperlinkPatch {
            location: Some("Sheet1!Z9".to_string()),
            display: Some("jump".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    let list = wb.hyperlinks("Sheet1").unwrap();
    assert_eq!(list.len(), 2);

    let err = wb
        .set_hyperlink("Sheet1", "D1", HyperlinkPatch::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidHyperlink);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.hyperlinks("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    let a1 = after.iter().find(|h| h.start_row == 1).unwrap();
    assert_eq!(a1.target.as_deref(), Some("https://anthropic.com"));
    assert_eq!(a1.tooltip.as_deref(), Some("home"));
    let b2 = after.iter().find(|h| h.start_row == 2).unwrap();
    assert_eq!(b2.location.as_deref(), Some("Sheet1!Z9"));
    assert_eq!(b2.reference, "B2:C3");

    let removed = reopened.remove_hyperlink("Sheet1", "B3").unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].reference, "B2:C3");
    assert_eq!(reopened.hyperlinks("Sheet1").unwrap().len(), 1);

    reopened
        .set_hyperlink("Sheet1", "A1",
            HyperlinkPatch {
                target: Some("https://example.com".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    let list = reopened.hyperlinks("Sheet1").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].target.as_deref(), Some("https://example.com"));

    let bytes = reopened.save_bytes().unwrap();
    let rels = sheet_rels_xml(&bytes).expect("sheet1 rels present");
    assert!(
        rels.contains("https://example.com"),
        "expected current target in sheet rels: {rels}"
    );
    assert!(
        !rels.contains("https://anthropic.com"),
        "expected orphan anthropic.com rel to be cleaned: {rels}"
    );

    reopened.remove_hyperlink("Sheet1", "A1").unwrap();
    assert!(reopened.hyperlinks("Sheet1").unwrap().is_empty());
    let bytes = reopened.save_bytes().unwrap();
    let rels = sheet_rels_xml(&bytes).unwrap_or_default();
    assert!(
        !rels.contains("https://example.com"),
        "expected example.com rel to be cleaned after final remove: {rels}"
    );
}

#[test]
fn set_hyperlink_populates_display_into_blank_top_left_cell() {
    let mut wb = Workbook::new().unwrap();
    wb.set_hyperlink("Sheet1", "A1",
        HyperlinkPatch {
            target: Some("https://anthropic.com".to_string()),
            display: Some("Anthropic".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let cell = wb.get_cell("Sheet1!A1").unwrap();
    assert_eq!(cell.value, ApiCellValue::String("Anthropic".to_string()));

    wb.set_value("Sheet1!B2", "existing").unwrap();
    wb.set_hyperlink("Sheet1", "B2",
        HyperlinkPatch {
            target: Some("https://example.com".to_string()),
            display: Some("do not overwrite".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let cell = wb.get_cell("Sheet1!B2").unwrap();
    assert_eq!(cell.value, ApiCellValue::String("existing".to_string()));

    wb.set_hyperlink("Sheet1", "C3:D4",
        HyperlinkPatch {
            location: Some("Sheet1!Z9".to_string()),
            display: Some("jump".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let top_left = wb.get_cell("Sheet1!C3").unwrap();
    assert_eq!(top_left.value, ApiCellValue::String("jump".to_string()));
    let other = wb.get_cell("Sheet1!D4").unwrap();
    assert_eq!(other.value, ApiCellValue::Blank);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let a1 = reopened.get_cell("Sheet1!A1").unwrap();
    assert_eq!(a1.value, ApiCellValue::String("Anthropic".to_string()));
}

fn sheet_rels_xml(bytes: &[u8]) -> Option<String> {
    use std::io::{Cursor, Read};
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
    let mut file = zip.by_name("xl/worksheets/_rels/sheet1.xml.rels").ok()?;
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    Some(out)
}
