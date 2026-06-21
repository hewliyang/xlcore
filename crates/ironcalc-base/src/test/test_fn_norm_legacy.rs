#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_normdist_matches_norm_dist() {
    let mut model = new_empty_model();

    model._set("A1", "=NORMDIST(42, 40, 1.5, TRUE)");
    model._set("A2", "=NORM.DIST(42, 40, 1.5, TRUE)");
    model._set("A3", "=NORMDIST(42, 40, 1.5, FALSE)");
    model._set("A4", "=NORM.DIST(42, 40, 1.5, FALSE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
    assert_eq!(model._get_text("A3"), model._get_text("A4"));
}

#[test]
fn fn_norminv_matches_norm_inv() {
    let mut model = new_empty_model();

    model._set("A1", "=NORMINV(0.908789, 40, 1.5)");
    model._set("A2", "=NORM.INV(0.908789, 40, 1.5)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_normsdist_one_arg_cumulative() {
    let mut model = new_empty_model();

    model._set("A1", "=NORMSDIST(1.333333)");
    model._set("A2", "=NORM.S.DIST(1.333333, TRUE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_normsinv_matches_norm_s_inv() {
    let mut model = new_empty_model();

    model._set("A1", "=NORMSINV(0.908789)");
    model._set("A2", "=NORM.S.INV(0.908789)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_norm_legacy_args_number() {
    let mut model = new_empty_model();

    model._set("A1", "=NORMSDIST(1, 2)");
    model._set("A2", "=NORMSINV()");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}
