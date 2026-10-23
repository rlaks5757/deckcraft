//! The app's side of the PPM host lock (the engine's is `deckcraft_engine::hostlock`): which of
//! the commands the app handles itself, dialogs and File tab pages a locked app shows. A command
//! or page nobody classified is refused, and the drift tests fail until it is classified.
//!
//! The commands that read a local picture, sound, video, theme or presentation to put *in* the
//! open presentation stay: they are edits. What a locked app loses is opening, creating and
//! closing files, Save As, export, printing and links, unless the parent's policy turns the
//! command on. Every command has a grade (see `deckcraft_engine::hostlock`); the parent's
//! overrides can turn `Off` commands on and `On` commands off.

use crate::backstage::Page;
use deckcraft_engine::hostlock::{Grade, Policy};

/// App commands a locked app runs as they are.
pub const UI_ALLOWED: &[&str] = &[
    "animation.preview",
    "app.about",
    "app.backstage",
    "app.browseThemeDialog",
    "app.changePictureDialog",
    "app.compareDialog",
    "app.dialog",
    "app.find",
    "app.insertAudioDialog",
    "app.insertPictureDialog",
    "app.insertVideoDialog",
    "app.palette",
    "app.pasteSpecial",
    "app.preferences",
    "app.qat",
    "app.replace",
    "app.reuseSlidesDialog",
    "app.smartArtPictureDialog",
    "review.thesaurusPane",
    "show.end",
    "show.presenter",
    "show.start",
    "view.collapseRibbon",
    "view.dark",
    "view.fit",
    "view.grayscale",
    "view.gridOptions",
    "view.gridlines",
    "view.guides",
    "view.newWindow",
    "view.normal",
    "view.notes",
    "view.notesPage",
    "view.outline",
    "view.pane",
    "view.reading",
    "view.ruler",
    "view.sorter",
    "view.tab",
    "view.zoom",
    "view.zoomDialog",
    "view.zoomIn",
    "view.zoomOut",
];

/// App commands a locked app never runs.
pub const UI_BLOCKED: &[&str] = &["app.closePresentation", "app.home", "app.openDialog"];

/// App commands that are off until the policy turns them on.
pub const UI_OFF: &[&str] = &["app.exportDialog", "app.links", "app.print", "app.saveAsDialog"];

/// Commands that are only another way in to a switch of their own, so the administrator has one
/// switch per feature: `(command, switch)`. The command follows the switch, page and all.
const ALIASES: &[(&str, &str)] = &[("app.exportDialog", "file.export"), ("app.links", "app.externalLinks"), ("app.saveAsDialog", "file.saveAs")];

/// The switch that decides `id`: itself, unless it is an alias.
fn switch(id: &str) -> &str {
    ALIASES.iter().find(|a| a.0 == id).map_or(id, |a| a.1)
}

/// Features that are no command but have a grade of their own: `(id, label, menu)`. Links the
/// app opens in a new tab (Help, About, a link in a slide) are `Off`.
pub const UI_FEATURES: &[(&str, &str, &[&str], Grade)] = &[("app.externalLinks", "Open links in a new tab", &["File", "Help"], Grade::Off)];

/// Dialogs that open once their command is on: `(dialog, command)`. Every other dialog opens.
pub const DIALOGS_GATED: &[(&str, &str)] = &[("export", "file.export")];

/// The grade of the command or feature `id` (an app command, an engine command, or a feature);
/// `None` for an id nobody classified.
pub fn grade(id: &str) -> Option<Grade> {
    if UI_BLOCKED.contains(&id) {
        Some(Grade::Locked)
    } else if UI_OFF.contains(&id) || UI_FEATURES.iter().any(|f| f.0 == id) {
        Some(Grade::Off)
    } else if UI_ALLOWED.contains(&id) {
        Some(Grade::On)
    } else {
        deckcraft_engine::hostlock::grade(id)
    }
}

/// Is `id` on for a session locked with `policy` (`None`: not locked)?
pub fn enabled(policy: Option<&Policy>, id: &str) -> bool {
    let id = switch(id);
    match policy {
        None => true,
        Some(p) => grade(id).is_some_and(|g| p.enabled(id, g)),
    }
}

