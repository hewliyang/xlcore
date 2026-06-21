#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_price_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=PRICE(DATE(2008,2,15),DATE(2017,11,15),0.0575,0.065,100,2,0)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 94.6343).abs() < 1e-3);
}

#[test]
fn fn_yield_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=YIELD(DATE(2008,2,15),DATE(2016,11,15),0.0575,95.04287,100,2,0)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 0.0650).abs() < 1e-4);
}

#[test]
fn fn_price_yield_roundtrip() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=PRICE(DATE(2008,2,15),DATE(2017,11,15),0.0575,0.065,100,2,0)",
    );
    model._set(
        "A2",
        "=YIELD(DATE(2008,2,15),DATE(2017,11,15),0.0575,A1,100,2,0)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A2").parse().unwrap();
    assert!((v - 0.065).abs() < 1e-5);
}

#[test]
fn fn_price_errors() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=PRICE(DATE(2017,11,15),DATE(2008,2,15),0.0575,0.065,100,2,0)",
    );
    model._set(
        "A2",
        "=PRICE(DATE(2008,2,15),DATE(2017,11,15),-0.01,0.065,100,2,0)",
    );
    model._set(
        "A3",
        "=YIELD(DATE(2008,2,15),DATE(2017,11,15),0.0575,0,100,2,0)",
    );
    model._set(
        "A4",
        "=PRICE(DATE(2008,2,15),DATE(2017,11,15),0.0575,0.065,100,3,0)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
    assert_eq!(model._get_text("A2"), *"#NUM!");
    assert_eq!(model._get_text("A3"), *"#NUM!");
    assert_eq!(model._get_text("A4"), *"#NUM!");
}
