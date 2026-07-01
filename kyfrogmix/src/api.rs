// Phase 1 du pilotage KyberFrog <-> kyfrogmix : une API HTTP en lecture
// seule, pour que le dashboard web KyberFrog (port 7700) puisse afficher
// l'état d'une instance kyfrogmix distante. Pas encore de contrôle
// (POST/mutations) — ça viendra en phase 2.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::state::{MixerState, OutputFormatsState, RoutingState};

pub const DEFAULT_PORT: u16 = 7800;

#[derive(Clone)]
pub struct ApiState {
    pub mixer: Arc<Mutex<MixerState>>,
    pub routing: Arc<Mutex<RoutingState>>,
    pub outputs: Arc<Mutex<OutputFormatsState>>,
}

#[derive(Serialize)]
struct StatusPayload {
    mixer: MixerState,
    routing: RoutingState,
    outputs: OutputFormatsState,
}

async fn status_handler(State(state): State<ApiState>) -> Json<StatusPayload> {
    Json(StatusPayload {
        mixer: state.mixer.lock().unwrap().clone(),
        routing: state.routing.lock().unwrap().clone(),
        outputs: state.outputs.lock().unwrap().clone(),
    })
}

/// Lance le serveur sur son propre thread + runtime tokio dédié — le thread
/// principal reste au event loop Slint, qui n'est pas Send/Sync et ne peut
/// pas partager un runtime tokio unique avec lui.
pub fn spawn(state: ApiState, port: u16) {
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(err) => {
                eprintln!("kyfrogmix API: impossible de démarrer le runtime tokio: {err}");
                return;
            }
        };

        rt.block_on(async move {
            let app = Router::new()
                .route("/status", get(status_handler))
                .with_state(state);

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            match tokio::net::TcpListener::bind(addr).await {
                Ok(listener) => {
                    println!("kyfrogmix API sur http://0.0.0.0:{port}/status");
                    if let Err(err) = axum::serve(listener, app).await {
                        eprintln!("kyfrogmix API: le serveur s'est arrêté: {err}");
                    }
                }
                Err(err) => {
                    eprintln!("kyfrogmix API désactivée: impossible de bind {addr}: {err}");
                }
            }
        });
    });
}
