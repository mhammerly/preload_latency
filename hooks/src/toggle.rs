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
    tracing::info!(
        "Initializing oscillating toggle; starts disabled but flips every {} seconds",
        toggle_window.as_secs()
    );
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
        mut updated_at,
        toggle_window,
    } = match toggle_state_lock.read() {
        Ok(current_state) => current_state.clone(),
        // Disable if we can't access the toggle state.
        _ => {
            tracing::warn!("Failed to access toggle state");
            return false;
        }
    };

    // Check how many periods of `toggle_window` seconds have passed since the last update. If >0
    // periods have passed, we must update the toggle state.
    let periods_elapsed = now.duration_since(updated_at).as_secs() / toggle_window.as_secs();
    if periods_elapsed > 0 {
        tracing::info!("Toggle period elapsed {periods_elapsed} times");

        // Move the `updated_at` time forward by `periods_elapsed` periods. If we're somehow
        // dealing with values outside the u32 range, pretend 0 periods have passed and leave
        // `updated_at` alone so we don't break anything.
        updated_at += toggle_window * periods_elapsed.try_into().unwrap_or(0);

        // We only have to flip the `enabled` toggle if an odd number of periods have passed.
        if periods_elapsed % 2 == 1 {
            tracing::debug!("Toggle state must flip from {} to {}.", enabled, !enabled);
            enabled = !enabled;
        } else {
            tracing::debug!("Toggle state stays the same at {enabled}");
        }

        let Ok(mut toggle_state) = toggle_state_lock.write() else {
            tracing::warn!("Failed to access toggle state");
            return false;
        };
        toggle_state.enabled = enabled;
        toggle_state.updated_at = updated_at;
    }

    enabled
}
