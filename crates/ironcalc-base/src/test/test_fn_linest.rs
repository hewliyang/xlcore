#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_linest_simple_slope_intercept() {
    let mut model = new_empty_model();
    for (i, v) in [1, 2, 3, 4, 5].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
        model._set(&format!("Y{}", i + 1), &v.to_string());
    }
    model._set("A1", "=LINEST(Z1:Z5,Y1:Y5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"0");
}

#[test]
fn fn_linest_stats() {
    let mut model = new_empty_model();
    for (i, v) in [1, 2, 3, 4, 5].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
        model._set(&format!("Y{}", i + 1), &v.to_string());
    }
    model._set("A1", "=LINEST(Z1:Z5,Y1:Y5,TRUE,TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"0");
    assert_eq!(model._get_text("A3"), *"1");
    assert_eq!(model._get_text("B4"), *"3");
    assert_eq!(model._get_text("A5"), *"10");
}

#[test]
fn fn_linest_multiple_regression() {
    let mut model = new_empty_model();
    let rows = [
        (1.0, 1.0, 9.0),
        (2.0, 1.0, 12.0),
        (3.0, 2.0, 20.0),
        (4.0, 2.0, 23.0),
        (5.0, 3.0, 31.0),
    ];
    for (i, (x1, x2, y)) in rows.iter().enumerate() {
        model._set(&format!("W{}", i + 1), &x1.to_string());
        model._set(&format!("X{}", i + 1), &x2.to_string());
        model._set(&format!("Z{}", i + 1), &y.to_string());
    }
    model._set("A1", "=LINEST(Z1:Z5,W1:X5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"5");
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("C1"), *"1");
}

#[test]
fn fn_linest_arg_count() {
    let mut model = new_empty_model();
    model._set("A1", "=LINEST()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}
