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
        vec![(2, 2), (3, 2), (4, 2), (5, 2), (6, 2), (2, 4)]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    );
}
