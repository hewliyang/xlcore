#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_unique_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=UNIQUE()");
    model._set("A2", "=UNIQUE(C1:C3, FALSE, FALSE, 9)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_unique_text_column() {
    let mut model = new_empty_model();
    model._set("C1", "banana");
    model._set("C2", "apple");
    model._set("C3", "banana");
    model._set("C4", "cherry");
    model._set("C5", "apple");
    model._set("A1", "=UNIQUE(C1:C5)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"banana");
    assert_eq!(model._get_text("A2"), *"apple");
    assert_eq!(model._get_text("A3"), *"cherry");
}

#[test]
fn fn_unique_case_insensitive() {
    let mut model = new_empty_model();
    model._set("C1", "Apple");
    model._set("C2", "apple");
    model._set("C3", "APPLE");
    model._set("A1", "=UNIQUE(C1:C3)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"Apple");
    assert_eq!(model._get_text("A2"), *"");
    assert_eq!(model._get_text("A3"), *"");
}

#[test]
fn fn_unique_exactly_once() {
    let mut model = new_empty_model();
    model._set("C1", "banana");
    model._set("C2", "apple");
    model._set("C3", "banana");
    model._set("C4", "cherry");
    model._set("A1", "=UNIQUE(C1:C4, FALSE, TRUE)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"apple");
    assert_eq!(model._get_text("A2"), *"cherry");
}

#[test]
fn fn_unique_whole_row() {
    let mut model = new_empty_model();
    model._set("C1", "a");
    model._set("D1", "1");
    model._set("C2", "a");
    model._set("D2", "2");
    model._set("C3", "a");
    model._set("D3", "1");
    model._set("A1", "=UNIQUE(C1:D3)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"1");
    assert_eq!(model._get_text("A2"), *"a");
    assert_eq!(model._get_text("B2"), *"2");
    assert_eq!(model._get_text("A3"), *"");
    assert_eq!(model._get_text("B3"), *"");
}

#[test]
fn fn_unique_by_col() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("D1", "2");
    model._set("E1", "1");
    model._set("C2", "x");
    model._set("D2", "y");
    model._set("E2", "x");
    model._set("A5", "=UNIQUE(C1:E2, TRUE)");
    model.evaluate();

    assert_eq!(model._get_text("A5"), *"1");
    assert_eq!(model._get_text("B5"), *"2");
    assert_eq!(model._get_text("A6"), *"x");
    assert_eq!(model._get_text("B6"), *"y");
    assert_eq!(model._get_text("C5"), *"");
    assert_eq!(model._get_text("C6"), *"");
}
