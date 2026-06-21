#![allow(clippy::unwrap_used)]

use crate::test::util::new_empty_model;

#[test]
fn fn_binomdist_matches_binom_dist() {
    let mut model = new_empty_model();

    model._set("A1", "=BINOMDIST(6, 10, 0.5, FALSE)");
    model._set("A2", "=BINOM.DIST(6, 10, 0.5, FALSE)");
    model._set("A3", "=BINOMDIST(6, 10, 0.5, TRUE)");
    model._set("A4", "=BINOM.DIST(6, 10, 0.5, TRUE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
    assert_eq!(model._get_text("A3"), model._get_text("A4"));
}

#[test]
fn fn_critbinom_matches_binom_inv() {
    let mut model = new_empty_model();

    model._set("A1", "=CRITBINOM(10, 0.5, 0.75)");
    model._set("A2", "=BINOM.INV(10, 0.5, 0.75)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_confidence_matches_confidence_norm() {
    let mut model = new_empty_model();

    model._set("A1", "=CONFIDENCE(0.05, 2.5, 50)");
    model._set("A2", "=CONFIDENCE.NORM(0.05, 2.5, 50)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_covar_matches_covariance_p() {
    let mut model = new_empty_model();

    model._set("A1", "3");
    model._set("A2", "2");
    model._set("A3", "4");
    model._set("B1", "9");
    model._set("B2", "7");
    model._set("B3", "12");
    model._set("C1", "=COVAR(A1:A3, B1:B3)");
    model._set("C2", "=COVARIANCE.P(A1:A3, B1:B3)");

    model.evaluate();

    assert_eq!(model._get_text("C1"), model._get_text("C2"));
}

#[test]
fn fn_expondist_matches_expon_dist() {
    let mut model = new_empty_model();

    model._set("A1", "=EXPONDIST(0.2, 10, TRUE)");
    model._set("A2", "=EXPON.DIST(0.2, 10, TRUE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_gammadist_matches_gamma_dist() {
    let mut model = new_empty_model();

    model._set("A1", "=GAMMADIST(10, 9, 2, FALSE)");
    model._set("A2", "=GAMMA.DIST(10, 9, 2, FALSE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_gammainv_matches_gamma_inv() {
    let mut model = new_empty_model();

    model._set("A1", "=GAMMAINV(0.5, 9, 2)");
    model._set("A2", "=GAMMA.INV(0.5, 9, 2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_loginv_matches_lognorm_inv() {
    let mut model = new_empty_model();

    model._set("A1", "=LOGINV(0.5, 3.5, 1.2)");
    model._set("A2", "=LOGNORM.INV(0.5, 3.5, 1.2)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_poisson_matches_poisson_dist() {
    let mut model = new_empty_model();

    model._set("A1", "=POISSON(2, 5, FALSE)");
    model._set("A2", "=POISSON.DIST(2, 5, FALSE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_weibull_matches_weibull_dist() {
    let mut model = new_empty_model();

    model._set("A1", "=WEIBULL(105, 20, 100, TRUE)");
    model._set("A2", "=WEIBULL.DIST(105, 20, 100, TRUE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_ztest_matches_z_test() {
    let mut model = new_empty_model();

    model._set("A1", "3");
    model._set("A2", "6");
    model._set("A3", "7");
    model._set("A4", "8");
    model._set("A5", "6");
    model._set("B1", "=ZTEST(A1:A5, 4)");
    model._set("B2", "=Z.TEST(A1:A5, 4)");

    model.evaluate();

    assert_eq!(model._get_text("B1"), model._get_text("B2"));
}

#[test]
fn fn_betainv_matches_beta_inv() {
    let mut model = new_empty_model();

    model._set("A1", "=BETAINV(0.5, 8, 10)");
    model._set("A2", "=BETA.INV(0.5, 8, 10)");
    model._set("A3", "=BETAINV(0.5, 8, 10, 1, 3)");
    model._set("A4", "=BETA.INV(0.5, 8, 10, 1, 3)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
    assert_eq!(model._get_text("A3"), model._get_text("A4"));
}

#[test]
fn fn_betadist_injects_cumulative_true() {
    let mut model = new_empty_model();

    model._set("A1", "=BETADIST(2, 8, 10, 1, 3)");
    model._set("A2", "=BETA.DIST(2, 8, 10, TRUE, 1, 3)");
    model._set("A3", "=BETADIST(0.4, 8, 10)");
    model._set("A4", "=BETA.DIST(0.4, 8, 10, TRUE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
    assert_eq!(model._get_text("A3"), model._get_text("A4"));
}

#[test]
fn fn_lognormdist_injects_cumulative_true() {
    let mut model = new_empty_model();

    model._set("A1", "=LOGNORMDIST(4, 3.5, 1.2)");
    model._set("A2", "=LOGNORM.DIST(4, 3.5, 1.2, TRUE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_hypgeomdist_injects_cumulative_false() {
    let mut model = new_empty_model();

    model._set("A1", "=HYPGEOMDIST(1, 4, 8, 20)");
    model._set("A2", "=HYPGEOM.DIST(1, 4, 8, 20, FALSE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_negbinomdist_injects_cumulative_false() {
    let mut model = new_empty_model();

    model._set("A1", "=NEGBINOMDIST(10, 5, 0.25)");
    model._set("A2", "=NEGBINOM.DIST(10, 5, 0.25, FALSE)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), model._get_text("A2"));
}

#[test]
fn fn_stat_legacy_args_number() {
    let mut model = new_empty_model();

    model._set("A1", "=BINOMDIST(1)");
    model._set("A2", "=LOGNORMDIST(1, 2)");
    model._set("A3", "=NEGBINOMDIST(1, 2)");
    model._set("A4", "=HYPGEOMDIST(1, 2, 3)");

    model.evaluate();

    assert_eq!(model._get_text("A1"), *"#ERROR!");
    assert_eq!(model._get_text("A2"), *"#ERROR!");
    assert_eq!(model._get_text("A3"), *"#ERROR!");
    assert_eq!(model._get_text("A4"), *"#ERROR!");
}
