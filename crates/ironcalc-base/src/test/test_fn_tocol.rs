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
fn fn_tocol_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=TOCOL()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_tocol_basic_by_row() {
    let mut model = setup();
    model._set("A1", "=TOCOL(G1:I2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("A3"), *"3");
    assert_eq!(model._get_text("A4"), *"4");
    assert_eq!(model._get_text("A5"), *"5");
    assert_eq!(model._get_text("A6"), *"6");
    assert_eq!(model._get_text("B1"), *"");
}

#[test]
fn fn_tocol_scan_by_column() {
    let mut model = setup();
    model._set("A1", "=TOCOL(G1:I2, 0, TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"4");
    assert_eq!(model._get_text("A3"), *"2");
    assert_eq!(model._get_text("A4"), *"5");
    assert_eq!(model._get_text("A5"), *"3");
    assert_eq!(model._get_text("A6"), *"6");
}

#[test]
fn fn_tocol_ignore_errors() {
    let mut model = setup();
    model._set("H1", "=1/0");
    model._set("A1", "=TOCOL(G1:I2, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"3");
    assert_eq!(model._get_text("A3"), *"4");
    assert_eq!(model._get_text("A4"), *"5");
    assert_eq!(model._get_text("A5"), *"6");
}

#[test]
fn fn_tocol_bad_ignore() {
    let mut model = setup();
    model._set("A1", "=TOCOL(G1:I2, 9)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
