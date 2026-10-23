//! Every command has a grade. `Locked`: never runs in a locked session, whatever the policy
//! says (opening, creating, closing and reverting files, templates, recovery; the parent's own
//! file arrives through [`crate::Session::open_host_file`]). `Off`: off until the parent's policy
//! turns it on (Save As, export, printing, links). `On`: on until the policy turns it off.
//! Edits inside the presentation are `On`, including the ones that read a local picture, sound,
//! video, theme or presentation to put in it. Save hands the presentation to the parent.
//!
//! The parent sends `policy: { overrides: { "<command id>": true | false } }` with the
//! presentation ([`Policy`]); only commands that differ from their grade are listed. A command
//! nobody classified is refused, and the drift test below fails until it is classified.

use serde_json::Value as Json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// True in builds made with the `ppm-host` feature (`ppm/build-web.sh`): every session is locked
/// and nothing at run time can unlock it.
pub const HOST_BUILD: bool = cfg!(feature = "ppm-host");

/// Text every host-lock refusal carries, so the UI can drop it without a message.
pub const BLOCKED_TEXT: &str = "blocked by the PPM host lock";

/// Whether a command runs by default and what a policy can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Locked,
    Off,
    On,
}

impl Grade {
    /// The name `commands.json` uses.
    pub fn name(self) -> &'static str {
        match self {
            Grade::Locked => "locked",
            Grade::Off => "off",
            Grade::On => "on",
        }
    }
}

/// The parent's overrides. Cloned handles share the state, so a service made once at start sees
/// a policy that arrives with the next open message.
#[derive(Clone, Debug, Default)]
pub struct Policy(Arc<Mutex<HashMap<String, bool>>>);

impl Policy {
    /// Is a command of `grade` on? `true` turns an `Off` command on, `false` turns an `On`
    /// command off; a `Locked` command never is, and an id nobody overrode keeps its grade.
    pub fn enabled(&self, id: &str, grade: Grade) -> bool {
        let over = self.0.lock().unwrap_or_else(|e| e.into_inner()).get(id).copied();
        match grade {
            Grade::Locked => false,
            Grade::Off => over == Some(true),
            Grade::On => over != Some(false),
        }
    }

    /// Replaces the overrides. Ids that are no command are never asked for, so they do nothing.
    pub fn set(&self, overrides: &[(String, bool)]) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = overrides.iter().cloned().collect();
    }
}