/// May a session locked with `policy` run the app command `id`?
pub fn command_permits(policy: Option<&Policy>, id: &str) -> bool {
    enabled(policy, id)
}

/// May the dialog `name` open?
pub fn dialog_permits(policy: Option<&Policy>, name: &str) -> bool {
    DIALOGS_GATED.iter().all(|(d, c)| *d != name || enabled(policy, c))
}

/// May the File tab show `page`? Account stays; Home, New and Open are the start screen, and Info
/// (file properties, the file's location) is hidden whatever the policy says.
pub fn page_permits(policy: Option<&Policy>, page: Page) -> bool {
    match page {
        Page::Account => true,
        Page::Home | Page::New | Page::Open | Page::Info => policy.is_none(),
        Page::SaveAs => enabled(policy, "file.saveAs"),
        Page::Export => enabled(policy, "file.export"),
        Page::Print => enabled(policy, "app.print"),
    }
}

/// Where an app command is in the menus (tab, then group); `[]` for one nobody can reach there.
fn location(id: &str) -> Vec<&'static str> {
    match id {
        "app.about" => vec!["File", "Help"],
        "app.backstage" => vec!["File"],
        "app.browseThemeDialog" => vec!["Design", "Themes"],
        "app.changePictureDialog" => vec!["Picture Format", "Adjust"],
        "app.closePresentation" => vec!["File", "Close"],
        "app.compareDialog" => vec!["Review", "Compare"],
        "app.find" | "app.replace" => vec!["Home", "Editing"],
        "app.home" => vec!["File", "Home"],
        "app.insertAudioDialog" | "app.insertVideoDialog" => vec!["Insert", "Media"],
        "app.insertPictureDialog" => vec!["Insert", "Images"],
        "app.openDialog" => vec!["File", "Open"],
        "app.palette" => vec!["View"],
        "app.pasteSpecial" => vec!["Home", "Clipboard"],
        "app.preferences" => vec!["File", "Options"],
        "app.print" => vec!["File", "Print"],
        "app.reuseSlidesDialog" => vec!["Home", "Slides"],
        "app.smartArtPictureDialog" => vec!["SmartArt Design"],
        "animation.preview" => vec!["Animations", "Preview"],
        "review.thesaurusPane" => vec!["Review", "Proofing"],
        _ if id.starts_with("view.") => vec!["View"],
        _ if id.starts_with("show.") => vec!["Slide Show"],
        _ => vec![],
    }
}

