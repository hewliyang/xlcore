#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_duration_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=DURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,0.09,2,1)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 5.993775).abs() < 1e-4);
}

#[test]
fn fn_mduration_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=MDURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,0.09,2,1)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 5.735670).abs() < 1e-4);
}

#[test]
fn fn_duration_default_basis() {
    let mut model = new_empty_model();
    model._set("A1", "=DURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,0.09,2)");
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!(v > 5.9 && v < 6.1);
}

#[test]
fn fn_duration_errors() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=DURATION(DATE(2016,1,1),DATE(2008,1,1),0.08,0.09,2,1)",
    );
    model._set(
        "A2",
        "=DURATION(DATE(2008,1,1),DATE(2016,1,1),-0.01,0.09,2,1)",
    );
    model._set(
        "A3",
        "=MDURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,-0.01,2,1)",
    );
    model._set(
        "A4",
        "=DURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,0.09,3,1)",
    );
    model._set(
        "A5",
        "=DURATION(DATE(2008,1,1),DATE(2016,1,1),0.08,0.09,2,5)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
    assert_eq!(model._get_text("A2"), *"#NUM!");
    assert_eq!(model._get_text("A3"), *"#NUM!");
    assert_eq!(model._get_text("A4"), *"#NUM!");
    assert_eq!(model._get_text("A5"), *"#NUM!");
}
