#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn setup() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model
}

#[test]
fn fn_expand_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=EXPAND(G1:H2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_expand_grow_default_pad() {
    let mut model = setup();
    model._set("A1", "=EXPAND(G1:H2, 3, 3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("C1"), *"#N/A");
    assert_eq!(model._get_text("A2"), *"3");
    assert_eq!(model._get_text("B2"), *"4");
    assert_eq!(model._get_text("C2"), *"#N/A");
    assert_eq!(model._get_text("A3"), *"#N/A");
    assert_eq!(model._get_text("B3"), *"#N/A");
    assert_eq!(model._get_text("C3"), *"#N/A");
}

#[test]
fn fn_expand_custom_pad() {
    let mut model = setup();
    model._set("A1", "=EXPAND(G1:H2, 3, 2, 0)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("A3"), *"0");
    assert_eq!(model._get_text("B3"), *"0");
}

#[test]
fn fn_expand_columns_omitted_keeps_width() {
    let mut model = setup();
    model._set("A1", "=EXPAND(G1:H2, 3, , \"x\")");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("A3"), *"x");
    assert_eq!(model._get_text("B3"), *"x");
    assert_eq!(model._get_text("C1"), *"");
}

#[test]
fn fn_expand_shrink_value_error() {
    let mut model = setup();
    model._set("A1", "=EXPAND(G1:H2, 1, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_expand_zero_value_error() {
    let mut model = setup();
    model._set("A1", "=EXPAND(G1:H2, 0, 3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