/// Commands a locked session runs as they are.
pub const ALLOWED: &[&str] = &[
    "animation.add",
    "animation.clear",
    "animation.get",
    "animation.list",
    "animation.move",
    "animation.options",
    "animation.painter",
    "animation.painterApply",
    "animation.remove",
    "animation.set",
    "animation.timing",
    "arrange.align",
    "arrange.bringForward",
    "arrange.bringToFront",
    "arrange.distribute",
    "arrange.flipHorizontal",
    "arrange.flipVertical",
    "arrange.group",
    "arrange.nudge",
    "arrange.regroup",
    "arrange.reorder",
    "arrange.rotateLeft",
    "arrange.rotateRight",
    "arrange.sendBackward",
    "arrange.sendToBack",
    "arrange.ungroup",
    "chart.data",
    "chart.options",
    "chart.type",
    "commands.list",
    "comment.add",
    "comment.delete",
    "comment.list",
    "comment.next",
    "comment.prev",
    "comment.reply",
    "comment.resolve",
    "design.applyIdea",
    "design.background",
    "design.browseTheme",
    "design.colors",
    "design.effects",
    "design.fonts",
    "design.headerFooter",
    "design.hideBackgroundGraphics",
    "design.ideas",
    "design.slideSize",
    "design.theme",
    "design.themes",
    "design.variant",
    "design.variants",
    "document.inspect",
    "draw.inkToShape",
    "edit.copy",
    "edit.cut",
    "edit.delete",
    "edit.deselect",
    "edit.duplicate",
    "edit.paste",
    "edit.pasteSpecial",
    "edit.pasteText",
    "edit.redo",
    "edit.select",
    "edit.selectAll",
    "edit.undo",
    "file.render",
    "file.saveBytes",
    "font.add",
    "font.replace",
    "font.rescan",
    "font.used",
    "format.align",
    "format.alignCenter",
    "format.alignLeft",
    "format.alignRight",
    "format.anchor",
    "format.autofit",
    "format.bold",
    "format.bullets",
    "format.caps",
    "format.case",
    "format.clear",
    "format.color",
    "format.columns",
    "format.direction",
    "format.font",
    "format.fontDialog",
    "format.grow",
    "format.highlight",
    "format.indent",
    "format.italic",
    "format.justify",
    "format.language",
    "format.lineSpacing",
    "format.margins",
    "format.numbering",
    "format.outdent",
    "format.painter",
    "format.painterApply",
    "format.paragraph",
    "format.paragraphDialog",
    "format.rtl",
    "format.ruler",
    "format.shrink",
    "format.size",
    "format.spacing",
    "format.state",
    "format.strikethrough",
    "format.subscript",
    "format.superscript",
    "format.tabs",
    "format.textOutline",
    "format.textShadow",
    "format.textTransparency",
    "format.textWarp",
    "format.underline",
    "format.wrap",
    "guide.add",
    "guide.delete",
    "guide.list",
    "guide.move",
    "handout.placeholders",
    "insert.actionButton",
    "insert.audio",
    "insert.chart",
    "insert.dateTime",
    "insert.hyperlink",
    "insert.icon",
    "insert.icons",
    "insert.photoAlbum",
    "insert.photoAlbumEdit",
    "insert.picture",
    "insert.slideNumber",
    "insert.smartArt",
    "insert.symbol",
    "insert.table",
    "insert.textBox",
    "insert.video",
    "insert.wordArt",
    "insert.zoom",
    "master.deleteLayout",
    "master.insertLayout",
    "master.insertPlaceholder",
    "master.placeholders",
    "master.preserve",
    "master.renameLayout",
    "media.gifPlay",
    "media.info",
    "media.options",
    "media.pause",
    "media.play",
    "media.posterFrame",
    "media.seek",
    "media.stop",
    "media.toggle",
    "notes.placeholders",
    "picture.adjust",
    "picture.artisticEffect",
    "picture.change",
    "picture.compress",
    "picture.crop",
    "picture.recolor",
    "picture.recolors",
    "picture.removeBackground",
    "picture.reset",
    "picture.style",
    "picture.styles",
    "picture.transparentColor",
    "review.accept",
    "review.accessibility",
    "review.addToDictionary",
    "review.compare",
    "review.dictionary",
    "review.endCompare",
    "review.replaceWord",
    "review.revisions",
    "review.spelling",
    "review.spellingChange",
    "review.thesaurus",
    "section.add",
    "section.move",
    "section.remove",
    "section.removeAll",
    "section.rename",
    "selection.hideAll",
    "selection.select",
    "selection.showAll",
    "shape.action",
    "shape.addPoint",
    "shape.adjust",
    "shape.altText",
    "shape.change",
    "shape.connect",
    "shape.deletePoint",
    "shape.editPoints",
    "shape.effects",
    "shape.fill",
    "shape.flip",
    "shape.freeform",
    "shape.insert",
    "shape.inspect",
    "shape.line",
    "shape.lock",
    "shape.merge",
    "shape.move",
    "shape.moveHandle",
    "shape.movePoint",
    "shape.pointType",
    "shape.points",
    "shape.presets",
    "shape.quickStyle",
    "shape.rename",
    "shape.resize",
    "shape.rotate",
    "shape.setBounds",
    "shape.setDefault",
    "shape.sites",
    "shape.sizePosition",
    "shape.smoothCorners",
    "shape.visible",
    "show.customShow",
    "show.fromCurrent",
    "show.fromStart",
    "show.setup",
    "slide.colorAt",
    "slide.delete",
    "slide.duplicate",
    "slide.first",
    "slide.fromOutline",
    "slide.go",
    "slide.hide",
    "slide.inspect",
    "slide.last",
    "slide.layout",
    "slide.move",
    "slide.new",
    "slide.next",
    "slide.notes",
    "slide.previous",
    "slide.rename",
    "slide.reset",
    "slide.reuse",
    "slide.selectSlides",
    "smartArt.addNode",
    "smartArt.colors",
    "smartArt.demote",
    "smartArt.fromText",
    "smartArt.layout",
    "smartArt.nodes",
    "smartArt.picture",
    "smartArt.promote",
    "smartArt.removeNode",
    "smartArt.setNodes",
    "smartArt.style",
    "smartArt.toShapes",
    "smartArt.toText",
    "table.borders",
    "table.cellAlign",
    "table.cellFill",
    "table.cellMargins",
    "table.columnWidth",
    "table.deleteColumn",
    "table.deleteRow",
    "table.distributeColumns",
    "table.distributeRows",
    "table.drawLine",
    "table.eraseLine",
    "table.insertColumnLeft",
    "table.insertColumnRight",
    "table.insertRowAbove",
    "table.insertRowBelow",
    "table.merge",
    "table.options",
    "table.rowHeight",
    "table.selectCells",
    "table.split",
    "table.style",
    "text.autocorrect",
    "text.commit",
    "text.compose",
    "text.delete",
    "text.edit",
    "text.exit",
    "text.find",
    "text.get",
    "text.insert",
    "text.move",
    "text.replace",
    "text.select",
    "text.selectAll",
    "text.selectParagraph",
    "text.selectWord",
    "text.set",
    "transition.applyAll",
    "transition.list",
    "transition.options",
    "transition.set",
    "transition.sound",
    "transition.timing",
    "view.closeMaster",
    "view.handoutMaster",
    "view.notesMaster",
    "view.slideMaster",
    "window.next",
];

