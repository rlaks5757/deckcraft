//! The Korean catalog and the PPM build's Korean start. Tests run with the `ppm-host` feature off,
//! so English is their default; Korean is switched on per thread.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui_kittest::kittest::Queryable;

use super::catalog::Catalog;
use super::*;
use crate::SlideApp;
use deckcraft_engine::Session;

#[test]
fn the_ppm_build_starts_in_korean_and_others_in_english() {
    assert_eq!(default_lang(true), Lang::KO);
    assert_eq!(default_lang(false), Lang::EN);
    // This test build has no `ppm-host` feature, so a fresh thread is English.
    assert_eq!(current(), default_lang(deckcraft_engine::hostlock::HOST_BUILD));
}

#[test]
fn the_korean_catalog_has_no_errors_and_no_duplicates() {
    let (catalog, errors) = Catalog::parse(include_str!("ko.tsv"));
    assert!(errors.is_empty(), "ko.tsv: {errors:?}");
    drop(catalog);
}

#[test]
fn untranslated_text_stays_english() {
    set_current(Lang::KO);
    assert_eq!(t("Home"), "홈");
    assert_eq!(t("No Such Label Anywhere"), "No Such Label Anywhere");
    assert_eq!(location("Home › Font"), format!("홈 › {}", tr(Lang::KO, "Font")));
    assert_eq!(fmt(t("Slide {n} of {total}"), &[("n", "2"), ("total", "5")]), "슬라이드 2/5");
    set_current(Lang::EN);
    assert_eq!(t("Home"), "Home");
}

/// Every label the administrator's screen lists has a Korean name.
#[test]
fn every_command_label_has_a_korean_name() {
    let json: serde_json::Value = serde_json::from_str(&crate::host_lock::commands_json()).unwrap();
    let rows = json["commands"].as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        let label = row["label"].as_str().unwrap();
        assert!(has(Lang::KO, label), "no Korean for command label {label:?} ({})", row["id"]);
        assert!(row["labelKo"].is_string(), "labelKo of {} is null", row["id"]);
    }
}

#[test]
fn ppm_commands_json_is_filled_in_korean() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppm/commands.json")).unwrap();
    assert!(!text.contains("\"labelKo\": null"));
}

fn korean_app() -> egui_kittest::Harness<'static, SlideApp> {
    set_current(Lang::KO);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(Session::with_new(), Default::default()),
    );
    h.run_steps(3);
    h
}

#[test]
fn the_ribbon_tabs_read_in_korean() {
    let h = korean_app();
    for tab in ["홈", "삽입", "디자인", "전환", "애니메이션", "슬라이드 쇼", "검토", "보기"] {
        assert!(h.query_all_by_label(tab).next().is_some(), "no ribbon tab {tab:?}");
    }
    assert!(h.query_by_label("Insert").is_none(), "the English tab name is still shown");
    set_current(Lang::EN);
}

#[test]
fn korean_ribbon_buttons_keep_clear_of_each_other() {
    let mut h = korean_app();
    for tab in ["홈", "삽입", "전환", "애니메이션"] {
        // The tab strip sits below the title bar, which has a Home button of its own.
        h.query_all_by_label(tab).find(|n| n.rect().top() > 40.0).expect("ribbon tab").click();
        h.run_steps(3);
        let mut rects: Vec<egui::Rect> = h.query_all_by_role(egui::accesskit::Role::Button).map(|n| n.rect()).filter(|r| r.top() > 60.0 && r.bottom() < 160.0).collect();
        rects.sort_by(|a, b| a.left().total_cmp(&b.left()));
        for pair in rects.windows(2) {
            // A split button's two halves sit inside the whole button.
            if pair[0].contains_rect(pair[1]) || pair[1].contains_rect(pair[0]) {
                continue;
            }
            let overlap = pair[0].intersect(pair[1]);
            assert!(overlap.width() < 2.0 || overlap.height() < 2.0, "ribbon buttons overlap on {tab}: {:?} {:?}", pair[0], pair[1]);
        }
        assert!(rects.iter().all(|r| r.right() <= 1280.0 || tab == "홈"), "a button runs off the window on {tab}");
    }
    set_current(Lang::EN);
}
