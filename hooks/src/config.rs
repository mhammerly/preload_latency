use std::collections::BTreeSet;
use std::net::ToSocketAddrs;

use libc::c_uint;

/// Configuration options for the hooks in [`crate::hooks`].
pub struct HookConfig {
    /// List of hosts to intercept. If empty, intercept all hosts.
    ///
    /// Read from a colon-separated list in the `PRELOAD_LATENCY_HOSTS` environment variable.
    ///
    /// If the `PRELOAD_LATENCY_RESOLVE` environment variable is set, these hosts are
    /// optimistically resolved using `getaddrinfo`. This is useful when a main binary somehow
    /// bypasses `getaddrinfo` when creating sockets for a host that should be intercepted.
    pub(crate) hosts: BTreeSet<String>,

    /// Duration in milliseconds to sleep before reading from or writing to intercepted sockets.
    ///
    /// Read from the `PRELOAD_LATENCY_MILLIS` environment variable.
    pub(crate) sleep_duration_millis: c_uint,
}

impl HookConfig {
    pub fn load() -> Self {
        let hosts = match std::env::var("PRELOAD_LATENCY_HOSTS") {
            Ok(hosts) => hosts.split(':').map(str::to_owned).collect(),
            _ => BTreeSet::new(),
        };

        let sleep_duration_millis = std::env::var("PRELOAD_LATENCY_MILLIS")
            .unwrap_or_default()
            .parse()
            .unwrap_or(200);

        Self {
            hosts,
            sleep_duration_millis,
        }
    }

    pub(crate) fn maybe_proactively_resolve_hosts(&self) {
        if std::env::var("PRELOAD_LATENCY_RESOLVE").is_ok() {
            for host in self.hosts.iter() {
                tracing::info!("Pre-resolving {host}...");
                // `to_socket_addrs()` goes through `getaddrinfo()` which tracks the results for us.
                let Ok(_resolved_addrs) = format!("{host}:80").as_str().to_socket_addrs() else {
                    tracing::warn!("Failed to resolve `{host}:80`");
                    continue;
                };
            }
        }
    }

    pub(crate) fn sleep_duration(&self) -> c_uint {
        self.sleep_duration_millis * 1000
    }
}
