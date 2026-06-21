#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn setup() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("G1", "a");
    model._set("H1", "3");
    model._set("G2", "b");
    model._set("H2", "1");
    model._set("G3", "c");
    model._set("H3", "2");
    model._set("J1", "3");
    model._set("J2", "1");
    model._set("J3", "2");
    model
}

#[test]
fn fn_sortby_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=SORTBY(G1:H3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_sortby_asc() {
    let mut model = setup();
    model._set("A1", "=SORTBY(G1:H3, J1:J3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"b");
    assert_eq!(model._get_text("B1"), *"1");
    assert_eq!(model._get_text("A2"), *"c");
    assert_eq!(model._get_text("B2"), *"2");
    assert_eq!(model._get_text("A3"), *"a");
    assert_eq!(model._get_text("B3"), *"3");
}

#[test]
fn fn_sortby_desc() {
    let mut model = setup();
    model._set("A1", "=SORTBY(G1:H3, J1:J3, -1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"a");
    assert_eq!(model._get_text("B1"), *"3");
    assert_eq!(model._get_text("A2"), *"c");
    assert_eq!(model._get_text("B2"), *"2");
    assert_eq!(model._get_text("A3"), *"b");
    assert_eq!(model._get_text("B3"), *"1");
}

#[test]
fn fn_sortby_two_keys() {
    let mut model = new_empty_model();
    model._set("G1", "x");
    model._set("G2", "y");
    model._set("G3", "z");
    model._set("H1", "1");
    model._set("H2", "1");
    model._set("H3", "2");
    model._set("J1", "2");
    model._set("J2", "1");
    model._set("J3", "5");
    model._set("A1", "=SORTBY(G1:G3, H1:H3, 1, J1:J3, 1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"y");
    assert_eq!(model._get_text("A2"), *"x");
    assert_eq!(model._get_text("A3"), *"z");
}

#[test]
fn fn_sortby_length_mismatch() {
    let mut model = setup();
    model._set("K1", "1");
    model._set("K2", "2");
    model._set("A1", "=SORTBY(G1:H3, K1:K2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_sortby_bad_order() {
    let mut model = setup();
    model._set("A1", "=SORTBY(G1:H3, J1:J3, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}
