//! Copied objects reach another window as DXF text even when the source drawing is an older
//! version whose text is written in a code page: accented letters made that text invalid UTF-8,
//! nothing reached the system clipboard, and the other window said the clipboard was empty.

use cadcraft_engine::Session;
use cadcraft_engine::cmd::clipboard;
use cadcraft_engine::cmd::file::{IoHooks, set_io};
use cadcraft_engine::doc::{EntityKind, HVal};
use serde_json::json;

#[test]
fn an_accented_text_from_an_r12_drawing_travels_to_another_window() {
    set_io(IoHooks {
        read: |b, name| cadcraft_io::read(b, name).map_err(|e| e.to_string()),
        write: |d, name| cadcraft_io::write(d, name).map_err(|e| e.to_string()),
        plot: None,
    });
    let mut s = Session::new();
    s.doc_mut().unwrap().header.set("ACADVER", HVal::Str("AC1009".into()));
    s.execute("text", &json!({ "at": [5, 5], "text": "ÁRVORE", "height": 1 })).unwrap();
    let all: Vec<String> = s.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    s.execute("copyclip", &json!({ "handles": all })).unwrap();
    let text = clipboard::system_text(&s).expect("the copy becomes text for the system clipboard");
    assert!(text.contains("ÁRVORE"), "the letters travel as they are");

    // The other window takes it as its clipboard and pastes the same text.
    let mut t = Session::new();
    assert!(clipboard::load_system_text(&mut t, &text));
    t.execute("pasteorig", &json!({})).unwrap();
    assert!(t.doc().unwrap().model.iter().any(|e| matches!(&e.kind, EntityKind::Text(x) if x.value == "ÁRVORE")));
}