/// Commands a locked session never runs.
pub const BLOCKED: &[&str] = &[
    "file.adopt",
    "file.close",
    "file.info",
    "file.new",
    "file.open",
    "file.openBytes",
    "file.properties",
    "file.recovery.discard",
    "file.recovery.list",
    "file.recovery.open",
    "file.recovery.save",
    "file.revert",
    "file.templates",
];

/// Commands that are off until the policy turns them on.
pub const OFF: &[&str] = &["file.export", "file.saveAs", "file.saveTemplate"];

/// Commands whose parameters decide, see [`permits`].
pub const CONDITIONAL: &[&str] = &["file.save"];

/// The grade of the engine command `id`; `None` for an id nobody classified.
pub fn grade(id: &str) -> Option<Grade> {
    if BLOCKED.contains(&id) {
        Some(Grade::Locked)
    } else if OFF.contains(&id) {
        Some(Grade::Off)
    } else if ALLOWED.contains(&id) || CONDITIONAL.contains(&id) {
        Some(Grade::On)
    } else {
        None
    }
}

/// Whether a session locked with `policy` (`None`: not locked) may run `id` with `params`.
pub fn permits(policy: Option<&Policy>, id: &str, params: &Json) -> bool {
    let Some(policy) = policy else { return true };
    let Some(grade) = grade(id) else { return false };
    if !policy.enabled(id, grade) {
        return false;
    }
    // Save keeps to the parent: a file name or path to write is Save As.
    id != "file.save" || params.get("path").is_none() || policy.enabled("file.saveAs", Grade::Off)
}

