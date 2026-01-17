use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct OscillatingToggle {
    enabled: bool,
    updated_at: Instant,
    toggle_window: Duration,
}

static TOGGLE_STATE: OnceLock<RwLock<OscillatingToggle>> = OnceLock::new();

pub fn init(toggle_window: Duration) {
    let enabled = false;
    let updated_at = Instant::now();
    TOGGLE_STATE.get_or_init(|| {
        RwLock::new(OscillatingToggle {
            enabled,
            updated_at,
            toggle_window,
        })
    });
}

pub fn is_active() -> bool {
    let Some(toggle_state_lock) = TOGGLE_STATE.get() else {
        // Enabled if no toggle window was configured
        return true;
    };

    // Get the current state without holding onto a read lock.
    let now = Instant::now();
    let OscillatingToggle {
        mut enabled,
        updated_at,
        toggle_window,
    } = match toggle_state_lock.read() {
        Ok(current_state) => current_state.clone(),
        // Disable if we can't access the toggle state.
        _ => {
            tracing::warn!("Failed to access toggle state");
            return false;
        }
    };

    // Check how many periods of `toggle_window` seconds have passed since the last update. If an
    // odd number of periods have passed, flip the toggle.
    let periods_elapsed = now.duration_since(updated_at).as_secs() / toggle_window.as_secs();
    if periods_elapsed % 2 == 1 {
        tracing::info!("Toggle window is flipping from {enabled} to {}", !enabled);
        enabled = !enabled;
        let Ok(mut toggle_state) = toggle_state_lock.write() else {
            tracing::warn!("Failed to access toggle state");
            return false;
        };
        toggle_state.enabled = enabled;
        toggle_state.updated_at = now;
    }

    enabled
}
