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
fn fn_choosecols_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=CHOOSECOLS(G1:I2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_choosecols_single() {
    let mut model = setup();
    model._set("A1", "=CHOOSECOLS(G1:I2, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("A2"), *"5");
    assert_eq!(model._get_text("B1"), *"");
}

#[test]
fn fn_choosecols_multiple_reordered() {
    let mut model = setup();
    model._set("A1", "=CHOOSECOLS(G1:I2, 3, 1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("B1"), *"1");
    assert_eq!(model._get_text("A2"), *"6");
    assert_eq!(model._get_text("B2"), *"4");
}

#[test]
fn fn_choosecols_repeated() {
    let mut model = setup();
    model._set("A1", "=CHOOSECOLS(G1:I2, 1, 1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"1");
}

#[test]
fn fn_choosecols_negative() {
    let mut model = setup();
    model._set("A1", "=CHOOSECOLS(G1:I2, -1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("A2"), *"6");
}

#[test]
fn fn_choosecols_out_of_range() {
    let mut model = setup();
    model._set("A1", "=CHOOSECOLS(G1:I2, 4)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_choosecols_zero() {
    let mut model = setup();
    model._set("A1", "=CHOOSECOLS(G1:I2, 0)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_choosecols_array_indices() {
    let mut model = setup();
    model._set("G4", "2");
    model._set("H4", "1");
    model._set("A1", "=CHOOSECOLS(G1:I2, G4:H4)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("B1"), *"1");
}
