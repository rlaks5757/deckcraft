//! The Korean catalog and the PPM build's Korean start. Tests run with the `ppm-host` feature off,
//! so English is their default; Korean is switched on per thread.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use egui_kittest::kittest::{NodeT, Queryable};

use super::catalog::Catalog;
use super::*;
use crate::SlideApp;

/// Lowest edge a ribbon button may reach (the ribbon is 100 px tall under the tab strip).
/// Where the ribbon clips: the window edge less the card margin (8) and its inner padding (10).
const CLIP_RIGHT: f32 = 1280.0 - 18.0;
const RIBBON_BOTTOM: f32 = 175.0;
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

fn app_in(lang: Lang) -> egui_kittest::Harness<'static, SlideApp> {
    set_current(lang);
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(Session::with_new(), Default::default()),
    );
    h.run_steps(1);
    // The test build embeds no craft fonts, so Hangul would be measured with a stand-in. Put the
    // PPM build's NanumGothic first, as `theme::font_definitions` does with the embedded one.
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppm/fonts-ko/fonts/nanum-gothic/NanumGothic-Regular.ttf");
    let bytes = std::fs::read(path).expect("NanumGothic in ppm/fonts-ko");
    let mut defs = crate::theme::font_definitions(&[]);
    defs.font_data.insert("nanum".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Name("bold".into())] {
        defs.families.entry(family).or_default().insert(0, "nanum".into());
    }
    h.ctx.set_fonts(defs);
    h.run_steps(3);
    h
}

#[test]
fn the_ribbon_tabs_read_in_korean() {
    let h = app_in(Lang::KO);
    for tab in ["홈", "삽입", "디자인", "전환", "애니메이션", "슬라이드 쇼", "검토", "보기"] {
        assert!(h.query_all_by_label(tab).next().is_some(), "no ribbon tab {tab:?}");
    }
    assert!(h.query_by_label("Insert").is_none(), "the English tab name is still shown");
    set_current(Lang::EN);
}

#[test]
fn the_status_bar_reads_in_korean() {
    let h = app_in(Lang::KO);
    for name in ["메모", "노트"] {
        assert!(h.query_all_by_label(name).next().is_some(), "the status bar button {name:?} is not Korean");
    }
    set_current(Lang::EN);
}

const TABS: [(&str, &str); 8] = [("Home", "홈"), ("Insert", "삽입"), ("Design", "디자인"), ("Transitions", "전환"), ("Animations", "애니메이션"), ("Slide Show", "슬라이드 쇼"), ("Review", "검토"), ("View", "보기")];

/// The rects of the buttons in the open ribbon tab (below the tab strip, above the canvas).
fn ribbon_buttons(h: &mut egui_kittest::Harness<'static, SlideApp>, tab: &str, first: bool) -> Vec<egui::Rect> {
    // Home is open to begin with, and a click on the open tab folds the ribbon away.
    if !first {
        h.query_all_by_label(tab).find(|n| n.rect().top() > 40.0).expect("ribbon tab").click();
        h.run_steps(3);
    }
    h.query_all_by_role(egui::accesskit::Role::Button).map(|n| n.rect()).filter(|r| r.top() > 60.0 && r.top() < RIBBON_BOTTOM).collect()
}

/// With Hangul measured in NanumGothic, no ribbon button of any tab overlaps another or leaves the
/// ribbon, and the buttons end no further right than the English ones do (the Home and Insert
/// tabs are wider than the window in English too; the ribbon clips what does not fit).
#[test]
fn korean_ribbon_buttons_keep_clear_of_each_other() {
    let mut en = app_in(Lang::EN);
    let english: Vec<f32> = TABS.iter().enumerate().map(|(i, (name, _))| ribbon_buttons(&mut en, name, i == 0).iter().map(|r| r.right()).fold(0.0, f32::max)).collect();
    let mut ko = app_in(Lang::KO);
    for (i, (_, tab)) in TABS.iter().enumerate() {
        let rects = ribbon_buttons(&mut ko, tab, i == 0);
        assert!(rects.len() >= 4, "{tab}: found only {} ribbon buttons", rects.len());
        let right = rects.iter().map(|r| r.right()).fold(0.0, f32::max);
        assert!(right <= english[i].max(CLIP_RIGHT) + 2.0, "{tab}: the buttons reach x={right}, English reaches {}", english[i]);
        for (j, a) in rects.iter().enumerate() {
            assert!(a.bottom() <= RIBBON_BOTTOM, "a button runs out of the ribbon on {tab}: {a:?}");
            for b in &rects[j + 1..] {
                // A split button's two halves sit inside the whole button.
                if a.contains_rect(*b) || b.contains_rect(*a) {
                    continue;
                }
                let overlap = a.intersect(*b);
                assert!(overlap.width() < 2.0 || overlap.height() < 2.0, "ribbon buttons overlap on {tab}: {a:?} {b:?}");
            }
        }
    }
    set_current(Lang::EN);
}

/// Ribbon buttons are listed by their spoken name (`\n` read as a space), which is how the quick
/// access toolbar looks them up, so every two-line label needs the one-line form too.
#[test]
fn two_line_labels_have_a_one_line_form() {
    for line in include_str!("ko.tsv").lines().filter(|l| !l.starts_with('#')) {
        let key = line.split('\t').nth(1).unwrap_or("");
        if key.contains("\\n") {
            let flat = key.replace("\\n", " ");
            assert!(has(Lang::KO, &flat), "no one-line form of {key:?}");
        }
    }
}
