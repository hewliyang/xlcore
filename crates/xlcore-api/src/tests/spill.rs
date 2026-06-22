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
