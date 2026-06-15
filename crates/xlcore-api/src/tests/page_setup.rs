use crate::*;

#[test]
fn page_setup_set_read_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    let initial = wb.page_setup("Sheet1").unwrap();
    assert_eq!(initial.sheet, "Sheet1");
    assert!(initial.page.is_none());
    assert!(initial.margins.is_none());
    assert!(initial.print_options.is_none());
    assert!(initial.header_footer.is_none());

    let info = wb
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                page: Some(PageSetupSettingsPatch {
                    orientation: Some(PageOrientation::Landscape),
                    paper_size: Some(9),
                    scale: Some(85),
                    fit_to_width: Some(1),
                    fit_to_height: Some(0),
                    page_order: Some(PageOrder::OverThenDown),
                    cell_comments: Some(PrintCellComments::AtEnd),
                    errors: Some(PrintErrors::Dash),
                    copies: Some(2),
                    ..Default::default()
                }),
                margins: Some(PageMarginsPatch {
                    left: Some(0.5),
                    right: Some(0.5),
                    top: Some(0.75),
                    bottom: Some(0.75),
                    header: Some(0.3),
                    footer: Some(0.3),
                }),
                print_options: Some(PrintOptionsPatch {
                    horizontal_centered: Some(true),
                    grid_lines: Some(true),
                    headings: Some(true),
                    ..Default::default()
                }),
                header_footer: Some(HeaderFooterPatch {
                    odd_header: Some("&LLeft&CCenter&RRight".to_string()),
                    odd_footer: Some("&CPage &P of &N".to_string()),
                    different_first: Some(true),
                    first_header: Some("&CCover".to_string()),
                    scale_with_doc: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let page = info.page.as_ref().unwrap();
    assert_eq!(page.orientation, Some(PageOrientation::Landscape));
    assert_eq!(page.scale, Some(85));
    assert_eq!(page.fit_to_width, Some(1));
    assert_eq!(page.copies, Some(2));
    let margins = info.margins.as_ref().unwrap();
    assert!((margins.top - 0.75).abs() < 1e-9);
    let po = info.print_options.as_ref().unwrap();
    assert_eq!(po.horizontal_centered, Some(true));
    let hf = info.header_footer.as_ref().unwrap();
    assert_eq!(hf.odd_header.as_deref(), Some("&LLeft&CCenter&RRight"));
    assert_eq!(hf.different_first, Some(true));

    let updated = wb
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                page: Some(PageSetupSettingsPatch {
                    scale: Some(120),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let page = updated.page.as_ref().unwrap();
    assert_eq!(page.scale, Some(120));
    assert_eq!(page.orientation, Some(PageOrientation::Landscape));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.page_setup("Sheet1").unwrap();
    let page = after.page.as_ref().unwrap();
    assert_eq!(page.scale, Some(120));
    assert_eq!(page.orientation, Some(PageOrientation::Landscape));
    assert_eq!(page.cell_comments, Some(PrintCellComments::AtEnd));
    let hf = after.header_footer.as_ref().unwrap();
    assert_eq!(hf.first_header.as_deref(), Some("&CCover"));
    assert_eq!(hf.scale_with_doc, Some(false));

    let removed = reopened.remove_page_setup("Sheet1").unwrap();
    assert!(removed.page.is_some());
    let cleared = reopened.page_setup("Sheet1").unwrap();
    assert!(cleared.page.is_none());
    assert!(cleared.margins.is_none());
    assert!(cleared.print_options.is_none());
    assert!(cleared.header_footer.is_none());

    let err = reopened
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                page: Some(PageSetupSettingsPatch {
                    scale: Some(5),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidPageSetup);

    let err = reopened
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                margins: Some(PageMarginsPatch {
                    left: Some(-0.1),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidPageSetup);

    let err = reopened
        .set_page_setup("Ghost", SheetPageSetupPatch::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn sheet_qualified_print_area_input_is_not_corrupted() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                print_area: Some("Sheet1!A1:C5".to_string()),
                print_title_rows: Some("Sheet1!1:2".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.print_area.as_deref(), Some("A1:C5"));
    assert_eq!(info.print_title_rows.as_deref(), Some("1:2"));

    let dns = wb.defined_names().unwrap();
    let area = dns.iter().find(|d| d.name == "_xlnm.Print_Area").unwrap();
    assert_eq!(area.reference, "Sheet1!$A$1:$C$5");
}

#[test]
fn print_area_titles_and_breaks_round_trip() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                print_area: Some("A1:D10".to_string()),
                print_title_rows: Some("1:2".to_string()),
                print_title_columns: Some("A:A".to_string()),
                row_breaks: Some(vec![10, 5, 5]),
                column_breaks: Some(vec![3]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.print_area.as_deref(), Some("A1:D10"));
    assert_eq!(info.print_title_rows.as_deref(), Some("1:2"));
    assert_eq!(info.print_title_columns.as_deref(), Some("A:A"));
    assert_eq!(info.row_breaks, vec![5, 10]);
    assert_eq!(info.column_breaks, vec![3]);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.page_setup("Sheet1").unwrap();
    assert_eq!(after.print_area.as_deref(), Some("A1:D10"));
    assert_eq!(after.print_title_rows.as_deref(), Some("1:2"));
    assert_eq!(after.print_title_columns.as_deref(), Some("A:A"));
    assert_eq!(after.row_breaks, vec![5, 10]);
    assert_eq!(after.column_breaks, vec![3]);

    let dns = reopened.defined_names().unwrap();
    let area = dns.iter().find(|d| d.name == "_xlnm.Print_Area").unwrap();
    assert_eq!(area.reference, "Sheet1!$A$1:$D$10");
    assert_eq!(area.scope.as_deref(), Some("Sheet1"));
    let titles = dns.iter().find(|d| d.name == "_xlnm.Print_Titles").unwrap();
    assert_eq!(titles.reference, "Sheet1!$A:$A,Sheet1!$1:$2");

    let updated = reopened
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                print_title_rows: Some("1:3".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.print_title_rows.as_deref(), Some("1:3"));
    assert_eq!(updated.print_title_columns.as_deref(), Some("A:A"));

    let cleared = reopened
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                print_area: Some(String::new()),
                print_title_rows: Some(String::new()),
                print_title_columns: Some(String::new()),
                row_breaks: Some(vec![]),
                column_breaks: Some(vec![]),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(cleared.print_area.is_none());
    assert!(cleared.print_title_rows.is_none());
    assert!(cleared.print_title_columns.is_none());
    assert!(cleared.row_breaks.is_empty());
    assert!(cleared.column_breaks.is_empty());
    assert!(reopened
        .defined_names()
        .unwrap()
        .iter()
        .all(|d| !d.name.starts_with("_xlnm.Print")));
}