/// Whether `id` can run in some form (menus show it).
pub fn listed(policy: Option<&Policy>, id: &str) -> bool {
    match policy {
        None => true,
        Some(p) => grade(id).is_some_and(|g| p.enabled(id, g)),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::{EngineError, Session};
    use serde_json::json;

    fn locked() -> Session {
        let mut s = Session::new();
        s.lock_host();
        s.start_host_blank().unwrap();
        s
    }

    fn refused(s: &mut Session, id: &str, params: Json) -> bool {
        matches!(s.execute(id, &params), Err(EngineError::HostLocked(_)))
    }

    #[test]
    fn every_command_is_classified_exactly_once() {
        for c in crate::cmd::command_specs() {
            let n =
                [ALLOWED.contains(&c.id), BLOCKED.contains(&c.id), OFF.contains(&c.id), CONDITIONAL.contains(&c.id)].iter().filter(|b| **b).count();
            assert_eq!(n, 1, "command `{}` must be in exactly one of ALLOWED / BLOCKED / OFF / CONDITIONAL (host lock)", c.id);
        }
    }

    #[test]
    fn lists_hold_no_stale_ids_and_are_sorted() {
        for list in [ALLOWED, BLOCKED, OFF, CONDITIONAL] {
            assert!(list.windows(2).all(|w| w[0] < w[1]), "keep the lists sorted and free of duplicates");
            for id in list {
                assert!(crate::find_command(id).is_some(), "`{id}` is classified but is not a command");
            }
        }
    }

    #[test]
    fn locked_session_refuses_every_blocked_command() {
        let mut s = locked();
        for id in BLOCKED.iter().chain(OFF) {
            assert!(refused(&mut s, id, json!({})), "{id} ran");
        }
        assert!(refused(&mut s, "file.save", json!({"path": "/tmp/x.pptx"})));
        assert_eq!(s.documents().len(), 1, "nothing was opened, created or closed");
    }

    #[test]
    fn locked_session_still_edits() {
        let mut s = locked();
        s.execute("slide.new", &json!({})).unwrap();
        s.execute("insert.textBox", &json!({})).unwrap();
        assert!(s.execute("file.saveBytes", &json!({"format": "pptx"})).is_ok());
        assert!(s.execute("file.render", &json!({})).is_ok());
    }

    #[test]
    fn overrides_turn_off_commands_on_and_on_commands_off() {
        let mut s = locked();
        let policy = s.host_policy().unwrap();
        let dead = json!({"path": "/nonexistent-dir/x.pptx"});
        assert!(refused(&mut s, "file.saveAs", dead.clone()));
        policy.set(&[("file.saveAs".into(), true), ("slide.new".into(), false), ("no.such".into(), true)]);
        assert!(!refused(&mut s, "file.saveAs", dead.clone()), "an Off command turns on");
        assert!(refused(&mut s, "slide.new", json!({})), "an On command turns off");
        assert!(!refused(&mut s, "insert.textBox", json!({})), "the rest is untouched");
        policy.set(&[]);
        assert!(refused(&mut s, "file.saveAs", dead));
        assert!(!refused(&mut s, "slide.new", json!({})));
    }

    #[test]
    fn locked_commands_stay_locked_whatever_the_policy_says() {
        let mut s = locked();
        let overrides: Vec<(String, bool)> = BLOCKED.iter().map(|id| (id.to_string(), true)).collect();
        s.set_policy(&overrides);
        for id in BLOCKED {
            assert!(refused(&mut s, id, json!({"path": "/tmp/x.pptx"})), "{id} ran");
        }
        assert_eq!(s.documents().len(), 1);
    }

    #[test]
    fn save_with_a_path_follows_save_as() {
        let mut s = locked();
        let path = json!({"path": "/nonexistent-dir/x.pptx"});
        assert!(refused(&mut s, "file.save", path.clone()));
        s.set_policy(&[("file.saveAs".to_string(), true)]);
        assert!(!refused(&mut s, "file.save", path));
    }

    #[test]
    fn locked_commands_are_not_listed() {
        let s = locked();
        let listed = |s: &Session, id: &str| s.commands().iter().any(|c| c.id == id);
        for id in BLOCKED.iter().chain(OFF) {
            assert!(!listed(&s, id), "{id} is listed");
        }
        assert!(listed(&s, "file.save"));
        s.set_policy(&[("file.export".to_string(), true), ("file.save".to_string(), false)]);
        assert!(listed(&s, "file.export") && !listed(&s, "file.save"));
    }

    #[test]
    fn the_host_file_opens_and_nothing_else_does() {
        let mut source = Session::with_new();
        source.execute("insert.textBox", &json!({})).unwrap();
        let data = source.execute("file.saveBytes", &json!({"format": "pptx"})).unwrap()["data"].as_str().unwrap().to_string();

        let mut s = locked();
        s.open_host_file(&json!({"name": "Plan.pptx", "data": data})).unwrap();
        assert!(s.active().unwrap().title().contains("Plan"));
        assert!(refused(&mut s, "file.openBytes", json!({"name": "Plan.pptx", "data": data})));
        assert!(s.open_host_file(&json!({"name": "Plan.pptx"})).is_err(), "bad input is an error, not a panic");
    }

    #[test]
    fn an_unlocked_session_is_unchanged() {
        let mut s = Session::new();
        if HOST_BUILD {
            return;
        }
        s.execute("file.new", &json!({})).unwrap();
        assert!(!s.host_locked());
        assert!(s.host_policy().is_none());
        assert!(s.commands().iter().any(|c| c.id == "file.open"));
    }
}
