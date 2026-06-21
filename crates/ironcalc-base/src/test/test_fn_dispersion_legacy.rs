#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

fn model_with_data() -> crate::model::Model<'static> {
    let mut model = new_empty_model();
    model._set("A1", "4");
    model._set("A2", "5");
    model._set("A3", "8");
    model._set("A4", "11");
    model._set("A5", "13");
    model._set("A6", "2");
    model
}

#[test]
fn fn_stdev_matches_stdev_s() {
    let mut model = model_with_data();

    model._set("B1", "=STDEV(A1:A6)");
    model._set("B2", "=STDEV.S(A1:A6)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), model._get_text("B2"));
}

#[test]
fn fn_stdevp_matches_stdev_p() {
    let mut model = model_with_data();

    model._set("B1", "=STDEVP(A1:A6)");
    model._set("B2", "=STDEV.P(A1:A6)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), model._get_text("B2"));
}

#[test]
fn fn_var_matches_var_s() {
    let mut model = model_with_data();

    model._set("B1", "=VAR(A1:A6)");
    model._set("B2", "=VAR.S(A1:A6)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), model._get_text("B2"));
}

#[test]
fn fn_varp_matches_var_p() {
    let mut model = model_with_data();

    model._set("B1", "=VARP(A1:A6)");
    model._set("B2", "=VAR.P(A1:A6)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), model._get_text("B2"));
}

#[test]
fn fn_rank_matches_rank_eq() {
    let mut model = model_with_data();

    model._set("B1", "=RANK(8, A1:A6)");
    model._set("B2", "=RANK.EQ(8, A1:A6)");
    model._set("C1", "=RANK(8, A1:A6, 1)");
    model._set("C2", "=RANK.EQ(8, A1:A6, 1)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), model._get_text("B2"));
    assert_eq!(model._get_text("C1"), model._get_text("C2"));
}

#[test]
fn fn_dispersion_legacy_arg_errors() {
    let mut model = model_with_data();

    model._set("B1", "=STDEV()");
    model._set("B2", "=STDEVP()");
    model._set("B3", "=VAR()");
    model._set("B4", "=VARP()");
    model._set("B5", "=RANK(8)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), "#ERROR!");
    assert_eq!(model._get_text("B2"), "#ERROR!");
    assert_eq!(model._get_text("B3"), "#ERROR!");
    assert_eq!(model._get_text("B4"), "#ERROR!");
    assert_eq!(model._get_text("B5"), "#ERROR!");
}
