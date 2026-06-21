#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn setup() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("G1", "1");
    model._set("H1", "2");
    model._set("G2", "3");
    model._set("H2", "4");
    model._set("G3", "5");
    model._set("H3", "6");
    model
}

#[test]
fn fn_chooserows_args_number() {
    let mut model = new_empty_model();
    model._set("A1", "=CHOOSEROWS(G1:H3)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#ERROR!");
}

#[test]
fn fn_chooserows_single() {
    let mut model = setup();
    model._set("A1", "=CHOOSEROWS(G1:H3, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("B1"), *"4");
    assert_eq!(model._get_text("A2"), *"");
}

#[test]
fn fn_chooserows_multiple_reordered() {
    let mut model = setup();
    model._set("A1", "=CHOOSEROWS(G1:H3, 3, 1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"5");
    assert_eq!(model._get_text("B1"), *"6");
    assert_eq!(model._get_text("A2"), *"1");
    assert_eq!(model._get_text("B2"), *"2");
}

#[test]
fn fn_chooserows_repeated() {
    let mut model = setup();
    model._set("A1", "=CHOOSEROWS(G1:H3, 2, 2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"3");
    assert_eq!(model._get_text("A2"), *"3");
}

#[test]
fn fn_chooserows_negative() {
    let mut model = setup();
    model._set("A1", "=CHOOSEROWS(G1:H3, -1)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"5");
    assert_eq!(model._get_text("B1"), *"6");
}

#[test]
fn fn_chooserows_out_of_range() {
    let mut model = setup();
    model._set("A1", "=CHOOSEROWS(G1:H3, 4)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_chooserows_zero() {
    let mut model = setup();
    model._set("A1", "=CHOOSEROWS(G1:H3, 0)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"#VALUE!");
}

#[test]
fn fn_chooserows_array_indices() {
    let mut model = setup();
    model._set("J1", "3");
    model._set("J2", "1");
    model._set("A1", "=CHOOSEROWS(G1:H3, J1:J2)");
    model.evaluate();
    assert_eq!(model._get_text("A1"), *"5");
    assert_eq!(model._get_text("A2"), *"1");
}
