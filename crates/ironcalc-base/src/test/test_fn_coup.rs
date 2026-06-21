#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_coup_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPNUM(DATE(2011,1,25))");
    model._set("A2", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),2,1,9)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_coup_example() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPDAYBS(DATE(2011,1,25),DATE(2011,11,15),2,1)");
    model._set("A2", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),2,1)");
    model._set("A3", "=COUPDAYSNC(DATE(2011,1,25),DATE(2011,11,15),2,1)");
    model._set("A4", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),2,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"71");
    assert_eq!(model._get_text("A2"), *"181");
    assert_eq!(model._get_text("A3"), *"110");
    assert_eq!(model._get_text("A4"), *"2");
}

#[test]
fn fn_coup_pcd_ncd_dates() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPPCD(DATE(2011,1,25),DATE(2011,11,15),2,1)");
    model._set("A3", "=COUPNCD(DATE(2011,1,25),DATE(2011,11,15),2,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"40497");
    assert_eq!(model._get_text("A3"), *"40678");
}

#[test]
fn fn_coup_freq_basis_validation() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),3,1)");
    model._set("A2", "=COUPNUM(DATE(2011,1,25),DATE(2011,11,15),2,5)");
    model._set("A3", "=COUPNUM(DATE(2011,11,15),DATE(2011,1,25),2,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
    assert_eq!(model._get_text("A2"), *"#NUM!");
    assert_eq!(model._get_text("A3"), *"#NUM!");
}

#[test]
fn fn_coup_bad_date() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPNUM(\"x\",DATE(2011,11,15),2,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_coup_default_basis() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPDAYBS(DATE(2011,1,25),DATE(2011,11,15),2)");
    model._set("A2", "=COUPDAYBS(DATE(2011,1,25),DATE(2011,11,15),2,0)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_coup_basis_360() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),2,0)");
    model._set("A2", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),4,0)");
    model._set("A3", "=COUPDAYS(DATE(2011,1,25),DATE(2011,11,15),2,3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"180");
    assert_eq!(model._get_text("A2"), *"90");
    assert_eq!(model._get_text("A3"), *"182.5");
}

#[test]
fn fn_coup_end_of_month() {
    let mut model = new_empty_model();
    model._set("A1", "=COUPPCD(DATE(2011,1,25),DATE(2012,2,29),2,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"40421");
}
