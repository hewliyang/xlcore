#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn grid_model() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("B1", "2");
    model._set("C1", "3");
    model._set("A2", "4");
    model._set("B2", "5");
    model._set("C2", "6");
    model
}

#[test]
fn fn_byrow_sum() {
    let mut model = grid_model();
    model._set("E1", "=BYROW(A1:C2,LAMBDA(r,SUM(r)))");

    model.evaluate();

    assert_eq!(model._get_text("E1"), *"6");
    assert_eq!(model._get_text("E2"), *"15");
}

#[test]
fn fn_bycol_sum() {
    let mut model = grid_model();
    model._set("E1", "=BYCOL(A1:C2,LAMBDA(c,SUM(c)))");

    model.evaluate();

    assert_eq!(model._get_text("E1"), *"5");
    assert_eq!(model._get_text("F1"), *"7");
    assert_eq!(model._get_text("G1"), *"9");
}

#[test]
fn fn_makearray_product() {
    let mut model = new_empty_model();
    model._set("A1", "=MAKEARRAY(2,3,LAMBDA(i,j,i*j))");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("C1"), *"3");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("B2"), *"4");
    assert_eq!(model._get_text("C2"), *"6");
}

#[test]
fn fn_makearray_index() {
    let mut model = new_empty_model();
    model._set("A1", "=MAKEARRAY(2,2,LAMBDA(i,j,(i-1)*2+j))");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("B1"), *"2");
    assert_eq!(model._get_text("A2"), *"3");
    assert_eq!(model._get_text("B2"), *"4");
}

#[test]
fn fn_makearray_wrong_arity() {
    let mut model = new_empty_model();
    model._set("A1", "=MAKEARRAY(2,2)");
    model._set("A3", "=BYROW(A1:C2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A3"), *"#ERROR!");
}

#[test]
fn fn_byrow_non_lambda() {
    let mut model = grid_model();
    model._set("E1", "=BYROW(A1:C2,5)");

    model.evaluate();

    assert_eq!(model._get_text("E1"), *"#VALUE!");
}

#[test]
fn fn_makearray_bad_dims() {
    let mut model = new_empty_model();
    model._set("A1", "=MAKEARRAY(0,2,LAMBDA(i,j,i))");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
