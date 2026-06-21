#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_permut_basic() {
    let mut model = new_empty_model();

    model._set("A1", "=PERMUT(5, 2)");
    model._set("A2", "=PERMUT(10, 3)");
    model._set("A3", "=PERMUT(4, 0)");
    model._set("A4", "=PERMUT(3, 3)");
    model._set("A5", "=PERMUT(5.9, 2.9)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), "20");
    assert_eq!(model._get_text("A2"), "720");
    assert_eq!(model._get_text("A3"), "1");
    assert_eq!(model._get_text("A4"), "6");
    assert_eq!(model._get_text("A5"), "20");
}

#[test]
fn fn_permut_errors() {
    let mut model = new_empty_model();

    model._set("A1", "=PERMUT(3, 4)");
    model._set("A2", "=PERMUT(-1, 2)");
    model._set("A3", "=PERMUT(5)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), "#NUM!");
    assert_eq!(model._get_text("A2"), "#NUM!");
    assert_eq!(model._get_text("A3"), "#ERROR!");
}

#[test]
fn fn_prob_basic() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("A4", "4");
    model._set("B1", "0.1");
    model._set("B2", "0.2");
    model._set("B3", "0.3");
    model._set("B4", "0.4");

    model._set("C1", "=PROB(A1:A4, B1:B4, 3)");
    model._set("C2", "=PROB(A1:A4, B1:B4, 2, 4)");
    model._set("C3", "=PROB(A1:A4, B1:B4, 1, 2)");
    model._set("C4", "=PROB(A1:A4, B1:B4, 4, 2)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), "0.3");
    assert_eq!(model._get_text("C2"), "0.9");
    assert_eq!(model._get_text("C3"), "0.3");
    assert_eq!(model._get_text("C4"), "0.9");
}

#[test]
fn fn_prob_errors() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("B1", "0.1");
    model._set("B2", "0.2");
    model._set("B3", "0.3");

    model._set("C1", "=PROB(A1:A3, B1:B3, 2)");
    model._set("C2", "=PROB(A1:A3, B1:B2, 2)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), "#NUM!");
    assert_eq!(model._get_text("C2"), "#N/A");
}

#[test]
fn fn_trimmean_basic() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("A4", "4");
    model._set("A5", "5");
    model._set("A6", "6");
    model._set("A7", "7");
    model._set("A8", "8");
    model._set("A9", "9");
    model._set("A10", "100");

    model._set("B1", "=TRIMMEAN(A1:A10, 0.2)");
    model._set("B2", "=TRIMMEAN(A1:A10, 0)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "5.5");
    assert_eq!(model._get_text("B2"), "14.5");
}

#[test]
fn fn_trimmean_errors() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");

    model._set("B1", "=TRIMMEAN(A1:A2, 1)");
    model._set("B2", "=TRIMMEAN(A1:A2, -0.1)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "#NUM!");
    assert_eq!(model._get_text("B2"), "#NUM!");
}
