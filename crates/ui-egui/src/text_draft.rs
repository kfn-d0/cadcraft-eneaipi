//! Pending editor text must reach its original command before a close request checks dirty state.

use cadcraft_engine::Input;

use crate::CadApp;

pub(crate) struct Draft {
    doc: u64,
    command: u64,
    input: u64,
    answers: Vec<Input>,
}

pub(crate) fn stage(app: &mut CadApp, answers: Vec<Input>) {
    app.text_draft = app.session.state().ok().map(|st| Draft {
        doc: st.uid,
        command: app.session.commands_started,
        input: app.session.inputs,
        answers,
    });
}

/// Refuse closing if the command cannot take the pending text. Stale drafts never reach a new target.
pub(crate) fn commit(app: &mut CadApp) -> bool {
    let Some(mut draft) = app.text_draft.take() else { return true };
    if app.session.state().ok().map(|st| st.uid) != Some(draft.doc)
        || app.session.commands_started != draft.command
        || app.session.inputs != draft.input
    {
        return true;
    }
    for answer in draft.answers.clone() {
        if let Err(e) = app.session.input(answer) {
            app.set_status(e.to_string());
            draft.input = app.session.inputs;
            app.text_draft = Some(draft);
            return false;
        }
    }
    let still_editing = app.session.current_prompt().is_some_and(|p| p.accept.text && !p.accept.point)
        && (app.session.text_editor().is_some() || app.session.editing_mtext().is_some());
    if still_editing {
        draft.input = app.session.inputs;
        app.text_draft = Some(draft);
        app.set_status(crate::tl!("Finish or cancel the text edit before closing.").to_string());
        return false;
    }
    true
}
