#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_drop_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=DROP(G1:H3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_drop_rows_positive() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model._set("G3", "5");
    model._set("H3", "6");
    model._set("A1", "=DROP(G1:H3, 1)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("B1"), *"4");
    assert_eq!(model._get_text("A2"), *"5");
    assert_eq!(model._get_text("B2"), *"6");
    assert_eq!(model._get_text("A3"), *"");
}

#[test]
fn fn_drop_rows_negative() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model._set("G3", "5");
    model._set("H3", "6");
    model._set("A1", "=DROP(G1:H3, -2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("A2"), *"");
}

#[test]
fn fn_drop_columns() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("I1", "3");
    model._set("G2", "4");
    model._set("H2", "5");
    model._set("I2", "6");
    model._set("A1", "=DROP(G1:I2, 0, 1)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("A2"), *"5");
    assert_eq!(model._get_text("B2"), *"6");
}

#[test]
fn fn_drop_columns_omitted() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model._set("A1", "=DROP(G1:H2, 1)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("B1"), *"4");
}

#[test]
fn fn_drop_empty_calc() {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model._set("A1", "=DROP(G1:H2, 2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#CALC!");
}
