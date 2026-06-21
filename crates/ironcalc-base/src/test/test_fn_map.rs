#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_map_square() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("C1", "=MAP(A1:A3,LAMBDA(x,x*x))");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"1");
    assert_eq!(model._get_text("C2"), *"4");
    assert_eq!(model._get_text("C3"), *"9");
}

#[test]
fn fn_map_two_arrays() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("B1", "10");
    model._set("B2", "20");
    model._set("B3", "30");
    model._set("C1", "=MAP(A1:A3,B1:B3,LAMBDA(a,b,a+b))");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"11");
    assert_eq!(model._get_text("C2"), *"22");
    assert_eq!(model._get_text("C3"), *"33");
}

#[test]
fn fn_reduce_sum() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("C1", "=REDUCE(0,A1:A3,LAMBDA(a,x,a+x))");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"6");
}

#[test]
fn fn_reduce_product() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("C1", "=REDUCE(1,A1:A3,LAMBDA(a,x,a*x))");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"6");
}

#[test]
fn fn_scan_running_sum() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("C1", "=SCAN(0,A1:A3,LAMBDA(a,x,a+x))");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"1");
    assert_eq!(model._get_text("C2"), *"3");
    assert_eq!(model._get_text("C3"), *"6");
}

#[test]
fn fn_map_wrong_arity() {
    let mut model = new_empty_model();

    model._set("A1", "=MAP(LAMBDA(x,x))");
    model._set("A2", "=REDUCE(0,LAMBDA(a,x,a+x))");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_map_non_lambda() {
    let mut model = new_empty_model();

    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "3");
    model._set("C1", "=MAP(A1:A3,5)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), *"#VALUE!");
}
