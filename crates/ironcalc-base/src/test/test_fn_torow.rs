#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn setup() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("I1", "3");
    model._set("G2", "4");
    model._set("H2", "5");
    model._set("I2", "6");
    model
}

#[test]
fn fn_torow_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=TOROW()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_torow_basic_by_row() {
    let mut model = setup();
    model._set("A1", "=TOROW(G1:I2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("C1"), *"3");
    assert_eq!(model._get_text("D1"), *"4");
    assert_eq!(model._get_text("E1"), *"5");
    assert_eq!(model._get_text("F1"), *"6");
    assert_eq!(model._get_text("A2"), *"");
}

#[test]
fn fn_torow_scan_by_column() {
    let mut model = setup();
    model._set("A1", "=TOROW(G1:I2, 0, TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"4");
    assert_eq!(model._get_text("C1"), *"2");
    assert_eq!(model._get_text("D1"), *"5");
    assert_eq!(model._get_text("E1"), *"3");
    assert_eq!(model._get_text("F1"), *"6");
}

#[test]
fn fn_torow_ignore_errors() {
    let mut model = setup();
    model._set("H1", "=1/0");
    model._set("A1", "=TOROW(G1:I2, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("C1"), *"4");
    assert_eq!(model._get_text("D1"), *"5");
    assert_eq!(model._get_text("E1"), *"6");
}

#[test]
fn fn_torow_bad_ignore() {
    let mut model = setup();
    model._set("A1", "=TOROW(G1:I2, 9)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
