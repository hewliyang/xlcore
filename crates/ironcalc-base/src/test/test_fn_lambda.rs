#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_lambda_immediate() {
    let mut model = new_empty_model();

    model._set("A1", "=LAMBDA(x,x+1)(5)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"6");
}

#[test]
fn fn_lambda_two_params() {
    let mut model = new_empty_model();

    model._set("A1", "=LAMBDA(x,y,x*y)(3,4)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"12");
}

#[test]
fn fn_lambda_closure_over_let() {
    let mut model = new_empty_model();

    model._set("A1", "=LET(n,10,LAMBDA(x,x+n))(5)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"15");
}

#[test]
fn fn_lambda_isomitted() {
    let mut model = new_empty_model();

    model._set("A1", "=LAMBDA(x,y,IF(ISOMITTED(y),x,x+y))(5)");
    model._set("A2", "=LAMBDA(x,y,IF(ISOMITTED(y),x,x+y))(5,2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"5");
    assert_eq!(model._get_text("A2"), *"7");
}

#[test]
fn fn_lambda_wrong_arity() {
    let mut model = new_empty_model();

    model._set("A1", "=LAMBDA(x,x+1)(5,6)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_lambda_uninvoked() {
    let mut model = new_empty_model();

    model._set("A1", "=LAMBDA(x,x+1)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#CALC!");
}

#[test]
fn fn_lambda_named() {
    let mut model = new_empty_model();

    model
        .new_defined_name("MYFN", None, "=LAMBDA(x,x*x)")
        .unwrap();
    model._set("A1", "=MYFN(4)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"16");
}

#[test]
fn fn_lambda_formula_roundtrip() {
    let mut model = new_empty_model();
    model._set("A1", "=LAMBDA(x,x+1)(5)");
    model.evaluate();
    assert_eq!(model._get_formula("A1"), *"=LAMBDA(x,x+1)(5)");
}
