use std::path::Path;

#[test]
fn list_validation_cells_get_dropdowns() {
    let layout = xlcore_export::extract(Path::new(
        "../../packages/xlsx-preview/tests/fixtures/data-validation-list.xlsx",
    ))
    .expect("extract");
    let sheet = &layout.sheets[0];

    let mut cells: Vec<(u32, u32)> = sheet
        .validation_dropdowns
        .iter()
        .map(|d| (d.r, d.c))
        .collect();
    cells.sort();

    assert_eq!(
        cells,
        vec![(2, 2), (2, 4), (3, 2), (4, 2), (5, 2), (6, 2)]
    );
}

#[test]
fn list_validations_resolve_inline_options() {
    let layout = xlcore_export::extract(Path::new(
        "../../packages/xlsx-preview/tests/fixtures/data-validation-list.xlsx",
    ))
    .expect("extract");
    let sheet = &layout.sheets[0];

    let opts_for = |r: u32, c: u32| -> &[String] {
        let d = sheet
            .validation_dropdowns
            .iter()
            .find(|d| d.r == r && d.c == c)
            .expect("dropdown");
        &sheet.validation_lists[d.list as usize]
    };

    assert_eq!(opts_for(2, 2), &["Open", "Closed", "Pending"]);
    assert_eq!(opts_for(6, 2), &["Open", "Closed", "Pending"]);
    assert_eq!(opts_for(2, 4), &["Yes", "No"]);
}
