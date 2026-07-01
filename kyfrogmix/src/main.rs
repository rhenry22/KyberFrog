mod api;
mod preview;
mod state;

slint::include_modules!();

use slint::{ModelRc, SharedString, VecModel};
use state::{MixerState, OutputFormatsState, RoutingState};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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

fn routing_rows_model(routing: &RoutingState) -> ModelRc<RoutingRow> {
    let rows: Vec<RoutingRow> = routing
        .rows
        .iter()
        .map(|row| RoutingRow {
            source: row.source.clone().into(),
            kind: row.kind.clone().into(),
            routed: ModelRc::from(Rc::new(VecModel::from(row.routed.clone()))),
        })
        .collect();
    ModelRc::from(Rc::new(VecModel::from(rows)))
}

fn sync_routing(ui: &MixerWindow, routing: &RoutingState) {
    let destinations: Vec<SharedString> =
        routing.destinations.iter().cloned().map(Into::into).collect();
    ui.set_routing_destinations(ModelRc::from(Rc::new(VecModel::from(destinations))));
    ui.set_routing_rows(routing_rows_model(routing));
}

fn output_format_rows_model(outputs: &OutputFormatsState) -> ModelRc<OutputFormatRow> {
    let rows: Vec<OutputFormatRow> = outputs
        .rows
        .iter()
        .map(|row| OutputFormatRow {
            name: row.name.clone().into(),
            format: row.format.clone().into(),
        })
        .collect();
    ModelRc::from(Rc::new(VecModel::from(rows)))
}

fn sync_output_formats(ui: &MixerWindow, outputs: &OutputFormatsState) {
    let formats: Vec<SharedString> =
        outputs.available_formats.iter().cloned().map(Into::into).collect();
    ui.set_output_formats(ModelRc::from(Rc::new(VecModel::from(formats))));
    ui.set_output_format_rows(output_format_rows_model(outputs));
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MixerWindow::new()?;
    // Arc<Mutex<..>> (pas Rc<RefCell<..>>) : cet état est aussi lu par le
    // thread de l'API HTTP (api.rs), qui tourne sur son propre runtime tokio
    // en dehors de l'event loop Slint.
    let state = Arc::new(Mutex::new(MixerState::default()));
    let routing = Arc::new(Mutex::new(RoutingState::default()));
    let outputs = Arc::new(Mutex::new(OutputFormatsState::default()));
    sync_ui(&ui, &state.lock().unwrap());
    sync_routing(&ui, &routing.lock().unwrap());
    sync_output_formats(&ui, &outputs.lock().unwrap());

    api::spawn(
        api::ApiState {
            mixer: state.clone(),
            routing: routing.clone(),
            outputs: outputs.clone(),
        },
        api::DEFAULT_PORT,
    );

    ui.on_toggle_mute({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |index| {
            let mut state = state.lock().unwrap();
            state.toggle_mute(index as usize);
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_set_opacity({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |index, value| {
            let mut state = state.lock().unwrap();
            state.set_opacity(index as usize, value);
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_cut({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move || {
            let mut state = state.lock().unwrap();
            state.cut();
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_trigger_transition({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |mode| {
            let mut state = state.lock().unwrap();
            state.trigger_transition(mode.as_str());
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_set_tbar({
        let state = state.clone();
        let ui_weak = ui.as_weak();
        move |position| {
            let mut state = state.lock().unwrap();
            state.set_tbar(position);
            sync_ui(&ui_weak.unwrap(), &state);
        }
    });

    ui.on_toggle_route({
        let routing = routing.clone();
        let ui_weak = ui.as_weak();
        move |row, col| {
            let mut routing = routing.lock().unwrap();
            routing.toggle_route(row as usize, col as usize);
            sync_routing(&ui_weak.unwrap(), &routing);
        }
    });

    ui.on_set_output_format({
        let outputs = outputs.clone();
        let ui_weak = ui.as_weak();
        move |index, format| {
            let mut outputs = outputs.lock().unwrap();
            outputs.set_format(index as usize, format.as_str());
            sync_output_formats(&ui_weak.unwrap(), &outputs);
        }
    });

    // Auto-nommées pour l'instant (pas de formulaire de saisie) — à remplacer
    // par un vrai picker (nom + type de source) une fois le modèle
    // shared::matrix branché.
    ui.on_add_routing_source({
        let routing = routing.clone();
        let ui_weak = ui.as_weak();
        move || {
            let mut routing = routing.lock().unwrap();
            let n = routing.rows.len() + 1;
            routing.add_source(format!("Nouvelle source {n}"), "kyber");
            sync_routing(&ui_weak.unwrap(), &routing);
        }
    });

    ui.on_remove_routing_source({
        let routing = routing.clone();
        let ui_weak = ui.as_weak();
        move |index| {
            let mut routing = routing.lock().unwrap();
            routing.remove_source(index as usize);
            sync_routing(&ui_weak.unwrap(), &routing);
        }
    });

    ui.on_add_routing_destination({
        let routing = routing.clone();
        let ui_weak = ui.as_weak();
        move || {
            let mut routing = routing.lock().unwrap();
            let n = routing.destinations.len() + 1;
            routing.add_destination(format!("Destination {n}"));
            sync_routing(&ui_weak.unwrap(), &routing);
        }
    });

    ui.on_remove_routing_destination({
        let routing = routing.clone();
        let ui_weak = ui.as_weak();
        move |index| {
            let mut routing = routing.lock().unwrap();
            routing.remove_destination(index as usize);
            sync_routing(&ui_weak.unwrap(), &routing);
        }
    });

    ui.run()
}
