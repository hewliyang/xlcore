#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn sum_of_spilled_range() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(3)");
    model._set("C1", "=SUM(A1#)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), "1".to_string());
    assert_eq!(model._get_text("A2"), "2".to_string());
    assert_eq!(model._get_text("A3"), "3".to_string());
    assert_eq!(model._get_text("C1"), "6".to_string());
}

#[test]
fn spill_reference_single_value() {
    let mut model = new_empty_model();
    model._set("A1", "42");
    model._set("C1", "=A1#");

    model.evaluate();

    assert_eq!(model._get_text("C1"), "42".to_string());
}

#[test]
fn spill_reference_empty_cell() {
    let mut model = new_empty_model();
    model._set("C1", "=A1#");

    model.evaluate();

    assert_eq!(model._get_text("C1"), "#REF!".to_string());
}

#[test]
fn spill_reference_spills_again() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(3)");
    model._set("C1", "=A1#");

    model.evaluate();

    assert_eq!(model._get_text("C1"), "1".to_string());
    assert_eq!(model._get_text("C2"), "2".to_string());
    assert_eq!(model._get_text("C3"), "3".to_string());
}

#[test]
fn spill_reference_stringify() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(3)");
    model._set("C1", "=SUM(A1#)");

    model.evaluate();

    assert_eq!(
        model._get_formula("C1"),
        "=SUM(A1#)".to_string()
    );
}
