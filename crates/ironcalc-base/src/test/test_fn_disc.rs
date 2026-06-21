#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_disc_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=DISC(1,2,3)");
    model._set("A2", "=DISC(1,2,3,4,5,6)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_disc_basic() {
    let mut model = new_empty_model();
    model._set("A1", "=DISC(DATE(2007,1,25),DATE(2007,6,15),97.975,100,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"0.052420213");
}

#[test]
fn fn_disc_settlement_ge_maturity() {
    let mut model = new_empty_model();
    model._set("A1", "=DISC(DATE(2007,6,15),DATE(2007,1,25),97.975,100,1)");
    model._set("A2", "=DISC(DATE(2007,1,25),DATE(2007,1,25),97.975,100,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
    assert_eq!(model._get_text("A2"), *"#NUM!");
}

#[test]
fn fn_disc_bad_basis() {
    let mut model = new_empty_model();
    model._set("A1", "=DISC(DATE(2007,1,25),DATE(2007,6,15),97.975,100,5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#NUM!");
}

#[test]
fn fn_disc_bad_date() {
    let mut model = new_empty_model();
    model._set("A1", "=DISC(\"x\",DATE(2007,6,15),97.975,100,1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_intrate_basic() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=INTRATE(DATE(2008,2,15),DATE(2008,5,15),1000000,1014420,2)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"0.05768");
}

#[test]
fn fn_received_basic() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=RECEIVED(DATE(2008,2,15),DATE(2008,5,15),1000000,0.0575,2)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"1014584.654407102");
}

#[test]
fn fn_pricedisc_basic() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=PRICEDISC(DATE(2008,2,16),DATE(2008,3,1),0.0525,100,2)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"99.795833333");
}

#[test]
fn fn_yielddisc_basic() {
    let mut model = new_empty_model();
    model._set(
        "A1",
        "=YIELDDISC(DATE(2008,2,16),DATE(2008,3,1),99.795,100,2)",
    );
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"0.052822572");
}

#[test]
fn fn_disc_default_basis() {
    let mut model = new_empty_model();
    model._set("A1", "=DISC(DATE(2007,1,25),DATE(2007,6,15),97.975,100)");
    model._set("A2", "=DISC(DATE(2007,1,25),DATE(2007,6,15),97.975,100,0)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}
