#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_mode_mult_two_modes() {
    let mut model = new_empty_model();
    model._set("Z1", "1");
    model._set("Z2", "2");
    model._set("Z3", "3");
    model._set("Z4", "4");
    model._set("Z5", "3");
    model._set("Z6", "2");
    model._set("Z7", "1");
    model._set("Z8", "2");
    model._set("Z9", "3");
    model._set("A1", "=MODE.MULT(Z1:Z9)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("A2"), *"3");
}

#[test]
fn fn_mode_mult_single_mode() {
    let mut model = new_empty_model();
    model._set("Z1", "1");
    model._set("Z2", "2");
    model._set("Z3", "2");
    model._set("Z4", "3");
    model._set("A1", "=MODE.MULT(Z1:Z4)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
}

#[test]
fn fn_mode_mult_no_repeats() {
    let mut model = new_empty_model();
    model._set("Z1", "1");
    model._set("Z2", "2");
    model._set("Z3", "3");
    model._set("A1", "=MODE.MULT(Z1:Z3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#N/A");
}

#[test]
fn fn_mode_mult_arg_count() {
    let mut model = new_empty_model();
    model._set("A1", "=MODE.MULT()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}
