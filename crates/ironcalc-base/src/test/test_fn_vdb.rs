#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_vdb_first_day() {
    let mut model = new_empty_model();
    model._set("A1", "=VDB(2400,300,3650,0,1,2,FALSE)");
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 1.32).abs() < 1e-2);
}

#[test]
fn fn_vdb_fractional() {
    let mut model = new_empty_model();
    model._set("A1", "=VDB(2400,300,10,0,0.875,1.5)");
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 315.0).abs() < 1e-2);
}

#[test]
fn fn_vdb_with_switch() {
    let mut model = new_empty_model();
    model._set("A1", "=VDB(2400,300,10,6,10)");
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 329.1456).abs() < 1e-2);
}

#[test]
fn fn_amorlinc_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=AMORLINC(2400,DATE(2008,8,19),DATE(2008,12,31),300,1,0.15,1)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 360.0).abs() < 1e-6);
}

#[test]
fn fn_amordegrc_example() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=AMORDEGRC(2400,DATE(2008,8,19),DATE(2008,12,31),300,1,0.15,1)",
    );
    model.evaluate();
    let v: f64 = model._get_text("A1").parse().unwrap();
    assert!((v - 776.0).abs() < 1.0);
}
