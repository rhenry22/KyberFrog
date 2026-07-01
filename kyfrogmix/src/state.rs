// Pure Rust mixer state — no dependency on `slint`, so it compiles and tests
// headless (no display server needed), same principle as `shared/` staying
// Win32-free in the main KyberFrog workspace.

use serde::Serialize;

const KNOWN_TRANSITIONS: [&str; 4] = ["cut", "mix", "wipe", "sting"];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LayerRow {
    pub name: String,
    pub source: String,
    pub opacity: f32,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RouteRow {
    pub source: String,
    pub kind: String,
    pub routed: Vec<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RoutingState {
    pub destinations: Vec<String>,
    pub rows: Vec<RouteRow>,
}

impl Default for RoutingState {
    fn default() -> Self {
        // Placeholder registry until shared::matrix feeds the real
        // source/destination list (see VIDEO_MATRIX_MIXER.md — Phase A item 1).
        fn routed_at(len: usize, on: &[usize]) -> Vec<bool> {
            let mut v = vec![false; len];
            for &i in on {
                v[i] = true;
            }
            v
        }
        let destinations: Vec<String> = ["Kyclient-1", "Kyclient-2", "Mixer L1", "Mixer L2"]
            .into_iter()
            .map(String::from)
            .collect();
        let n = destinations.len();
        Self {
            rows: vec![
                RouteRow { source: "Regie-1".into(), kind: "kyber".into(), routed: routed_at(n, &[0]) },
                RouteRow { source: "Regie-2".into(), kind: "kyber".into(), routed: routed_at(n, &[1]) },
                RouteRow { source: "Resolume-out".into(), kind: "spout".into(), routed: routed_at(n, &[2]) },
                RouteRow { source: "BM-Sub0".into(), kind: "capture".into(), routed: routed_at(n, &[3]) },
                RouteRow { source: "BM-Sub1".into(), kind: "capture".into(), routed: routed_at(n, &[]) },
            ],
            destinations,
        }
    }
}

impl RoutingState {
    pub fn toggle_route(&mut self, row: usize, col: usize) {
        if let Some(cell) = self.rows.get_mut(row).and_then(|r| r.routed.get_mut(col)) {
            *cell = !*cell;
        }
    }

    pub fn add_source(&mut self, name: impl Into<String>, kind: impl Into<String>) {
        let routed = vec![false; self.destinations.len()];
        self.rows.push(RouteRow { source: name.into(), kind: kind.into(), routed });
    }

    pub fn remove_source(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
        }
    }

    pub fn add_destination(&mut self, name: impl Into<String>) {
        self.destinations.push(name.into());
        for row in &mut self.rows {
            row.routed.push(false);
        }
    }

    pub fn remove_destination(&mut self, index: usize) {
        if index >= self.destinations.len() {
            return;
        }
        self.destinations.remove(index);
        for row in &mut self.rows {
            if index < row.routed.len() {
                row.routed.remove(index);
            }
        }
    }
}

// Standards vidéo proposés pour les sorties DeckLink — liste à valider
// empiriquement avec `ffmpeg -f decklink -list_formats 1` sur la carte réelle
// (cf. Blackmagic_PCie_int.md, "Points ouverts").
const KNOWN_FORMATS: [&str; 8] = [
    "1080p50", "1080p59.94", "1080p60",
    "1080i50", "1080i59.94",
    "2160p29.97", "2160p50", "2160p59.94",
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OutputFormatRow {
    pub name: String,
    pub format: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OutputFormatsState {
    pub available_formats: Vec<String>,
    pub rows: Vec<OutputFormatRow>,
}

impl Default for OutputFormatsState {
    fn default() -> Self {
        // Placeholder : les 4 sous-devices de la DeckLink 4K Pro, tous en
        // rôle "sortie" ici — l'assignation capture/sortie par sous-device
        // (Blackmagic_PCie_int.md §"Choix d'architecture") reste à modéliser
        // séparément, cette vue ne couvre que le standard vidéo des sorties.
        Self {
            available_formats: KNOWN_FORMATS.iter().map(|s| s.to_string()).collect(),
            rows: vec![
                OutputFormatRow { name: "Sortie 1 (sub-device 0)".into(), format: "1080p59.94".into() },
                OutputFormatRow { name: "Sortie 2 (sub-device 1)".into(), format: "1080p59.94".into() },
                OutputFormatRow { name: "Sortie 3 (sub-device 2)".into(), format: "1080i59.94".into() },
                OutputFormatRow { name: "Sortie 4 (sub-device 3)".into(), format: "2160p29.97".into() },
            ],
        }
    }
}

impl OutputFormatsState {
    pub fn set_format(&mut self, index: usize, format: &str) {
        if !KNOWN_FORMATS.contains(&format) {
            return;
        }
        if let Some(row) = self.rows.get_mut(index) {
            row.format = format.to_string();
        }
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

    fn sample_routing() -> RoutingState {
        RoutingState {
            destinations: vec!["D1".into(), "D2".into()],
            rows: vec![
                RouteRow { source: "S1".into(), kind: "kyber".into(), routed: vec![true, false] },
                RouteRow { source: "S2".into(), kind: "spout".into(), routed: vec![false, false] },
            ],
        }
    }

    #[test]
    fn toggle_route_flips_cell() {
        let mut routing = sample_routing();
        routing.toggle_route(0, 1);
        assert!(routing.rows[0].routed[1]);
        routing.toggle_route(0, 1);
        assert!(!routing.rows[0].routed[1]);
    }

    #[test]
    fn toggle_route_out_of_bounds_row_is_noop() {
        let mut routing = sample_routing();
        let before = routing.clone();
        routing.toggle_route(99, 0);
        assert_eq!(routing, before);
    }

    #[test]
    fn toggle_route_out_of_bounds_col_is_noop() {
        let mut routing = sample_routing();
        let before = routing.clone();
        routing.toggle_route(0, 99);
        assert_eq!(routing, before);
    }

    #[test]
    fn default_routing_seeds_expected_destinations() {
        let routing = RoutingState::default();
        assert_eq!(routing.destinations.len(), 4);
        assert_eq!(routing.rows.len(), 5);
    }

    #[test]
    fn add_source_appends_row_with_matching_routed_length() {
        let mut routing = sample_routing();
        routing.add_source("S3", "spout");
        assert_eq!(routing.rows.len(), 3);
        assert_eq!(routing.rows[2].source, "S3");
        assert_eq!(routing.rows[2].routed.len(), routing.destinations.len());
        assert!(routing.rows[2].routed.iter().all(|&r| !r));
    }

    #[test]
    fn remove_source_removes_the_right_row() {
        let mut routing = sample_routing();
        routing.remove_source(0);
        assert_eq!(routing.rows.len(), 1);
        assert_eq!(routing.rows[0].source, "S2");
    }

    #[test]
    fn remove_source_out_of_bounds_is_noop() {
        let mut routing = sample_routing();
        let before = routing.clone();
        routing.remove_source(99);
        assert_eq!(routing, before);
    }

    #[test]
    fn add_destination_extends_every_row() {
        let mut routing = sample_routing();
        routing.add_destination("D3");
        assert_eq!(routing.destinations.len(), 3);
        for row in &routing.rows {
            assert_eq!(row.routed.len(), 3);
            assert!(!row.routed[2]);
        }
    }

    #[test]
    fn remove_destination_removes_the_right_column_everywhere() {
        let mut routing = sample_routing();
        routing.remove_destination(0);
        assert_eq!(routing.destinations, vec!["D2".to_string()]);
        assert_eq!(routing.rows[0].routed, vec![false]); // was [true, false], col 0 removed
    }

    #[test]
    fn remove_destination_out_of_bounds_is_noop() {
        let mut routing = sample_routing();
        let before = routing.clone();
        routing.remove_destination(99);
        assert_eq!(routing, before);
    }

    #[test]
    fn set_format_accepts_known_format() {
        let mut outputs = OutputFormatsState::default();
        outputs.set_format(0, "2160p50");
        assert_eq!(outputs.rows[0].format, "2160p50");
    }

    #[test]
    fn set_format_rejects_unknown_format() {
        let mut outputs = OutputFormatsState::default();
        let before = outputs.clone();
        outputs.set_format(0, "8k144");
        assert_eq!(outputs, before);
    }

    #[test]
    fn set_format_out_of_bounds_is_noop() {
        let mut outputs = OutputFormatsState::default();
        let before = outputs.clone();
        outputs.set_format(99, "1080p60");
        assert_eq!(outputs, before);
    }

    #[test]
    fn default_output_formats_seeds_four_subdevices() {
        let outputs = OutputFormatsState::default();
        assert_eq!(outputs.rows.len(), 4);
    }
}
