use crate::*;

#[test]
fn lists_gets_and_round_trips_parts() {
    let workbook = Workbook::new().unwrap();
    let names = workbook.part_names().unwrap();
    assert!(names.contains(&"xl/workbook.xml".to_string()));
    assert!(names.contains(&"[Content_Types].xml".to_string()));

    let workbook_xml = workbook.get_part_xml("xl/workbook.xml").unwrap().unwrap();
    assert!(workbook_xml.contains("<sheets>"));
    assert!(workbook.get_part_xml("xl/missing.xml").unwrap().is_none());
    assert!(workbook
        .get_part_xml("/xl/workbook.xml")
        .unwrap()
        .is_some());
}

#[test]
fn sets_new_unmodeled_part_and_preserves_it() {
    let mut workbook = Workbook::new().unwrap();
    let custom = "<?xml version=\"1.0\"?>\n<root><note>escape hatch</note></root>";
    workbook.set_part_xml("customXml/item1.xml", custom).unwrap();

    assert_eq!(
        workbook.get_part_xml("customXml/item1.xml").unwrap().as_deref(),
        Some(custom)
    );

    let bytes = workbook.save_bytes().unwrap();
    let reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_part_xml("customXml/item1.xml").unwrap().as_deref(),
        Some(custom)
    );
}

#[test]
fn overwrites_existing_modeled_part() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", "before").unwrap();
    let edited = workbook
        .get_part_xml("xl/worksheets/sheet1.xml")
        .unwrap()
        .unwrap()
        .replace("before", "after");
    workbook.set_part_xml("xl/worksheets/sheet1.xml", &edited).unwrap();
    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().value,
        CellValue::String("after".to_string())
    );
}

#[test]
fn removes_part() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_part_xml("customXml/item1.xml", "<root/>").unwrap();
    assert!(workbook.remove_part_xml("customXml/item1.xml").unwrap());
    assert!(workbook.get_part_xml("customXml/item1.xml").unwrap().is_none());
    assert!(!workbook.remove_part_xml("customXml/item1.xml").unwrap());
}
