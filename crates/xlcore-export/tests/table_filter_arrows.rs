use std::path::Path;

#[test]
fn autofilter_arrows_carry_column_identity() {
    let layout = xlcore_export::extract(Path::new(
        "../../tests/fixtures/tables/autofilter-hidden-rows.xlsx",
    ))
    .expect("extract");
    let sheet = &layout.sheets[0];
    let arrows = &sheet.table_filter_arrows;
    assert_eq!(arrows.len(), 3, "one arrow per autofilter column");

    let names: Vec<&str> = arrows.iter().map(|a| a.column_name.as_str()).collect();
    assert_eq!(names, vec!["Region", "Product", "Amount"]);

    for (i, a) in arrows.iter().enumerate() {
        assert_eq!(a.r, 1);
        assert_eq!(a.column_offset, i as u32);
        assert_eq!(a.c, 1 + i as u32);
        assert_eq!(a.range_ref, "Filtered!A1:C7");
    }
}
