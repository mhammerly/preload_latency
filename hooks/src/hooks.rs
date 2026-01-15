use std::collections::BTreeSet;
use std::sync::{Mutex, OnceLock};

use libc::{addrinfo, c_char, c_int, c_void, hostent, iovec, size_t, sockaddr, socklen_t, ssize_t};

use crate::config::HookConfig;
use crate::util;

static CONFIG: OnceLock<HookConfig> = OnceLock::new();

// List of addresses resolved for the hosts in `HOSTS`.
static HOST_ADDRS: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

// List of sockets connected to the IP addresses in `HOST_ADDRS`.
static HOST_SOCKETS: Mutex<BTreeSet<c_int>> = Mutex::new(BTreeSet::new());

/// Runs [`_ld_preload_init`] when the library is loaded.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".init_array")]
pub static LD_PRELOAD_INIT: extern "C" fn() = _ld_preload_init;

/// Runs on library load to set up hooks.
///
/// A list of hosts to intercept network calls for may be provided in the `PRELOAD_LATENCY_HOSTS`
/// environment variable as a colon-delimited list. If not specified, all hosts are intercepted.
///
/// If `PRELOAD_LATENCY_HOSTS` is set, each host must be resolved by a call to `getaddrinfo`. If
/// the main binary somehow bypasses `getaddrinfo` you may set the `PRELOAD_LATENCY_RESOLVE`
/// environment variable to resolve each host using `getaddrinfo` proactively at startup.
pub extern "C" fn _ld_preload_init() {
    tracing_subscriber::fmt::init();
    tracing::info!("Initializing hooks...");
    CONFIG.get_or_init(HookConfig::load);
    CONFIG.wait().maybe_proactively_resolve_hosts();
    tracing::info!("Initialization done.");
}

fn should_intercept_host(host: &str) -> bool {
    let config = CONFIG.wait();
    config.hosts.contains(host) || config.hosts.is_empty()
}

fn should_intercept_ip(ip: &String) -> bool {
    HOST_ADDRS
        .lock()
        .map(|addrs| addrs.contains(ip))
        .unwrap_or(CONFIG.wait().hosts.is_empty())
}

fn should_intercept_socket(socket: c_int) -> bool {
    // Definitely don't want to intercept stdin, stdout, stderr
    if socket <= 2 {
        false
    } else {
        HOST_SOCKETS
            .lock()
            .map(|sockets| sockets.contains(&socket))
            .unwrap_or(false)
    }
}

hook! {
    unsafe fn getaddrinfo(node: *const c_char, service: *const c_char, hints: *const addrinfo, res: *mut *mut addrinfo) -> c_int => w_getaddrinfo {
        unsafe {
            tracing::trace!("Entering getaddrinfo");
            let result = real!(getaddrinfo)(node, service, hints, res);

            if result == 0 && let Ok(node_str) = util::utf8_from_ptr(node) && should_intercept_host(node_str) && let Ok(mut addrs) = HOST_ADDRS.lock() {
                tracing::info!("Resolving tracked host: {node_str}");
                let mut addr = *res;
                while !addr.is_null() {
                    let ip = util::get_in_addr((*addr).ai_addr);
                    tracing::info!("> Tracking {ip}");
                    addrs.insert(ip);
                    addr = (*addr).ai_next;
                }
            }

            result
        }
    }
}

hook! {
    unsafe fn gethostbyname(name: *const c_char) -> hostent => w_gethostbyname {
        unsafe {
            tracing::trace!("Entering gethostbyname");
            real!(gethostbyname)(name)
        }
    }
}

hook! {
    unsafe fn gethostbyaddr(addr: *const c_void, size: socklen_t, addr_type: c_int) -> hostent => w_gethostbyaddr {
        unsafe {
            tracing::trace!("Entering gethostbyname");
            real!(gethostbyaddr)(addr, size, addr_type)
        }
    }
}

