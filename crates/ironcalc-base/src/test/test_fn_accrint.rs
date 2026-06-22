#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_accrintm() {
    let mut model = new_empty_model();

    model._set("A1", "=ACCRINTM(DATE(2008,4,1),DATE(2008,6,15),0.1,1000,3)");

    model.evaluate();

    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 20.5479).abs() < 1e-3);
}

#[test]
fn fn_accrint() {
    let mut model = new_empty_model();

    model._set(
        "A1",
        "=ACCRINT(DATE(2008,3,1),DATE(2008,8,31),DATE(2008,5,1),0.1,1000,2,0)",
    );

    model.evaluate();

    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 16.66667).abs() < 1e-3);
}

#[test]
fn fn_pricemat() {
    let mut model = new_empty_model();

    model._set(
        "A1",
        "=PRICEMAT(DATE(2008,2,15),DATE(2008,4,13),DATE(2007,11,11),0.061,0.061,0)",
    );

    model.evaluate();

    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 99.98449).abs() < 1e-3);
}

#[test]
fn fn_yieldmat() {
    let mut model = new_empty_model();

    model._set(
        "A1",
        "=YIELDMAT(DATE(2008,3,15),DATE(2008,11,3),DATE(2007,11,8),0.0625,100.0123,0)",
    );

    model.evaluate();

    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 0.060954).abs() < 1e-5);
}

#[test]
fn fn_accrint_errors() {
    let mut model = new_empty_model();

    model._set("A1", "=ACCRINTM(DATE(2008,6,15),DATE(2008,4,1),0.1,1000,3)");
    model._set(
        "A2",
        "=ACCRINT(DATE(2008,3,1),DATE(2008,8,31),DATE(2008,5,1),0.1,1000,3,0)",
    );

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#NUM!");
    assert_eq!(model._get_text("A2"), *"#NUM!");
}
