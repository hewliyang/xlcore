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
