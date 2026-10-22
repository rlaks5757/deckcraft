//! PPM host lock: a locked app keeps the editor and Save, and loses every way to reach local
//! files, printing, exports and links. The engine side is tested in `hostlock.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use deckcraft_engine::Session;
use deckcraft_engine::hostlock::BLOCKED_TEXT;
use serde_json::json;

use crate::{Inbox, InboxEvent, InboxFile, Services, SlideApp};

type Downloads = Arc<Mutex<Vec<String>>>;

/// Services that count every use of a platform feature the lock must remove.
fn counting(hits: &Arc<AtomicUsize>, downloads: &Downloads, inbox: Inbox) -> Services {
    let hit = || {
        let hits = hits.clone();
        move || {
            hits.fetch_add(1, Ordering::SeqCst);
        }
    };
    let (pick_open, pick_save, open_async) = (hit(), hit(), hit());
    let downloads = downloads.clone();
    Services {
        pick_open: Some(Box::new(move |_| {
            pick_open();
            None
        })),
        pick_save: Some(Box::new(move |_| {
            pick_save();
            None
        })),
        open_async: Some(Box::new(move |_, _| open_async())),
        download: Some(Box::new(move |name, _| downloads.lock().unwrap().push(name.to_string()))),
        inbox: Some(inbox),
        ..Default::default()
    }
}

fn harness(locked: bool, overrides: &[(&str, bool)], services: Services) -> egui_kittest::Harness<'static, SlideApp> {
    let mut session = Session::with_new();
    if locked {
        session.lock_host();
        session.set_policy(&overrides.iter().map(|(id, on)| (id.to_string(), *on)).collect::<Vec<_>>());
    }
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1280.0, 800.0)).build_ui_state(
        |ui, app: &mut SlideApp| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
        SlideApp::new(session, services),
    );
    h.run_steps(3);
    h
}

fn locked(overrides: &[(&str, bool)]) -> (egui_kittest::Harness<'static, SlideApp>, Arc<AtomicUsize>, Downloads, Inbox) {
    let hits = Arc::new(AtomicUsize::new(0));
    let downloads = Downloads::default();
    let inbox = Inbox::default();
    let mut services = counting(&hits, &downloads, inbox.clone());
    services.save_as_pptx = true;
    (harness(true, overrides, services), hits, downloads, inbox)
}

fn pptx_with_slides(extra: usize) -> Vec<u8> {
    let mut s = Session::with_new();
    for _ in 0..extra {
        s.execute("slide.new", &json!({})).unwrap();
    }
    deckcraft_engine::cmd::file::save_bytes(&s.active().unwrap().doc, "pptx").unwrap()
}

fn slides(app: &SlideApp) -> usize {
    app.session.active().unwrap().doc.slides.len()
}

#[test]
fn blocked_commands_do_nothing_quietly() {
    let (mut h, hits, downloads, _) = locked(&[]);
    for id in [
        "file.new",
        "file.open",
        "file.close",
        "file.revert",
        "file.templates",
        "file.export",
        "file.saveAs",
        "app.openDialog",
        "app.home",
        "app.print",
        "app.exportDialog",
        "app.saveAsDialog",
        "app.closePresentation",
    ] {
        let r = h.state_mut().run(id, json!({}));
        assert!(r.is_err_and(|e| e.contains(BLOCKED_TEXT)), "{id} must be refused by the lock");
        h.run_steps(2);
        assert!(h.state().status.is_none() && h.state().toast.is_none(), "{id} must refuse without a message");
    }
    assert_eq!(h.state().session.documents().len(), 1, "no presentation was created or closed");
    assert_eq!(hits.load(Ordering::SeqCst), 0, "no picker ran");
    assert!(downloads.lock().unwrap().is_empty(), "nothing was handed to the host");
}

#[test]
fn shortcuts_for_blocked_commands_do_nothing() {
    let (mut h, hits, downloads, _) = locked(&[]);
    for key in [egui::Key::O, egui::Key::P, egui::Key::N, egui::Key::W] {
        h.key_press_modifiers(egui::Modifiers::COMMAND, key);
        h.run_steps(3);
    }
    assert_eq!(h.state().session.documents().len(), 1);
    assert!(h.state().dialog.is_none(), "dialog");
    assert!(h.state().status.is_none(), "status {:?}", h.state().status);
    assert!(h.state().toast.is_none(), "toast");
    assert!(!h.state().backstage_open());
    assert_eq!(hits.load(Ordering::SeqCst), 0);
    assert!(downloads.lock().unwrap().is_empty());
}

#[test]
fn locked_app_still_edits() {
    let (mut h, ..) = locked(&[]);
    let before = slides(h.state());
    h.state_mut().run("slide.new", json!({})).unwrap();
    assert_eq!(slides(h.state()), before + 1);
    h.state_mut().run("edit.undo", json!({})).unwrap();
    assert_eq!(slides(h.state()), before);
}

