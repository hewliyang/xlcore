use ooxmlsdk::common::XmlNamespaceDecl;
use ooxmlsdk::parts::spreadsheet_document::SpreadsheetDocument;
use ooxmlsdk::sdk::{SdkPart, SpreadsheetDocumentType};
use ooxmlsdk::schemas::schemas_openxmlformats_org_spreadsheetml_2006_main as x;

fn main() {
    let mut doc = SpreadsheetDocument::create(SpreadsheetDocumentType::Workbook);
    let wb_part = doc.add_workbook_part().unwrap();
    wb_part
        .set_root_element(
            &mut doc,
            x::Workbook {
                xmlns: vec![XmlNamespaceDecl::new(
                    "",
                    "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
                )],
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = doc.to_package_bytes().unwrap();
    std::fs::write("rust.xlsx", &bytes).unwrap();
    eprintln!("wrote rust.xlsx ({} bytes)", bytes.len());
}
