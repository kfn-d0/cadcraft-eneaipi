//! Deferred clipboard operations keep their target; direct host reads cannot eat a new Ctrl+V.

#![cfg(not(target_arch = "wasm32"))]

use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services, menus};
use egui::{Context, Event, RawInput, ViewportCommand};
use serde_json::json;

fn app(services: Services) -> CadApp {
    let mut app = CadApp::new(Session::new(), services);
    let handles = app.run("line", json!({ "points": [[0, 0], [10, 0]] })).unwrap()["handles"].clone();
    app.run("copyclip", json!({ "handles": handles })).unwrap();
    app.session.new_drawing(false);
    app
}

fn frame(app: &mut CadApp, ctx: &Context, time: f64, events: Vec<Event>) -> bool {
    ctx.begin_pass(RawInput { time: Some(time), events, ..Default::default() });
    menus::shortcuts(app, ctx);
    let mut out = ctx.end_pass();
    out.textures_delta.clear();
    out.viewport_output.into_values().flat_map(|v| v.commands).any(|c| c == ViewportCommand::RequestPaste)
}

#[test]
fn deferred_paste_does_not_follow_a_layout_switch() {
    for name in ["Layout1", "Layout2"] {
        for answer in [false, true] {
            let mut app = app(Services::default());
            let ctx = Context::default();
            app.start("pasteorig");
            assert!(frame(&mut app, &ctx, 0.01, vec![]));
            app.run("layout.set", json!({ "name": name })).unwrap();
            let events = if answer { vec![Event::Paste("not DXF".into())] } else { vec![] };
            frame(&mut app, &ctx, 0.6, events);
            assert_eq!(app.session.doc().unwrap().entity_count(), 0, "no paste on {name}, answer={answer}");
            assert!(app.session.running.is_none());
        }
    }
}

#[test]
fn direct_empty_clipboard_answer_cannot_consume_the_next_keyboard_paste() {
    let services = Services { clipboard_text: Some(Box::new(|| None)), ..Services::default() };
    let mut app = app(services);
    app.set_clipboard_late_answers(false);
    let ctx = Context::default();
    app.start("pasteorig");
    assert!(!frame(&mut app, &ctx, 0.01, vec![]), "the host answered: no untagged paste request");
    let once = app.session.doc().unwrap().entity_count();
    assert!(once > 0, "the menu paste uses the drawing clipboard when there is no system text");
    frame(&mut app, &ctx, 0.02, vec![Event::Paste("not DXF".into())]);
    assert_eq!(app.session.running.as_ref().map(|r| r.id.as_str()), Some("pasteclip"), "the fresh Ctrl+V is not discarded");
    app.session.input(cadcraft_engine::Input::Point(cadcraft_geom::Vec2::new(20.0, 20.0))).unwrap();
    assert!(app.session.doc().unwrap().entity_count() > once, "the next paste really adds objects");
}

#[test]
fn direct_clipboard_reads_keep_table_pasting() {
    let services = Services { clipboard_text: Some(Box::new(|| Some("a	b
c	d".into()))), ..Services::default() };
    let mut app = app(services);
    let ctx = Context::default();
    app.start("pasteclip");
    assert!(!frame(&mut app, &ctx, 0.01, vec![]));
    assert_eq!(app.session.running.as_ref().map(|r| r.id.as_str()), Some("pastetable"));
}

#[test]
fn deferred_paste_does_not_follow_a_viewport_mode_switch() {
    use cadcraft_engine::doc::{Common, EntityKind, Space, Viewport};
    use cadcraft_geom::{Vec2, Vec3};

    let mut app = app(Services::default());
    let paper = Space::Paper("Layout1".into());
    app.run("layout.set", json!({ "name": "Layout1" })).unwrap();
    let vp = Viewport {
        id: 2,
        center: Vec3::new(50.0, 50.0, 0.0),
        width: 80.0,
        height: 60.0,
        view_center: Vec2::ZERO,
        view_height: 100.0,
        ..Default::default()
    };
    let handle = app.session.doc_mut().unwrap().add(&paper, Common::default(), EntityKind::Viewport(vp)).unwrap();
    let count = app.session.doc().unwrap().entity_count();
    let ctx = Context::default();
    app.start("pasteorig");
    assert!(frame(&mut app, &ctx, 0.01, vec![]));
    app.run("mspace", json!({ "handle": handle.hex() })).unwrap();
    frame(&mut app, &ctx, 0.02, vec![Event::Paste("not DXF".into())]);
    assert_eq!(app.session.doc().unwrap().entity_count(), count, "paper-space paste must not enter the model");
    app.start("pasteorig");
    assert!(frame(&mut app, &ctx, 0.03, vec![]));
    app.run("pspace", json!({})).unwrap();
    frame(&mut app, &ctx, 0.04, vec![Event::Paste("not DXF".into())]);
    assert_eq!(app.session.doc().unwrap().entity_count(), count, "model-space paste must not enter the sheet");
}
