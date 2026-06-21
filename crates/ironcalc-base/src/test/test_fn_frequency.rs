#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_frequency_classic() {
    let mut model = new_empty_model();
    let data = [79, 85, 78, 85, 50, 81, 95, 88, 97];
    for (i, v) in data.iter().enumerate() {
        model._set(&format!("Y{}", i + 1), &v.to_string());
    }
    let bins = [70, 79, 89];
    for (i, v) in bins.iter().enumerate() {
        model._set(&format!("Z{}", i + 1), &v.to_string());
    }
    model._set("AB1", "=FREQUENCY(Y1:Y9,Z1:Z3)");
    model.evaluate();
    assert_eq!(model._get_text("AB1"), *"1");
    assert_eq!(model._get_text("AB2"), *"2");
    assert_eq!(model._get_text("AB3"), *"4");
    assert_eq!(model._get_text("AB4"), *"2");
}

#[test]
fn fn_frequency_ignores_non_numeric() {
    let mut model = new_empty_model();
    model._set("Y1", "5");
    model._set("Y2", "hello");
    model._set("Y3", "15");
    model._set("Y4", "25");
    model._set("Z1", "10");
    model._set("Z2", "20");
    model._set("AB1", "=FREQUENCY(Y1:Y4,Z1:Z2)");
    model.evaluate();
    assert_eq!(model._get_text("AB1"), *"1");
    assert_eq!(model._get_text("AB2"), *"1");
    assert_eq!(model._get_text("AB3"), *"1");
}

#[test]
fn fn_frequency_args_number() {
    let mut model = new_empty_model();
    model._set("AB1", "=FREQUENCY(Y1:Y9)");
    model.evaluate();
    assert_eq!(model._get_text("AB1"), *"#ERROR!");
}
