// Pure Rust mixer state — no dependency on `slint`, so it compiles and tests
// headless (no display server needed), same principle as `shared/` staying
// Win32-free in the main KyberFrog workspace.

const KNOWN_TRANSITIONS: [&str; 4] = ["cut", "mix", "wipe", "sting"];

#[derive(Clone, Debug, PartialEq)]
pub struct LayerRow {
    pub name: String,
    pub source: String,
    pub opacity: f32,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MixerState {
    pub layers: Vec<LayerRow>,
    pub pgm_label: String,
    pub pst_label: String,
    pub tbar_position: f32,
    pub transition_mode: String,
    pub recording: bool,
}

impl Default for MixerState {
    fn default() -> Self {
        // Placeholder rows until shared::matrix feeds the real routing table
        // (see VIDEO_MATRIX_MIXER.md — Phase A item 1).
        Self {
            layers: vec![
                LayerRow { name: "Layer 4 (top)".into(), source: "Regie-1".into(), opacity: 1.0, visible: true },
                LayerRow { name: "Layer 3".into(), source: "Resolume-out".into(), opacity: 0.8, visible: true },
                LayerRow { name: "Layer 2".into(), source: "BM-Sub0".into(), opacity: 1.0, visible: false },
                LayerRow { name: "Layer 1 (bg)".into(), source: "Regie-2".into(), opacity: 1.0, visible: true },
            ],
            pgm_label: "Regie-1".into(),
            pst_label: "Resolume-out".into(),
            tbar_position: 0.0,
            transition_mode: "mix".into(),
            recording: false,
        }
    }
}

impl MixerState {
    pub fn toggle_mute(&mut self, index: usize) {
        if let Some(row) = self.layers.get_mut(index) {
            row.visible = !row.visible;
        }
    }

    pub fn set_opacity(&mut self, index: usize, value: f32) {
        if let Some(row) = self.layers.get_mut(index) {
            row.opacity = value.clamp(0.0, 1.0);
        }
    }

    pub fn set_tbar(&mut self, position: f32) {
        self.tbar_position = position.clamp(0.0, 1.0);
    }

    pub fn trigger_transition(&mut self, mode: &str) {
        if KNOWN_TRANSITIONS.contains(&mode) {
            self.transition_mode = mode.to_string();
        }
    }

    pub fn cut(&mut self) {
        std::mem::swap(&mut self.pgm_label, &mut self.pst_label);
        self.tbar_position = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MixerState {
        MixerState {
            layers: vec![
                LayerRow { name: "L1".into(), source: "A".into(), opacity: 1.0, visible: true },
                LayerRow { name: "L2".into(), source: "B".into(), opacity: 0.5, visible: false },
            ],
            pgm_label: "A".into(),
            pst_label: "B".into(),
            tbar_position: 0.3,
            transition_mode: "mix".into(),
            recording: false,
        }
    }

    #[test]
    fn toggle_mute_flips_visibility() {
        let mut state = sample();
        state.toggle_mute(0);
        assert!(!state.layers[0].visible);
        state.toggle_mute(0);
        assert!(state.layers[0].visible);
    }

    #[test]
    fn toggle_mute_out_of_bounds_is_noop() {
        let mut state = sample();
        let before = state.clone();
        state.toggle_mute(99);
        assert_eq!(state, before);
    }

    #[test]
    fn set_opacity_clamps_to_unit_range() {
        let mut state = sample();
        state.set_opacity(0, 5.0);
        assert_eq!(state.layers[0].opacity, 1.0);
        state.set_opacity(0, -3.0);
        assert_eq!(state.layers[0].opacity, 0.0);
    }

    #[test]
    fn set_opacity_out_of_bounds_is_noop() {
        let mut state = sample();
        let before = state.clone();
        state.set_opacity(99, 0.5);
        assert_eq!(state, before);
    }

    #[test]
    fn set_tbar_clamps_to_unit_range() {
        let mut state = sample();
        state.set_tbar(2.0);
        assert_eq!(state.tbar_position, 1.0);
        state.set_tbar(-1.0);
        assert_eq!(state.tbar_position, 0.0);
    }

    #[test]
    fn cut_swaps_pgm_and_pst_and_resets_tbar() {
        let mut state = sample();
        state.cut();
        assert_eq!(state.pgm_label, "B");
        assert_eq!(state.pst_label, "A");
        assert_eq!(state.tbar_position, 0.0);
    }

    #[test]
    fn trigger_transition_accepts_known_modes() {
        let mut state = sample();
        state.trigger_transition("wipe");
        assert_eq!(state.transition_mode, "wipe");
    }

    #[test]
    fn trigger_transition_rejects_unknown_mode() {
        let mut state = sample();
        state.trigger_transition("teleport");
        assert_eq!(state.transition_mode, "mix");
    }

    #[test]
    fn default_seeds_four_layers() {
        let state = MixerState::default();
        assert_eq!(state.layers.len(), 4);
    }
}
