#[test]
fn array_comparison_broadcasts_to_mask() {
    use crate::test::util::new_empty_model;
    let mut model = new_empty_model();
    model._set("B1", "10");
    model._set("B2", "20");
    model._set("B3", "30");
    model._set("B4", "5");
    model._set("Y1", "2024");
    model._set("Y2", "2024");
    model._set("Y3", "2024");
    model._set("Y4", "2023");
    model._set("A1", "=Y1:Y4=2024");
    model._set("C1", "=SUMPRODUCT((Y1:Y4=2024)*1)");
    model._set("D1", "=FILTER(B1:B4, Y1:Y4=2024)");
    model._set("E1", "=MEDIAN(FILTER(B1:B4, Y1:Y4=2024))");
    model._set("F1", "=PERCENTILE.INC(FILTER(B1:B4, Y1:Y4=2024), 0.5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), "TRUE");
    assert_eq!(model._get_text("A2"), "TRUE");
    assert_eq!(model._get_text("A3"), "TRUE");
    assert_eq!(model._get_text("A4"), "FALSE");
    assert_eq!(model._get_text("C1"), "3");
    assert_eq!(model._get_text("D1"), "10");
    assert_eq!(model._get_text("D2"), "20");
    assert_eq!(model._get_text("D3"), "30");
    assert_eq!(model._get_text("E1"), "20");
    assert_eq!(model._get_text("F1"), "20");
}

#[test]
fn array_comparison_scalar_on_left_and_operators() {
    use crate::test::util::new_empty_model;
    let mut model = new_empty_model();
    model._set("B1", "10");
    model._set("B2", "20");
    model._set("B3", "30");
    model._set("G1", "=20>=B1:B3");
    model._set("H1", "=SUMPRODUCT((B1:B3>15)*1)");
    model.evaluate();
    assert_eq!(model._get_text("G1"), "TRUE");
    assert_eq!(model._get_text("G2"), "TRUE");
    assert_eq!(model._get_text("G3"), "FALSE");
    assert_eq!(model._get_text("H1"), "2");
}

#[test]
fn array_comparison_array_vs_array() {
    use crate::test::util::new_empty_model;
    let mut model = new_empty_model();
    model._set("B1", "1");
    model._set("B2", "2");
    model._set("C1", "1");
    model._set("C2", "5");
    model._set("D1", "=SUMPRODUCT((B1:B2=C1:C2)*1)");
    model.evaluate();
    assert_eq!(model._get_text("D1"), "1");
}
