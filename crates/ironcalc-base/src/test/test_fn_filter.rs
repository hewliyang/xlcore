#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_filter_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=FILTER()");
    model._set("A2", "=FILTER(C1:C3)");
    model._set("A3", "=FILTER(C1:C3, D1:D3, 1, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
    assert_eq!(model._get_text("A3"), *"#ERROR!");
}

#[test]
fn fn_filter_rows() {
    let mut model = new_empty_model();
    model._set("C1", "apple");
    model._set("D1", "10");
    model._set("C2", "banana");
    model._set("D2", "20");
    model._set("C3", "cherry");
    model._set("D3", "30");
    model._set("C4", "date");
    model._set("D4", "40");
    model._set("E1", "TRUE");
    model._set("E2", "FALSE");
    model._set("E3", "TRUE");
    model._set("E4", "FALSE");
    model._set("A1", "=FILTER(C1:D4, E1:E4)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"apple");
    assert_eq!(model._get_text("B1"), *"10");
    assert_eq!(model._get_text("A2"), *"cherry");
    assert_eq!(model._get_text("B2"), *"30");
    assert_eq!(model._get_text("A3"), *"");
    assert_eq!(model._get_text("B3"), *"");
}

#[test]
fn fn_filter_cols() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("D1", "2");
    model._set("E1", "3");
    model._set("C2", "a");
    model._set("D2", "b");
    model._set("E2", "c");
    model._set("C4", "TRUE");
    model._set("D4", "FALSE");
    model._set("E4", "TRUE");
    model._set("A6", "=FILTER(C1:E2, C4:E4)");
    model.evaluate();

    assert_eq!(model._get_text("A6"), *"1");
    assert_eq!(model._get_text("B6"), *"3");
    assert_eq!(model._get_text("A7"), *"a");
    assert_eq!(model._get_text("B7"), *"c");
    assert_eq!(model._get_text("C6"), *"");
}

#[test]
fn fn_filter_no_match_if_empty() {
    let mut model = new_empty_model();
    model._set("C1", "apple");
    model._set("C2", "banana");
    model._set("E1", "FALSE");
    model._set("E2", "FALSE");
    model._set("A1", "=FILTER(C1:C2, E1:E2, \"none\")");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"none");
}

#[test]
fn fn_filter_no_match_calc() {
    let mut model = new_empty_model();
    model._set("C1", "apple");
    model._set("C2", "banana");
    model._set("E1", "FALSE");
    model._set("E2", "FALSE");
    model._set("A1", "=FILTER(C1:C2, E1:E2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#CALC!");
}

#[test]
fn fn_filter_dimension_mismatch() {
    let mut model = new_empty_model();
    model._set("C1", "apple");
    model._set("C2", "banana");
    model._set("C3", "cherry");
    model._set("E1", "TRUE");
    model._set("E2", "FALSE");
    model._set("A1", "=FILTER(C1:C3, E1:E2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
