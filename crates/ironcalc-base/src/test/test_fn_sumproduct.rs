#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn multiplies_corresponding_range_values() {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("B1", "10");
    model._set("B2", "20");
    model._set("B3", "30");
    model._set("C1", "=SUMPRODUCT(A1:A3,B1:B3)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"140");
}

#[test]
fn supports_more_than_two_arrays() {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("A2", "2");
    model._set("B1", "10");
    model._set("B2", "20");
    model._set("C1", "2");
    model._set("C2", "3");
    model._set("D1", "=SUMPRODUCT(A1:A2,B1:B2,C1:C2)");

    model.evaluate();

    assert_eq!(model._get_text("D1"), *"140");
}

#[test]
fn supports_array_constants_and_ignores_text() {
    let mut model = new_empty_model();
    model._set("A1", r#"=SUMPRODUCT({1,2;"x",4},{10,20;30,40})"#);

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"210");
}

#[test]
fn mismatched_dimensions_return_value_error() {
    let mut model = new_empty_model();
    model._set("A1", "=SUMPRODUCT({1,2},{1;2})");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
