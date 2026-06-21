#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_aggregate_sum() {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("A4", "4");
    model._set("A5", "5");
    model._set("B1", "=AGGREGATE(9,4,A1:A5)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"15");
}

#[test]
fn fn_aggregate_average_ignore_error() {
    let mut model = new_empty_model();
    model._set("A1", "10");
    model._set("A2", "=1/0");
    model._set("A3", "20");
    model._set("A4", "30");
    model._set("B1", "=AGGREGATE(1,6,A1:A4)");
    model._set("B2", "=AGGREGATE(1,4,A1:A4)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"20");
    assert_eq!(model._get_text("B2"), *"#DIV/0!");
}

#[test]
fn fn_aggregate_large_ignore_error() {
    let mut model = new_empty_model();
    model._set("A1", "5");
    model._set("A2", "=1/0");
    model._set("A3", "8");
    model._set("A4", "3");
    model._set("A5", "10");
    model._set("B1", "=AGGREGATE(14,6,A1:A5,2)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"8");
}

#[test]
fn fn_aggregate_max_ignore_error() {
    let mut model = new_empty_model();
    model._set("A1", "5");
    model._set("A2", "=1/0");
    model._set("A3", "8");
    model._set("A4", "3");
    model._set("B1", "=AGGREGATE(4,6,A1:A4)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"8");
}

#[test]
fn fn_aggregate_invalid_args() {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("B1", "=AGGREGATE(20,0,A1)");
    model._set("B2", "=AGGREGATE(9,8,A1)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"#VALUE!");
    assert_eq!(model._get_text("B2"), *"#VALUE!");
}