hook! {
    unsafe fn connect(socket: c_int, address: *const sockaddr, len: socklen_t) -> c_int => w_connect {
        unsafe {
            tracing::trace!("Entering connect");
            let result = real!(connect)(socket, address, len);

            let ip = util::get_in_addr(address);
            if should_intercept_ip(&ip) && let Ok(mut sockets) = HOST_SOCKETS.lock() {
                tracing::info!("Connecting socket to tracked IP: {ip}");
                tracing::info!("> {socket}");
                sockets.insert(socket);
            }

            result
        }
    }
}

hook! {
    unsafe fn bind(socket: c_int, address: *const sockaddr, address_len: socklen_t) -> c_int => w_bind {
        unsafe {
            tracing::trace!("Entering bind");
            let result = real!(bind)(socket, address, address_len);

            let ip = util::get_in_addr(address);
            if should_intercept_ip(&ip) && let Ok(mut sockets) = HOST_SOCKETS.lock() {
                tracing::info!("Binding socket to tracked IP: {ip}");
                tracing::info!("> {socket}");
                sockets.insert(socket);
            }

            result
        }
    }
}

hook! {
    unsafe fn send(socket: c_int, buf: *const c_void, len: size_t, flags: c_int) -> ssize_t => w_send {
        unsafe {
            tracing::trace!("Entering send");
            if should_intercept_socket(socket) {
                tracing::debug!("Sleeping before send() on socket {socket}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(send)(socket, buf, len, flags)
        }
    }
}

hook! {
    unsafe fn recv(socket: c_int, buf: *mut c_void, len: size_t, flags: c_int) -> ssize_t => w_recv {
        unsafe {
            tracing::trace!("Entering recv");
            if should_intercept_socket(socket) {
                tracing::debug!("Sleeping before recv() on socket {socket}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(recv)(socket, buf, len, flags)
        }
    }
}

hook! {
    unsafe fn sendto(socket: c_int, buf: *const c_void, len: size_t, flags: c_int, addr: *const sockaddr, addrlen: socklen_t) -> ssize_t => w_sendto {
        unsafe {
            tracing::trace!("Entering sendto");
            if should_intercept_socket(socket) {
                tracing::debug!("Sleeping before sendto() on socket {socket}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(sendto)(socket, buf, len, flags, addr, addrlen)
        }
    }
}

hook! {
    unsafe fn recvfrom(socket: c_int, buf: *mut c_void, len: size_t, flags: c_int, addr: *mut sockaddr, addrlen: socklen_t) -> ssize_t => w_recvfrom {
        unsafe {
            tracing::trace!("Entering recvfrom");
            if should_intercept_socket(socket) {
                tracing::debug!("Sleeping before recvfrom() on socket {socket}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(recvfrom)(socket, buf, len, flags, addr, addrlen)
        }
    }
}

hook! {
    unsafe fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t => w_write {
        unsafe {
            if should_intercept_socket(fd) {
                tracing::debug!("Sleeping before write() on socket {fd}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(write)(fd, buf, count)
        }
    }
}

hook! {
    unsafe fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t => w_read {
        unsafe {
            if should_intercept_socket(fd) {
                tracing::debug!("Sleeping before read() on socket {fd}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(read)(fd, buf, count)
        }
    }
}

hook! {
    unsafe fn writev(fd: c_int, iov: *const iovec, count: c_int) -> ssize_t => w_writev {
        unsafe {
            if should_intercept_socket(fd) {
                tracing::debug!("Sleeping before writev() on socket {fd}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(writev)(fd, iov, count)
        }
    }
}

hook! {
    unsafe fn readv(fd: c_int, iov: *const iovec, count: c_int) -> ssize_t => w_readv {
        unsafe {
            if should_intercept_socket(fd) {
                tracing::debug!("Sleeping before readv() on socket {fd}...");
                libc::usleep(CONFIG.wait().sleep_duration());
            }

            real!(readv)(fd, iov, count)
        }
    }
}
