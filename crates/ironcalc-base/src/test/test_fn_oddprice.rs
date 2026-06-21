#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_oddlprice_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=ODDLPRICE(DATE(2008,2,7),DATE(2008,6,15),DATE(2007,10,15),0.0375,0.0405,100,2,0)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 99.87827).abs() < 1e-3);
}

#[test]
fn fn_oddlyield_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=ODDLYIELD(DATE(2008,4,20),DATE(2008,6,15),DATE(2007,12,24),0.0375,99.875,100,2,0)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 0.045192).abs() < 1e-5);
}

#[test]
fn fn_oddfprice_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=ODDFPRICE(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0785,0.0625,100,2,1)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 113.59792).abs() < 1e-3);
}

#[test]
fn fn_oddfyield_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=ODDFYIELD(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0785,113.597717,100,2,1)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 0.0625).abs() < 1e-4);
}

#[test]
fn fn_oddprice_errors() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=ODDLPRICE(DATE(2008,2,7),DATE(2008,6,15),DATE(2007,10,15),0.0375,0.0405,100,3,0)",
    );
    model._set(
        "A2",
        "=ODDFPRICE(DATE(2008,11,11),DATE(2021,3,1),DATE(2008,10,15),DATE(2009,3,1),0.0785,0.0625,100,2,7)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
    assert_eq!(model._get_text("A2"), *"#NUM!");
}
