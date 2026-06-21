#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_logest_simple() {
    let mut model = new_empty_model();
    for (i, v) in [2, 4, 8, 16, 32].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
        model._set(&format!("Y{}", i + 1), &(i + 1).to_string());
    }
    model._set("A1", "=LOGEST(Z1:Z5,Y1:Y5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("B1"), *"1");
}

#[test]
fn fn_logest_stats() {
    let mut model = new_empty_model();
    for (i, v) in [2, 4, 8, 16, 32].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
        model._set(&format!("Y{}", i + 1), &(i + 1).to_string());
    }
    model._set("A1", "=LOGEST(Z1:Z5,Y1:Y5,TRUE,TRUE)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("B1"), *"1");
    assert_eq!(model._get_text("A3"), *"1");
}

#[test]
fn fn_logest_non_positive() {
    let mut model = new_empty_model();
    for (i, v) in [2, -4, 8, 16, 32].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
        model._set(&format!("Y{}", i + 1), &(i + 1).to_string());
    }
    model._set("A1", "=LOGEST(Z1:Z5,Y1:Y5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
}

#[test]
fn fn_logest_arg_count() {
    let mut model = new_empty_model();
    model._set("A1", "=LOGEST()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}
