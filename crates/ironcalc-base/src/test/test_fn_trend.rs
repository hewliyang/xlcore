#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_trend_predict_new_xs() {
    let mut model = new_empty_model();
    for (i, v) in [2, 4, 6, 8, 10].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
        model._set(&format!("Y{}", i + 1), &(i + 1).to_string());
    }
    model._set("W1", "6");
    model._set("W2", "7");
    model._set("A1", "=TREND(Z1:Z5,Y1:Y5,W1:W2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"12");
    assert_eq!(model._get_text("A2"), *"14");
}

#[test]
fn fn_trend_known_xs_default() {
    let mut model = new_empty_model();
    for (i, v) in [2, 4, 6, 8, 10].iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
    }
    model._set("A1", "=TREND(Z1:Z5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"2");
    assert_eq!(model._get_text("A5"), *"10");
}

#[test]
fn fn_trend_arg_count() {
    let mut model = new_empty_model();
    model._set("A1", "=TREND()");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}
