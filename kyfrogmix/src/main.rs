slint::include_modules!();

use slint::{ModelRc, VecModel};
use std::rc::Rc;

fn main() -> Result<(), slint::PlatformError> {
    let ui = MixerWindow::new()?;

    // Placeholder rows until shared::matrix feeds the real routing table
    // (see VIDEO_MATRIX_MIXER.md — Phase A item 1).
    let layers = Rc::new(VecModel::from(vec![
        LayerData { name: "Layer 4 (top)".into(), source: "Regie-1".into(), opacity: 1.0, visible: true },
        LayerData { name: "Layer 3".into(), source: "Resolume-out".into(), opacity: 0.8, visible: true },
        LayerData { name: "Layer 2".into(), source: "BM-Sub0".into(), opacity: 1.0, visible: false },
        LayerData { name: "Layer 1 (bg)".into(), source: "Regie-2".into(), opacity: 1.0, visible: true },
    ]));
    ui.set_layers(ModelRc::from(layers.clone()));

    ui.on_toggle_mute({
        let layers = layers.clone();
        move |index| {
            if let Some(mut row) = layers.row_data(index as usize) {
                row.visible = !row.visible;
                layers.set_row_data(index as usize, row);
            }
        }
    });

    ui.on_set_opacity({
        let layers = layers.clone();
        move |index, value| {
            if let Some(mut row) = layers.row_data(index as usize) {
                row.opacity = value;
                layers.set_row_data(index as usize, row);
            }
        }
    });

    ui.on_cut({
        let ui_weak = ui.as_weak();
        move || {
            let ui = ui_weak.unwrap();
            let pgm = ui.get_pgm_label();
            let pst = ui.get_pst_label();
            ui.set_pgm_label(pst);
            ui.set_pst_label(pgm);
            ui.set_tbar_position(0.0);
        }
    });

    ui.on_trigger_transition({
        let ui_weak = ui.as_weak();
        move |mode| {
            let ui = ui_weak.unwrap();
            ui.set_transition_mode(mode);
        }
    });

    ui.on_set_tbar({
        let ui_weak = ui.as_weak();
        move |position| {
            let ui = ui_weak.unwrap();
            ui.set_tbar_position(position.clamp(0.0, 1.0));
        }
    });

    ui.run()
}
