use libc::{c_char, c_int, c_void, sockaddr, socklen_t};

unsafe extern "C" {
    /// The `libc` crate doesn't export this but we can extern it ourselves.
    fn inet_ntop(af: c_int, src: *const c_void, dst: *mut c_void, size: socklen_t)
    -> *const c_char;
}

/// Create a `*const c_char` (`libc` C string) from a UTF8 Rust `&str`.
pub unsafe fn utf8_from_ptr<'a>(ptr: *const c_char) -> Result<&'a str, std::str::Utf8Error> {
    unsafe { std::str::from_utf8(std::ffi::CStr::from_ptr(ptr).to_bytes()) }
}

/// Create a human-readable IP address `String` from a `*const sockaddr`. Returns
/// an empty string if the `sockaddr` is not IPv4 or IPv6.
pub unsafe fn get_in_addr(addr: *const sockaddr) -> String {
    unsafe {
        let mut buf = [0; 45];
        match (*addr).sa_family.into() {
            libc::AF_INET => {
                let addr = (*addr.cast::<libc::sockaddr_in>()).sin_addr;
                inet_ntop(
                    libc::AF_INET,
                    (&raw const addr).cast(),
                    buf.as_mut_ptr().cast(),
                    45,
                )
            }
            libc::AF_INET6 => {
                let addr = (*addr.cast::<libc::sockaddr_in6>()).sin6_addr;
                inet_ntop(
                    libc::AF_INET6,
                    (&raw const addr).cast(),
                    buf.as_mut_ptr().cast(),
                    45,
                )
            }
            _ => buf.as_ptr(),
        };
        utf8_from_ptr(buf.as_ptr()).unwrap_or("").to_owned()
    }
}
