#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_sort_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=SORT()");
    model._set("A2", "=SORT(C1:C3, 1, 1, FALSE, 9)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}

#[test]
fn fn_sort_numeric_ascending() {
    let mut model = new_empty_model();
    model._set("C1", "3");
    model._set("C2", "1");
    model._set("C3", "2");
    model._set("A1", "=SORT(C1:C3)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"1");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("A3"), *"3");
}

#[test]
fn fn_sort_numeric_descending() {
    let mut model = new_empty_model();
    model._set("C1", "3");
    model._set("C2", "1");
    model._set("C3", "2");
    model._set("A1", "=SORT(C1:C3, 1, -1)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("A2"), *"2");
    assert_eq!(model._get_text("A3"), *"1");
}

#[test]
fn fn_sort_by_second_column() {
    let mut model = new_empty_model();
    model._set("C1", "a");
    model._set("D1", "3");
    model._set("C2", "b");
    model._set("D2", "1");
    model._set("C3", "c");
    model._set("D3", "2");
    model._set("A1", "=SORT(C1:D3, 2)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"b");
    assert_eq!(model._get_text("B1"), *"1");
    assert_eq!(model._get_text("A2"), *"c");
    assert_eq!(model._get_text("B2"), *"2");
    assert_eq!(model._get_text("A3"), *"a");
    assert_eq!(model._get_text("B3"), *"3");
}

#[test]
fn fn_sort_text_case_insensitive() {
    let mut model = new_empty_model();
    model._set("C1", "banana");
    model._set("C2", "Apple");
    model._set("C3", "cherry");
    model._set("A1", "=SORT(C1:C3)");
    model.evaluate();

    assert_eq!(model._get_text("A1"), *"Apple");
    assert_eq!(model._get_text("A2"), *"banana");
    assert_eq!(model._get_text("A3"), *"cherry");
}

#[test]
fn fn_sort_bad_sort_order() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("C2", "2");
    model._set("A1", "=SORT(C1:C2, 1, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_sort_bad_sort_index() {
    let mut model = new_empty_model();
    model._set("C1", "1");
    model._set("C2", "2");
    model._set("A1", "=SORT(C1:C2, 5)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_sort_by_col() {
    let mut model = new_empty_model();
    model._set("C1", "3");
    model._set("D1", "1");
    model._set("E1", "2");
    model._set("C2", "x");
    model._set("D2", "y");
    model._set("E2", "z");
    model._set("A5", "=SORT(C1:E2, 1, 1, TRUE)");
    model.evaluate();

    assert_eq!(model._get_text("A5"), *"1");
    assert_eq!(model._get_text("B5"), *"2");
    assert_eq!(model._get_text("C5"), *"3");
    assert_eq!(model._get_text("A6"), *"y");
    assert_eq!(model._get_text("B6"), *"z");
    assert_eq!(model._get_text("C6"), *"x");
}
