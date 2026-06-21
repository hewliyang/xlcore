#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_percentile_inc_basic() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("A4", "4");

    model._set("B1", "=PERCENTILE(A1:A4, 0.25)");
    model._set("B2", "=PERCENTILE.INC(A1:A4, 0.25)");
    model._set("B3", "=PERCENTILE.INC(A1:A4, 0.5)");
    model._set("B4", "=PERCENTILE.INC(A1:A4, 0)");
    model._set("B5", "=PERCENTILE.INC(A1:A4, 1)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "1.75");
    assert_eq!(model._get_text("B2"), "1.75");
    assert_eq!(model._get_text("B3"), "2.5");
    assert_eq!(model._get_text("B4"), "1");
    assert_eq!(model._get_text("B5"), "4");
}

#[test]
fn fn_percentile_exc_basic() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("A4", "4");

    model._set("B1", "=PERCENTILE.EXC(A1:A4, 0.25)");
    model._set("B2", "=PERCENTILE.EXC(A1:A4, 0.5)");
    model._set("B3", "=PERCENTILE.EXC(A1:A4, 0.1)");
    model._set("B4", "=PERCENTILE.EXC(A1:A4, 0.9)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "1.25");
    assert_eq!(model._get_text("B2"), "2.5");
    assert_eq!(model._get_text("B3"), "#NUM!");
    assert_eq!(model._get_text("B4"), "#NUM!");
}

#[test]
fn fn_quartile_basic() {
    let mut model = new_empty_model();

    for i in 1..=8 {
        model._set(&format!("A{i}"), &i.to_string());
    }

    model._set("B1", "=QUARTILE(A1:A8, 0)");
    model._set("B2", "=QUARTILE(A1:A8, 1)");
    model._set("B3", "=QUARTILE.INC(A1:A8, 2)");
    model._set("B4", "=QUARTILE(A1:A8, 3)");
    model._set("B5", "=QUARTILE(A1:A8, 4)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "1");
    assert_eq!(model._get_text("B2"), "2.75");
    assert_eq!(model._get_text("B3"), "4.5");
    assert_eq!(model._get_text("B4"), "6.25");
    assert_eq!(model._get_text("B5"), "8");
}

#[test]
fn fn_quartile_exc_basic() {
    let mut model = new_empty_model();

    for i in 1..=8 {
        model._set(&format!("A{i}"), &i.to_string());
    }

    model._set("B1", "=QUARTILE.EXC(A1:A8, 1)");
    model._set("B2", "=QUARTILE.EXC(A1:A8, 2)");
    model._set("B3", "=QUARTILE.EXC(A1:A8, 3)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "2.25");
    assert_eq!(model._get_text("B2"), "4.5");
    assert_eq!(model._get_text("B3"), "6.75");
}

#[test]
fn fn_quartile_invalid() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("B1", "=QUARTILE(A1:A2, 5)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "#NUM!");
}
