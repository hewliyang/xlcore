#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_chidist_matches_chisq_dist_rt() {
    let mut model = new_empty_model();

    model._set("A1", "=CHIDIST(18.307, 10)");
    model._set("A2", "=CHISQ.DIST.RT(18.307, 10)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_chiinv_matches_chisq_inv_rt() {
    let mut model = new_empty_model();

    model._set("A1", "=CHIINV(0.05, 10)");
    model._set("A2", "=CHISQ.INV.RT(0.05, 10)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_chitest_matches_chisq_test() {
    let mut model = new_empty_model();

    model._set("A1", "58");
    model._set("A2", "35");
    model._set("B1", "45");
    model._set("B2", "48");
    model._set("C1", "=CHITEST(A1:A2, B1:B2)");
    model._set("C2", "=CHISQ.TEST(A1:A2, B1:B2)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), model._get_text("C2"));
}

#[test]
fn fn_fdist_matches_f_dist_rt() {
    let mut model = new_empty_model();

    model._set("A1", "=FDIST(15.2068649, 6, 4)");
    model._set("A2", "=F.DIST.RT(15.2068649, 6, 4)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_finv_matches_f_inv_rt() {
    let mut model = new_empty_model();

    model._set("A1", "=FINV(0.01, 6, 4)");
    model._set("A2", "=F.INV.RT(0.01, 6, 4)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_ftest_matches_f_test() {
    let mut model = new_empty_model();

    model._set("A1", "6");
    model._set("A2", "7");
    model._set("A3", "9");
    model._set("B1", "20");
    model._set("B2", "28");
    model._set("B3", "31");
    model._set("C1", "=FTEST(A1:A3, B1:B3)");
    model._set("C2", "=F.TEST(A1:A3, B1:B3)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), model._get_text("C2"));
}

#[test]
fn fn_tinv_matches_t_inv_2t() {
    let mut model = new_empty_model();

    model._set("A1", "=TINV(0.05, 10)");
    model._set("A2", "=T.INV.2T(0.05, 10)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_ttest_matches_t_test() {
    let mut model = new_empty_model();

    model._set("A1", "3");
    model._set("A2", "4");
    model._set("A3", "5");
    model._set("B1", "6");
    model._set("B2", "19");
    model._set("B3", "5");
    model._set("C1", "=TTEST(A1:A3, B1:B3, 2, 1)");
    model._set("C2", "=T.TEST(A1:A3, B1:B3, 2, 1)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), model._get_text("C2"));
}

#[test]
fn fn_chi_f_t_legacy_args_number() {
    let mut model = new_empty_model();

    model._set("A1", "=CHIDIST(1)");
    model._set("A2", "=TINV(0.05)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
}
