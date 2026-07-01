// Downscale/throttle rules for live source thumbnails — pure logic, no
// dependency on `slint` or a capture backend, so it's testable without the
// V4L2 hardware this dev environment doesn't have.

use std::time::{Duration, Instant};

/// Cadence cible des vignettes, indépendante du framerate natif de la source
/// (ex: une capture à 59.94 fps ne pousse une image vers l'UI qu'à 10 fps).
pub const PREVIEW_FPS: f64 = 10.0;

/// Les vignettes sont 10x plus petites (en largeur et hauteur) que la source.
pub const PREVIEW_SCALE_DIVISOR: u32 = 10;

/// Dimensions de la vignette pour une source de `source_width`x`source_height`.
/// Toujours au moins 1px de chaque côté, même pour une source minuscule.
pub fn preview_dimensions(source_width: u32, source_height: u32) -> (u32, u32) {
    (
        (source_width / PREVIEW_SCALE_DIVISOR).max(1),
        (source_height / PREVIEW_SCALE_DIVISOR).max(1),
    )
}

/// Décide si une frame source doit être poussée vers la vignette, en cadençant
/// à `PREVIEW_FPS` peu importe le rythme d'arrivée des frames en amont.
pub struct PreviewThrottle {
    min_interval: Duration,
    last_emit: Option<Instant>,
}

impl PreviewThrottle {
    pub fn new() -> Self {
        Self {
            min_interval: Duration::from_secs_f64(1.0 / PREVIEW_FPS),
            last_emit: None,
        }
    }

    /// À appeler à chaque frame source disponible ; renvoie `true` si cette
    /// frame doit être downscalée et envoyée à l'UI.
    pub fn should_emit(&mut self, now: Instant) -> bool {
        match self.last_emit {
            Some(last) if now.duration_since(last) < self.min_interval => false,
            _ => {
                self.last_emit = Some(now);
                true
            }
        }
    }
}

impl Default for PreviewThrottle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_dimensions_divides_by_ten() {
        assert_eq!(preview_dimensions(1920, 1080), (192, 108));
    }

    #[test]
    fn preview_dimensions_floors_at_one_pixel() {
        assert_eq!(preview_dimensions(5, 5), (1, 1));
    }

    #[test]
    fn throttle_emits_on_first_call() {
        let mut throttle = PreviewThrottle::new();
        assert!(throttle.should_emit(Instant::now()));
    }

    #[test]
    fn throttle_rejects_frame_too_soon() {
        let mut throttle = PreviewThrottle::new();
        let t0 = Instant::now();
        assert!(throttle.should_emit(t0));
        // 50ms later is well under the 100ms (10fps) interval.
        assert!(!throttle.should_emit(t0 + Duration::from_millis(50)));
    }

    #[test]
    fn throttle_emits_again_after_interval_elapsed() {
        let mut throttle = PreviewThrottle::new();
        let t0 = Instant::now();
        assert!(throttle.should_emit(t0));
        assert!(throttle.should_emit(t0 + Duration::from_millis(110)));
    }
}
