#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_percentrank_inc_basic() {
    let mut model = new_empty_model();

    let data = [13, 12, 11, 8, 4, 3, 2, 1, 1, 1];
    for (i, v) in data.iter().enumerate() {
        model._set(&format!("A{}", i + 1), &v.to_string());
    }

    model._set("B1", "=PERCENTRANK(A1:A10, 2)");
    model._set("B2", "=PERCENTRANK.INC(A1:A10, 4)");
    model._set("B3", "=PERCENTRANK.INC(A1:A10, 8)");
    model._set("B4", "=PERCENTRANK.INC(A1:A10, 5)");
    model._set("B5", "=PERCENTRANK.INC(A1:A10, 1)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "0.333");
    assert_eq!(model._get_text("B2"), "0.555");
    assert_eq!(model._get_text("B3"), "0.666");
    assert_eq!(model._get_text("B4"), "0.583");
    assert_eq!(model._get_text("B5"), "0");
}

#[test]
fn fn_percentrank_significance() {
    let mut model = new_empty_model();

    let data = [13, 12, 11, 8, 4, 3, 2, 1, 1, 1];
    for (i, v) in data.iter().enumerate() {
        model._set(&format!("A{}", i + 1), &v.to_string());
    }

    model._set("B1", "=PERCENTRANK(A1:A10, 5, 1)");
    model._set("B2", "=PERCENTRANK(A1:A10, 5, 2)");
    model._set("B3", "=PERCENTRANK(A1:A10, 5, 4)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "0.5");
    assert_eq!(model._get_text("B2"), "0.58");
    assert_eq!(model._get_text("B3"), "0.5833");
}

#[test]
fn fn_percentrank_out_of_range() {
    let mut model = new_empty_model();

    for i in 1..=4 {
        model._set(&format!("A{i}"), &i.to_string());
    }

    model._set("B1", "=PERCENTRANK(A1:A4, 0)");
    model._set("B2", "=PERCENTRANK(A1:A4, 5)");
    model._set("B3", "=PERCENTRANK(A1:A4, 1, 0)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "#N/A");
    assert_eq!(model._get_text("B2"), "#N/A");
    assert_eq!(model._get_text("B3"), "#NUM!");
}

#[test]
fn fn_percentrank_exc_basic() {
    let mut model = new_empty_model();

    for i in 1..=4 {
        model._set(&format!("A{i}"), &i.to_string());
    }

    model._set("B1", "=PERCENTRANK.EXC(A1:A4, 1)");
    model._set("B2", "=PERCENTRANK.EXC(A1:A4, 2)");
    model._set("B3", "=PERCENTRANK.EXC(A1:A4, 4)");
    model._set("B4", "=PERCENTRANK.EXC(A1:A4, 2.5)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "0.2");
    assert_eq!(model._get_text("B2"), "0.4");
    assert_eq!(model._get_text("B3"), "0.8");
    assert_eq!(model._get_text("B4"), "0.5");
}
