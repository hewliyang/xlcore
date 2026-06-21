#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_munit_3() {
    let mut model = new_empty_model();
    model._set("A1", "=MUNIT(3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"0");
    assert_eq!(model._get_text("C1"), *"0");
    assert_eq!(model._get_text("A2"), *"0");
    assert_eq!(model._get_text("B2"), *"1");
    assert_eq!(model._get_text("C2"), *"0");
    assert_eq!(model._get_text("A3"), *"0");
    assert_eq!(model._get_text("B3"), *"0");
    assert_eq!(model._get_text("C3"), *"1");
}

#[test]
fn fn_munit_invalid() {
    let mut model = new_empty_model();
    model._set("A1", "=MUNIT(0)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
