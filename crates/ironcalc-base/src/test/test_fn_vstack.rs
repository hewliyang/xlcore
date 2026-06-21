#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_vstack_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=VSTACK()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_vstack_basic() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("D1", "2");
    model._set("C2", "3");
    model._set("D2", "4");
    model._set("A1", "=VSTACK(C1:D1, C2:D2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("A2"), *"3");
    assert_eq!(model._get_text("B2"), *"4");
}

#[test]
fn fn_vstack_ragged_pads_na() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("I1", "3");
    model._set("G2", "9");
    model._set("A1", "=VSTACK(G1:I1, G2:G2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("C1"), *"3");
    assert_eq!(model._get_text("A2"), *"9");
    assert_eq!(model._get_text("B2"), *"#N/A");
    assert_eq!(model._get_text("C2"), *"#N/A");
}

#[test]
fn fn_vstack_scalar() {
    let mut model = new_empty_model();
    model._set("A1", "=VSTACK(1, 2, 3)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("A3"), *"3");
}
