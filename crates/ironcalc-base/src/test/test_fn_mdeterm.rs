#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_mdeterm_basic() {
    let mut model = new_empty_model();

    model._set("A1", "5");
    model._set("B1", "=MDETERM(A1)");

    model._set("A3", "1");
    model._set("B3", "2");
    model._set("A4", "3");
    model._set("B4", "4");
    model._set("C3", "=MDETERM(A3:B4)");

    model._set("A6", "6");
    model._set("B6", "1");
    model._set("C6", "1");
    model._set("A7", "4");
    model._set("B7", "-2");
    model._set("C7", "5");
    model._set("A8", "2");
    model._set("B8", "8");
    model._set("C8", "7");
    model._set("D6", "=MDETERM(A6:C8)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), *"5");
    assert_eq!(model._get_text("C3"), *"-2");
    assert_eq!(model._get_text("D6"), *"-306");
}

#[test]
fn fn_mdeterm_singular() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("B1", "2");
    model._set("A2", "2");
    model._set("B2", "4");
    model._set("C1", "=MDETERM(A1:B2)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"0");
}

#[test]
fn fn_mdeterm_errors() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("B1", "2");
    model._set("C1", "3");
    model._set("A2", "4");
    model._set("B2", "5");
    model._set("C2", "6");
    model._set("D1", "=MDETERM(A1:C2)");

    model._set("A4", "1");
    model._set("B4", "x");
    model._set("A5", "3");
    model._set("B5", "4");
    model._set("C4", "=MDETERM(A4:B5)");

    model.evaluate();

    assert_eq!(model._get_text("D1"), *"#VALUE!");
    assert_eq!(model._get_text("C4"), *"#VALUE!");
}