/// The command list the administrator's screen shows (`ppm/commands.json`, copied to `dist/web`
/// by `ppm/build-web.sh`): `{ app, schema, commands }` with `{ id, label, labelKo, location,
/// grades }` for every command and feature, by id. `labelKo` comes from the Korean catalog and is
/// `null` only for a label it lacks (a test keeps that from happening).
pub fn commands_json() -> String {
    let entry = |id: &str, label: &str, location: &[&str], grade: &str| {
        let ko = crate::i18n::has(crate::i18n::Lang::KO, label).then(|| crate::i18n::tr(crate::i18n::Lang::KO, label));
        serde_json::json!({"id": id, "label": label, "labelKo": ko, "location": location, "grades": {"builder": grade}})
    };
    let mut rows: Vec<(String, serde_json::Value)> = Vec::new();
    for c in deckcraft_engine::cmd::command_specs() {
        let grade = deckcraft_engine::hostlock::grade(c.id).map_or("unclassified", Grade::name);
        rows.push((c.id.to_string(), entry(c.id, c.label, c.menu, grade)));
    }
    for (id, label, ..) in crate::UI_COMMANDS {
        if rows.iter().all(|r| r.0 != *id) && switch(id) == *id {
            let grade = grade(id).map_or("unclassified", Grade::name);
            rows.push((id.to_string(), entry(id, label, &location(id), grade)));
        }
    }
    for (id, label, menu, g) in UI_FEATURES {
        rows.push((id.to_string(), entry(id, label, menu, g.name())));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let list: Vec<serde_json::Value> = rows.into_iter().map(|r| r.1).collect();
    serde_json::to_string_pretty(&serde_json::json!({"app": "deck", "schema": 1, "commands": list})).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use deckcraft_engine::Session;

    #[test]
    fn every_app_command_is_classified_exactly_once() {
        for (id, ..) in crate::UI_COMMANDS {
            let n = [UI_ALLOWED.contains(id), UI_BLOCKED.contains(id), UI_OFF.contains(id)].iter().filter(|b| **b).count();
            assert_eq!(n, 1, "app command `{id}` must be in exactly one of UI_ALLOWED / UI_BLOCKED / UI_OFF (host lock)");
        }
        for id in UI_ALLOWED.iter().chain(UI_BLOCKED).chain(UI_OFF) {
            assert!(crate::UI_COMMANDS.iter().any(|c| c.0 == *id), "`{id}` is classified but is not an app command");
        }
        for list in [UI_ALLOWED, UI_BLOCKED, UI_OFF] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "keep the lists sorted and free of duplicates");
        }
    }

    #[test]
    fn one_switch_decides_a_feature_and_its_aliases() {
        let mut s = Session::new();
        s.lock_host();
        let p = s.host_policy();
        for (alias, target) in ALIASES {
            assert!(!enabled(p.as_ref(), alias) && !enabled(p.as_ref(), target), "{alias} starts off");
            s.set_policy(&[(target.to_string(), true)]);
            assert!(enabled(p.as_ref(), alias) && enabled(p.as_ref(), target), "{target} turns {alias} on");
            s.set_policy(&[(alias.to_string(), true)]);
            assert!(!enabled(p.as_ref(), alias), "{alias} has no switch of its own");
            s.set_policy(&[]);
        }
        assert!(dialog_permits(p.as_ref(), "about"));
        assert!(!dialog_permits(p.as_ref(), "export"));
        s.set_policy(&[("file.export".to_string(), true)]);
        assert!(dialog_permits(p.as_ref(), "export"));
    }

    #[test]
    fn aliases_name_real_switches_and_have_no_row() {
        let json = commands_json();
        for (alias, target) in ALIASES {
            assert!(grade(target).is_some(), "{alias} follows the unclassified `{target}`");
            assert!(crate::UI_COMMANDS.iter().any(|c| c.0 == *alias), "`{alias}` is no app command");
            assert!(!json.contains(&format!("\"id\": \"{alias}\"")), "{alias} must not be a switch of its own");
        }
        assert!(!json.contains("\"App\""), "an app command has a menu location");
    }

    #[test]
    fn gated_dialogs_name_known_commands() {
        for (dialog, id) in DIALOGS_GATED {
            assert!(grade(id).is_some(), "dialog `{dialog}` names the unclassified command `{id}`");
        }
    }

    #[test]
    fn pages_follow_the_policy() {
        let mut s = Session::new();
        s.lock_host();
        let p = s.host_policy();
        let shown = |p: Option<&Policy>| Page::ALL.into_iter().filter(|pg| page_permits(p, *pg)).map(Page::id).collect::<Vec<_>>();
        assert_eq!(shown(p.as_ref()), ["account"], "Info is hidden");
        s.set_policy(&[
            ("app.print".to_string(), true),
            ("file.export".to_string(), true),
            ("file.saveAs".to_string(), true),
            ("app.openDialog".to_string(), true),
        ]);
        assert_eq!(shown(p.as_ref()), ["saveAs", "print", "export", "account"], "an open page stays locked");
        assert_eq!(Page::ALL.len(), shown(None).len());
    }

    /// `ppm/commands.json` is the table: run with `UPDATE_COMMANDS_JSON=1` to write it again.
    #[test]
    fn commands_json_matches_the_tables() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppm/commands.json");
        let want = commands_json();
        assert!(!want.contains("unclassified"), "a command has no grade");
        assert!(!want.contains("\"labelKo\": null"), "a command label has no Korean translation in i18n/ko.tsv");
        if std::env::var_os("UPDATE_COMMANDS_JSON").is_some() {
            std::fs::write(&path, &want).unwrap();
        }
        let have = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(have, want, "ppm/commands.json is out of date: run the tests with UPDATE_COMMANDS_JSON=1");
    }
}
