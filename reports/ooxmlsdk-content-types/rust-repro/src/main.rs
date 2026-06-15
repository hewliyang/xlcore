use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use ooxmlsdk::parts::workbook_styles_part::WorkbookStylesPart;
use ooxmlsdk::sdk::{SdkPart, SpreadsheetDocumentType};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;

fn main() {
    let mut doc = SpreadsheetDocument::create(SpreadsheetDocumentType::Workbook);
    let wb_part = doc.add_workbook_part().unwrap();

    let ws_part: WorksheetPart = wb_part.add_new_part_auto_id(&mut doc).unwrap();
    ws_part
        .set_root_element(
            &mut doc,
            x::Worksheet {
                xmlns: vec![ooxmlsdk::common::XmlNamespaceDecl::new(
                    "",
                    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
                )],
                sheet_data: Box::new(x::SheetData::default()),
                ..Default::default()
            },
        )
        .unwrap();
    let sheet_rid = ws_part.relationship_id().unwrap().to_string();

    let styles_part: WorkbookStylesPart = wb_part.add_new_part_auto_id(&mut doc).unwrap();
    styles_part
        .set_root_element(
            &mut doc,
            x::Stylesheet {
                xmlns: vec![ooxmlsdk::common::XmlNamespaceDecl::new(
                    "",
                    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
                )],
                ..Default::default()
            },
        )
        .unwrap();

    wb_part
        .set_root_element(
            &mut doc,
            x::Workbook {
                xmlns: vec![ooxmlsdk::common::XmlNamespaceDecl::new(
                    "",
                    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
                )],
                sheets: Box::new(x::Sheets {
                    sheet: vec![x::Sheet {
                        name: "Sheet1".to_string(),
                        sheet_id: 1,
                        id: sheet_rid,
                        ..Default::default()
                    }],
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = doc.to_package_bytes().unwrap();
    std::fs::write("rust.xlsx", &bytes).unwrap();
    eprintln!("wrote rust.xlsx ({} bytes)", bytes.len());
}
