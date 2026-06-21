#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_hstack_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=HSTACK()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_hstack_basic() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("C2", "2");
    model._set("D1", "3");
    model._set("D2", "4");
    model._set("A1", "=HSTACK(C1:C2, D1:D2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("B2"), *"4");
}

#[test]
fn fn_hstack_ragged_pads_na() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("C2", "2");
    model._set("C3", "3");
    model._set("D1", "9");
    model._set("A1", "=HSTACK(C1:C3, D1:D1)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"9");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("B2"), *"#N/A");
    assert_eq!(model._get_text("A3"), *"3");
    assert_eq!(model._get_text("B3"), *"#N/A");
}

#[test]
fn fn_hstack_scalar() {
    let mut model = new_empty_model();
    model._set("A1", "=HSTACK(1, 2, 3)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("C1"), *"3");
}
