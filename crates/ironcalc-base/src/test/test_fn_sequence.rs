#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_sequence_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE()");
    model._set("A2", "=SEQUENCE(1,2,3,4,5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_sequence_spills() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(3,2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("A2"), *"3");
    assert_eq!(model._get_text("B2"), *"4");
    assert_eq!(model._get_text("A3"), *"5");
    assert_eq!(model._get_text("B3"), *"6");
}

#[test]
fn fn_sequence_start_step() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(2,3,10,5)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"10");
    assert_eq!(model._get_text("B1"), *"15");
    assert_eq!(model._get_text("C1"), *"20");
    assert_eq!(model._get_text("A2"), *"25");
    assert_eq!(model._get_text("B2"), *"30");
    assert_eq!(model._get_text("C2"), *"35");
}

#[test]
fn fn_sequence_zero_dimension() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(0)");
    model._set("A2", "=SEQUENCE(2,-1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
    assert_eq!(model._get_text("A2"), *"#VALUE!");
}

#[test]
fn fn_sequence_single_arg() {
    let mut model = new_empty_model();
    model._set("A1", "=SEQUENCE(3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("A3"), *"3");
}
