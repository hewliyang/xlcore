#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_mmult_2x3_by_3x2() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("I1", "3");
    model._set("G2", "4");
    model._set("H2", "5");
    model._set("I2", "6");
    model._set("K1", "7");
    model._set("L1", "8");
    model._set("K2", "9");
    model._set("L2", "10");
    model._set("K3", "11");
    model._set("L3", "12");
    model._set("A1", "=MMULT(G1:I2, K1:L3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"58");
    assert_eq!(model._get_text("B1"), *"64");
    assert_eq!(model._get_text("A2"), *"139");
    assert_eq!(model._get_text("B2"), *"154");
}

#[test]
fn fn_mmult_dim_mismatch() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model._set("K1", "1");
    model._set("K2", "2");
    model._set("K3", "3");
    model._set("A1", "=MMULT(G1:H2, K1:K3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_mmult_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=MMULT(G1:H2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}
