mod state;

slint::include_modules!();

use slint::{ModelRc, VecModel};
use state::MixerState;
use std::cell::RefCell;
use std::rc::Rc;

fn layers_model(state: &MixerState) -> ModelRc<LayerData> {
    let rows: Vec<LayerData> = state
        .layers
        .iter()
        .map(|row| LayerData {
            name: row.name.clone().into(),
            source: row.source.clone().into(),
            opacity: row.opacity,
            visible: row.visible,
        })
        .collect();
    ModelRc::from(Rc::new(VecModel::from(rows)))
}

fn sync_ui(ui: &MixerWindow, state: &MixerState) {
    ui.set_layers(layers_model(state));
    ui.set_pgm_label(state.pgm_label.clone().into());
    ui.set_pst_label(state.pst_label.clone().into());
    ui.set_tbar_position(state.tbar_position);
    ui.set_transition_mode(state.transition_mode.clone().into());
    ui.set_recording(state.recording);
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MixerWindow::new()?;
    let state = Rc::new(RefCell::new(MixerState::default()));
    sync_ui(&ui, &state.borrow());

    ui.on_toggle_mute({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |index| {
            let mut state = state.borrow_mut();
            state.toggle_mute(index as usize);
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_set_opacity({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |index, value| {
            let mut state = state.borrow_mut();
            state.set_opacity(index as usize, value);
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_cut({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move || {
            let mut state = state.borrow_mut();
            state.cut();
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_trigger_transition({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |mode| {
            let mut state = state.borrow_mut();
            state.trigger_transition(mode.as_str());
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_set_tbar({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |position| {
            let mut state = state.borrow_mut();
            state.set_tbar(position);
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.run()
}
