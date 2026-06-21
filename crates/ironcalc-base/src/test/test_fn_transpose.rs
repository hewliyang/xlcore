#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn setup_source(model: &mut crate::model::Model) {
    model._set("C1", "1");
    model._set("D1", "2");
    model._set("E1", "3");
    model._set("C2", "4");
    model._set("D2", "5");
    model._set("E2", "6");
}

#[test]
fn fn_transpose_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=TRANSPOSE()");
    model._set("A2", "=TRANSPOSE(C1:E2, 1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_transpose_spills() {
    let mut model = new_empty_model();
    setup_source(&mut model);
    model._set("A4", "=TRANSPOSE(C1:E2)");
    model.evaluate();

    assert_eq!(model._get_text("A4"), *"1");
    assert_eq!(model._get_text("B4"), *"4");
    assert_eq!(model._get_text("A5"), *"2");
    assert_eq!(model._get_text("B5"), *"5");
    assert_eq!(model._get_text("A6"), *"3");
    assert_eq!(model._get_text("B6"), *"6");
}

#[test]
fn fn_transpose_spilled_cell_readable() {
    let mut model = new_empty_model();
    setup_source(&mut model);
    model._set("A4", "=TRANSPOSE(C1:E2)");
    model._set("C5", "=B5*10");
    model.evaluate();

    assert_eq!(model._get_text("B5"), *"5");
    assert_eq!(model._get_text("C5"), *"50");
}

#[test]
fn fn_transpose_idempotent() {
    let mut model = new_empty_model();
    setup_source(&mut model);
    model._set("A4", "=TRANSPOSE(C1:E2)");
    model._set("C5", "=B5*10");
    model.evaluate();
    model.evaluate();
    model.evaluate();

    assert_eq!(model._get_text("A4"), *"1");
    assert_eq!(model._get_text("B6"), *"6");
    assert_eq!(model._get_text("C5"), *"50");
}

#[test]
fn fn_transpose_spill_collision() {
    let mut model = new_empty_model();
    setup_source(&mut model);
    model._set("B5", "blocker");
    model._set("A4", "=TRANSPOSE(C1:E2)");
    model.evaluate();

    assert_eq!(model._get_text("A4"), *"#SPILL!");
    assert_eq!(model._get_text("B5"), *"blocker");
    assert_eq!(model._get_text("A5"), *"");
}
