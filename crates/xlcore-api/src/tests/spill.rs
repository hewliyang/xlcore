use crate::*;

#[test]
fn persists_authored_dynamic_array_spill() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_value("Sheet1!B1", 2.0).unwrap();
    workbook.set_value("Sheet1!C1", 3.0).unwrap();
    workbook
        .set_formula("Sheet1!A3", "=MAP(A1:C1,LAMBDA(x,x*x))")
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes.clone()).unwrap();

    assert_eq!(
        reopened.get_cell("Sheet1!A3").unwrap().value,
        CellValue::Number(1.0)
    );
    assert_eq!(
        reopened.get_cell("Sheet1!B3").unwrap().value,
        CellValue::Number(4.0)
    );
    assert_eq!(
        reopened.get_cell("Sheet1!C3").unwrap().value,
        CellValue::Number(9.0)
    );

    let xml = reopened
        .get_part_xml("xl/worksheets/sheet1.xml")
        .unwrap()
        .unwrap();
    assert!(
        xml.contains("t=\"array\"") && xml.contains("ref=\"A3:C3\""),
        "anchor must carry array ref, got {xml}"
    );
    assert!(
        xml.contains("<c r=\"B3\"><v>4</v></c>"),
        "spilled B3 cached value missing, got {xml}"
    );
    assert!(
        xml.contains("<c r=\"C3\"><v>9</v></c>"),
        "spilled C3 cached value missing, got {xml}"
    );
}

#[test]
fn scalar_formula_not_marked_as_array() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 10.0).unwrap();
    workbook.set_value("Sheet1!B1", 20.0).unwrap();
    workbook.set_formula("Sheet1!C1", "=A1+B1").unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();

    assert_eq!(
        reopened.get_cell("Sheet1!C1").unwrap().value,
        CellValue::Number(30.0)
    );

    let xml = reopened
        .get_part_xml("xl/worksheets/sheet1.xml")
        .unwrap()
        .unwrap();
    assert!(
        !xml.contains("t=\"array\""),
        "scalar formula must not be marked array, got {xml}"
    );
}

#[test]
fn emits_dynamic_array_metadata() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_value("Sheet1!B1", 2.0).unwrap();
    workbook.set_value("Sheet1!C1", 3.0).unwrap();
    workbook
        .set_formula("Sheet1!A3", "=MAP(A1:C1,LAMBDA(x,x*x))")
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();

    let metadata = reopened
        .get_part_xml("xl/metadata1.xml")
        .unwrap()
        .expect("xl/metadata1.xml present");
    assert!(
        metadata.contains("XLDAPR"),
        "metadata must declare XLDAPR, got {metadata}"
    );
    assert!(
        metadata.contains("dynamicArrayProperties") && metadata.contains("fDynamic=\"1\""),
        "metadata must carry dynamic array props, got {metadata}"
    );

    let content_types = reopened
        .get_part_xml("[Content_Types].xml")
        .unwrap()
        .expect("content types present");
    assert!(
        content_types.contains("sheetMetadata"),
        "content types must override sheetMetadata, got {content_types}"
    );

    let xml = reopened
        .get_part_xml("xl/worksheets/sheet1.xml")
        .unwrap()
        .unwrap();
    assert!(
        xml.contains("cm=\"1\""),
        "anchor must carry cm attr, got {xml}"
    );

    assert_eq!(
        reopened.get_cell("Sheet1!A3").unwrap().value,
        CellValue::Number(1.0)
    );
    assert_eq!(
        reopened.get_cell("Sheet1!B3").unwrap().value,
        CellValue::Number(4.0)
    );
    assert_eq!(
        reopened.get_cell("Sheet1!C3").unwrap().value,
        CellValue::Number(9.0)
    );
}

#[test]
fn no_metadata_without_dynamic_arrays() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 10.0).unwrap();
    workbook.set_value("Sheet1!B1", 20.0).unwrap();
    workbook.set_formula("Sheet1!C1", "=A1+B1").unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let reopened = Workbook::open_bytes(bytes).unwrap();

    assert!(
        reopened.get_part_xml("xl/metadata1.xml").unwrap().is_none(),
        "scalar-only workbook must not gain metadata1.xml"
    );
    assert!(
        !reopened
            .get_part_xml("[Content_Types].xml")
            .unwrap()
            .unwrap()
            .contains("sheetMetadata"),
        "scalar-only workbook must not gain sheetMetadata override"
    );
}