#[test]
fn backstage_has_no_blocked_pages() {
    use crate::backstage::Page;
    let (mut h, ..) = locked(&[]);
    for page in [Page::Open, Page::New, Page::Home, Page::SaveAs, Page::Export, Page::Print] {
        h.state_mut().open_backstage(page);
        h.run_steps(2);
        assert!(!h.state().page_allowed(page));
        assert!(h.state().backstage.as_ref().is_some_and(|b| b.page == Page::Info), "{page:?} opened");
    }
}

#[test]
fn save_reaches_the_host_as_pptx_and_nothing_else_does() {
    let (mut h, hits, downloads, _) = locked(&[]);
    h.state_mut().save();
    h.run_steps(2);
    {
        let saved = downloads.lock().unwrap();
        assert_eq!(saved.len(), 1, "Save hands one file to the host");
        assert!(saved[0].ends_with(".pptx"), "{saved:?}");
    }
    assert!(!h.state_mut().export_as("pdf", json!({})), "export is off");
    assert!(!h.state_mut().print_pdf(json!({})), "printing is off");
    assert_eq!(downloads.lock().unwrap().len(), 1);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "no save picker ran");
}

#[test]
fn download_follows_the_policy() {
    for (allow, name, passes) in [
        (&[][..], "a.pptx", true),
        (&[][..], "a.pdf", false),
        (&[][..], "a.png", false),
        (&["app.print"][..], "a.pdf", true),
        (&["app.print"][..], "a.png", false),
        (&["file.export"][..], "a.pdf", true),
        (&["file.export"][..], "a.png", true),
    ] {
        let downloads = Downloads::default();
        let mut services = counting(&Arc::default(), &downloads, Inbox::default());
        let policy = deckcraft_engine::hostlock::Policy::default();
        policy.set(&allow.iter().map(|id| (id.to_string(), true)).collect::<Vec<_>>());
        services.lock_for_host(policy);
        (services.download.as_mut().unwrap())(name, b"x");
        assert_eq!(!downloads.lock().unwrap().is_empty(), passes, "{name} with {allow:?}");
    }
}

#[test]
fn services_lose_pickers_and_the_open_request() {
    let hits = Arc::new(AtomicUsize::new(0));
    let mut services = counting(&hits, &Downloads::default(), Inbox::default());
    services.lock_for_host(deckcraft_engine::hostlock::Policy::default());
    assert!(services.pick_open.is_none() && services.pick_save.is_none() && services.read.is_none() && services.write.is_none());
    let pick = services.open_async.as_mut().unwrap();
    pick("open", 1);
    pick("drop", 2);
    assert_eq!(hits.load(Ordering::SeqCst), 0, "a presentation can't be picked");
    pick("picture", 3);
    assert_eq!(hits.load(Ordering::SeqCst), 1, "a picture for the slide still can");
}

#[test]
fn external_links_are_stripped_unless_the_group_is_on() {
    for (allow, kept) in [(&[][..], 0), (&["app.externalLinks"][..], 1)] {
        let (h, ..) = locked(&allow.iter().map(|id| (*id, true)).collect::<Vec<_>>());
        h.ctx.output_mut(|o| o.commands.push(egui::OutputCommand::OpenUrl(egui::output::OpenUrl::new_tab("https://example.com"))));
        h.state().strip_host_output(&h.ctx);
        let n = h.ctx.output(|o| o.commands.iter().filter(|c| matches!(c, egui::OutputCommand::OpenUrl(_))).count());
        assert_eq!(n, kept, "{allow:?}");
    }
}

#[test]
fn the_host_opens_its_presentation_and_nothing_else_does() {
    let (mut h, _, _, inbox) = locked(&[]);
    // A drop is ignored.
    let id = h.state_mut().begin_external_file("drop");
    inbox.lock().unwrap().push(InboxEvent::File(InboxFile { id, name: "x.pptx".into(), bytes: pptx_with_slides(2) }));
    h.run_steps(3);
    assert_eq!(slides(h.state()), 1, "a dropped presentation did not open");
    // The host's file opens.
    let id = h.state_mut().begin_external_file("open");
    inbox.lock().unwrap().push(InboxEvent::File(InboxFile { id, name: "y.pptx".into(), bytes: pptx_with_slides(2) }));
    h.run_steps(3);
    assert_eq!(slides(h.state()), 3, "the host's presentation opened");
}

#[test]
fn overrides_turn_commands_on_and_off() {
    let (mut h, ..) = locked(&[("file.export", true), ("app.print", true), ("slide.new", false)]);
    assert!(h.state().allowed("file.export") && h.state().allowed("app.print"));
    assert!(!h.state().allowed("app.externalLinks") && !h.state().allowed("slide.new"));
    assert!(h.state_mut().run("slide.new", json!({})).is_err_and(|e| e.contains(BLOCKED_TEXT)), "an On command turned off");
    assert!(h.state_mut().run("file.new", json!({})).is_err(), "locked commands stay locked");
}

#[test]
fn an_unlocked_app_is_unchanged() {
    let mut h = harness(false, &[], Services::default());
    assert!(!h.state().host_locked());
    assert!(h.state().allowed("file.new") && h.state().allowed("app.openDialog"));
    h.state_mut().run("file.new", json!({})).unwrap();
    assert_eq!(h.state().session.documents().len(), 2);
}
