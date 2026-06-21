#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn model_with_data() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("A1", "10");
    model._set("A2", "20");
    model._set("A3", "30");
    model._set("A4", "40");
    model._set("A5", "20");
    model
}

#[test]
fn fn_xmatch_args_number() {
    let mut model = new_empty_model();
    model._set("B1", "=XMATCH(1)");
    model._set("B2", "=XMATCH(1, A1:A5, 0, 1, 1)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"#ERROR!");
    assert_eq!(model._get_text("B2"), *"#ERROR!");
}

#[test]
fn fn_xmatch_exact() {
    let mut model = model_with_data();
    model._set("B1", "=XMATCH(30, A1:A5)");
    model._set("B2", "=XMATCH(30, A1:A5, 0)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("B2"), *"3");
}

#[test]
fn fn_xmatch_next_smaller() {
    let mut model = model_with_data();
    model._set("B1", "=XMATCH(35, A1:A5, -1)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"3");
}

#[test]
fn fn_xmatch_next_larger() {
    let mut model = model_with_data();
    model._set("B1", "=XMATCH(25, A1:A5, 1)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"3");
}

#[test]
fn fn_xmatch_wildcard() {
    let mut model = new_empty_model();
    model._set("A1", "apple");
    model._set("A2", "banana");
    model._set("A3", "cherry");
    model._set("B1", r#"=XMATCH("ban*", A1:A3, 2)"#);
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"2");
}

#[test]
fn fn_xmatch_reverse() {
    let mut model = model_with_data();
    model._set("B1", "=XMATCH(20, A1:A5, 0, -1)");
    model._set("B2", "=XMATCH(20, A1:A5, 0, 1)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"5");
    assert_eq!(model._get_text("B2"), *"2");
}

#[test]
fn fn_xmatch_not_found() {
    let mut model = model_with_data();
    model._set("B1", "=XMATCH(99, A1:A5)");
    model._set("B2", "=XMATCH(5, A1:A5, -1)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"#N/A");
    assert_eq!(model._get_text("B2"), *"#N/A");
}

#[test]
fn fn_xmatch_binary_ascending() {
    let mut model = new_empty_model();
    model._set("A1", "10");
    model._set("A2", "20");
    model._set("A3", "30");
    model._set("A4", "40");
    model._set("B1", "=XMATCH(30, A1:A4, 0, 2)");
    model._set("B2", "=XMATCH(35, A1:A4, -1, 2)");
    model.evaluate();
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("B2"), *"3");
}
