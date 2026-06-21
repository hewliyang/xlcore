#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_tdist_one_tail_matches_t_dist_rt() {
    let mut model = new_empty_model();

    model._set("A1", "=TDIST(1.5, 10, 1)");
    model._set("A2", "=T.DIST.RT(1.5, 10)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_tdist_two_tail_matches_t_dist_2t() {
    let mut model = new_empty_model();

    model._set("A1", "=TDIST(1.5, 10, 2)");
    model._set("A2", "=T.DIST.2T(1.5, 10)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_tdist_arg_errors() {
    let mut model = new_empty_model();

    model._set("A1", "=TDIST(-1, 10, 1)");
    model._set("A2", "=TDIST(1.5, 10, 3)");
    model._set("A3", "=TDIST(1.5, 10)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), "#NUM!");
    assert_eq!(model._get_text("A2"), "#NUM!");
    assert_eq!(model._get_text("A3"), "#ERROR!");
}

fn model_with_repeats() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("A1", "1");
    model._set("A2", "2");
    model._set("A3", "2");
    model._set("A4", "3");
    model._set("A5", "4");
    model
}

#[test]
fn fn_mode_returns_most_frequent() {
    let mut model = model_with_repeats();

    model._set("B1", "=MODE(A1:A5)");
    model._set("B2", "=MODE.SNGL(A1:A5)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "2");
    assert_eq!(model._get_text("B2"), "2");
}

#[test]
fn fn_mode_no_repeat_is_na() {
    let mut model = new_empty_model();
    model._set("A1", "5");
    model._set("A2", "6");
    model._set("A3", "7");
    model._set("A4", "2");
    model._set("A5", "3");
    model._set("A6", "4");

    model._set("B1", "=MODE(A1:A6)");
    model._set("B2", "=MODE.SNGL(A1:A6)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "#N/A");
    assert_eq!(model._get_text("B2"), "#N/A");
}
