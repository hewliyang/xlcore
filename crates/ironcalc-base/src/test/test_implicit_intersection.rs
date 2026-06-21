#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn simple_colum() {
    let mut model = new_empty_model();
    // We populate cells A1 to A3
    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");

    model._set("C2", "=@A1:A3");

    model.evaluate();

    assert_eq!(model._get_text("C2"), "2".to_string());
}

#[test]
fn legacy_range_implicit_intersection() {
    let mut model = new_empty_model();
    // We populate cells A1 to A3
    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");

    model._set("C2", "=A1:A3");
    model._set("C5", "=A1:A3");
    model._set("D2", "=SUM(SIN(A:A)");

    model.evaluate();

    assert_eq!(model._get_text("C2"), "2".to_string());
    assert_eq!(model._get_text("C5"), "#VALUE!".to_string());
    assert_eq!(model._get_text("D2"), "1.89188842".to_string());
}

#[test]
fn at_range_non_intersecting() {
    let mut model = new_empty_model();
    model._set("A1", "10");
    model._set("A2", "20");
    model._set("A3", "30");

    model._set("C5", "=@A1:A3");

    model.evaluate();

    assert_eq!(model._get_text("C5"), *"#VALUE!");
}

#[test]
fn at_range_horizontal_intersecting() {
    let mut model = new_empty_model();
    model._set("A1", "10");
    model._set("B1", "20");
    model._set("C1", "30");

    model._set("B5", "=@A1:C1");

    model.evaluate();

    assert_eq!(model._get_text("B5"), *"20");
}

#[test]
fn at_range_single_cell() {
    let mut model = new_empty_model();
    model._set("A1", "42");

    model._set("C5", "=@A1:A1");
    model._set("C6", "=@A1");

    model.evaluate();

    assert_eq!(model._get_text("C5"), *"42");
    assert_eq!(model._get_text("C6"), *"42");
}

#[test]
fn at_array_collapses_to_first() {
    let mut model = new_empty_model();

    model._set("C2", "=@SEQUENCE(3)");

    model.evaluate();

    assert_eq!(model._get_text("C2"), *"1");
}

#[test]
fn concat() {
    let mut model = new_empty_model();
    model._set("A1", "=CONCAT(@B1:B3)");
    model._set("A2", "=CONCAT(B1:B3)");
    model._set("B1", "Hello");
    model._set("B2", " ");
    model._set("B3", "world!");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"Hello");
    assert_eq!(model._get_text("A2"), *"Hello world!");
}
