//! LAYOUT / -LAYOUT typed at the command line asks for an option and the layout names it needs
//! instead of running the next inputs as commands.

use cadcraft_doc::Space;
use cadcraft_engine::Session;

fn layouts(s: &Session) -> Vec<String> {
    let mut ls: Vec<_> = s.doc().unwrap().layouts.iter().map(|l| (l.tab_order, l.name.clone())).collect();
    ls.sort();
    ls.into_iter().map(|(_, n)| n).collect()
}

#[test]
fn typed_layout_options_prompt_for_names() {
    let mut s = Session::new();
    assert_eq!(layouts(&s), ["Layout1", "Layout2"]);

    s.cmdline("LAYOUT").unwrap();
    let p = s.current_prompt().expect("LAYOUT prompts").display();
    assert_eq!(p, "Enter layout option [Copy/Delete/New/Template/Rename/SAveas/Set/?] <set>:");
    s.cmdline("n").unwrap();
    assert_eq!(s.current_prompt().unwrap().display(), "Enter name of new layout <Layout3>:");
    s.cmdline("Sheet9").unwrap();
    assert!(s.current_prompt().is_none());
    assert_eq!(layouts(&s), ["Layout1", "Layout2", "Sheet9"]);

    // Script form, names with spaces, and the current layout as the default.
    s.script("-LAYOUT S Layout2\n-LAYOUT R\nLayout2\nSheet A\n-LAYOUT C\n\n\n").unwrap();
    assert_eq!(s.layout_space(), Space::Paper("Sheet A".into()));
    assert_eq!(layouts(&s), ["Layout1", "Sheet A", "Sheet9", "Sheet A (2)"]);
    s.script("-LAYOUT D Sheet9\n").unwrap();
    assert_eq!(layouts(&s), ["Layout1", "Sheet A", "Sheet A (2)"]);

    // An invalid name re-prompts; Enter at the option prompt is Set.
    s.cmdline("LAYOUT N").unwrap();
    s.cmdline("Layout1").unwrap();
    assert!(s.current_prompt().is_some_and(|p| p.message == "Enter name of new layout"));
    s.input(cadcraft_engine::Input::Cancel).unwrap();
    s.script("LAYOUT\n\nModel\n").unwrap();
    assert_eq!(s.layout_space(), Space::Model);

    // ? lists; Template reports and ends without taking the next input.
    let n = s.log.len();
    s.cmdline("LAYOUT ? Sheet*").unwrap();
    assert!(s.log[n..].iter().any(|l| l == "Layout: Sheet A (2)"), "{:?}", &s.log[n..]);
    assert!(!s.log[n..].iter().any(|l| l == "Layout: Layout1"));
    let n = s.log.len();
    s.cmdline("LAYOUT T").unwrap();
    assert!(s.current_prompt().is_none());
    assert!(s.log[n..].iter().any(|l| l.contains("not available yet")), "{:?}", &s.log[n..]);
    assert_eq!(layouts(&s), ["Layout1", "Sheet A", "Sheet A (2)"]);

    // The whole command is one undo step.
    s.cmdline("LAYOUT N Sheet7").unwrap();
    s.undo().unwrap();
    assert_eq!(layouts(&s), ["Layout1", "Sheet A", "Sheet A (2)"]);
}

#[test]
fn switching_layout_finishes_the_command_in_its_original_space() {
    use cadcraft_engine::Input;
    use cadcraft_geom::Vec2;
    use serde_json::json;

    for (from, to) in [("Model", "Layout1"), ("Layout1", "Layout2"), ("Layout2", "Model")] {
        let mut s = Session::new();
        s.execute("layout.set", &json!({ "name": from })).unwrap();
        s.cmdline("LINE 0,0 10,0").unwrap();
        assert!(s.running.is_some());
        // An invalid destination must leave the current command alone.
        assert!(s.execute("layout.set", &json!({ "name": "Missing layout" })).is_err());
        assert!(s.running.is_some());
        let original = s.doc().unwrap().space(&s.space()).unwrap().clone();
        s.execute("layout.set", &json!({ "name": to })).unwrap();
        assert!(s.running.is_none(), "{from} -> {to}");
        let count = s.doc().unwrap().entity_count();
        s.input(Input::Point(Vec2::new(20.0, 0.0))).unwrap();
        assert_eq!(s.doc().unwrap().entity_count(), count, "the old LINE cannot add a segment on {to}");
        let source = if from == "Model" { Space::Model } else { Space::Paper(from.into()) };
        assert_eq!(*s.doc().unwrap().space(&source).unwrap(), original, "completed segments stay on {from}");
    }
}
