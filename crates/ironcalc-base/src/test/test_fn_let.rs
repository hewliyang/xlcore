#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_let_simple() {
    let mut model = new_empty_model();

    model._set("A1", "=LET(x,5,x*2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"10");
}

#[test]
fn fn_let_chained_bindings() {
    let mut model = new_empty_model();

    model._set("A1", "=LET(x,1,y,x+1,x+y)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3");
}

#[test]
fn fn_let_nested() {
    let mut model = new_empty_model();

    model._set("A1", "=LET(x,2,LET(y,3,x*y))");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"6");
}

#[test]
fn fn_let_range_binding() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("B1", "=LET(d,A1:A3,SUM(d))");

    model.evaluate();

    assert_eq!(model._get_text("B1"), *"6");
}

#[test]
fn fn_let_wrong_arity() {
    let mut model = new_empty_model();

    model._set("A1", "=LET(x,5)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_let_undefined_var() {
    let mut model = new_empty_model();

    model._set("A1", "=LET(x,5,zzz)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#NAME?");
}
