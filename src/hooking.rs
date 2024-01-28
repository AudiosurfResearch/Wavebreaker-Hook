use std::ffi::c_int;
use std::ffi::CString;

use tracing::trace;
use windows::core::s;
use windows::core::PCSTR;
use windows::Win32::Networking::WinInet::INTERNET_FLAG_RELOAD;
use windows::Win32::Networking::WinInet::INTERNET_FLAG_SECURE;

use crate::config::CONFIG;

#[crochet::hook(compile_check, "Wininet.dll", "InternetConnectA")]
unsafe fn connect_hook(
    hinternet: c_int,
    server_name: PCSTR,
    mut port: u16,
    username: PCSTR,
    password: PCSTR,
    service: u32,
    flags: u32,
    context: usize,
) -> c_int {
    trace!(
        "connect_hook called: {:?} {:?}",
        CString::from_vec_unchecked(server_name.as_bytes().to_vec()),
        port
    );
    let config = CONFIG.get().unwrap();
    if config.main.force_insecure && port == 443 {
        port = 80;
    } else if !config.main.force_insecure && port == 80 {
        port = 443;
    }

    call_original!(
        hinternet,
        server_name,
        port,
        username,
        password,
        service,
        flags,
        context
    )
}

#[crochet::hook(compile_check, "Wininet.dll", "HttpOpenRequestA")]
unsafe fn openrequest_hook(
    hconnect: c_int,
    verb: PCSTR,
    object_name: PCSTR,
    version: PCSTR,
    referrer: PCSTR,
    accept_types: *const PCSTR,
    mut flags: u32,
    context: usize,
) -> c_int {
    trace!(
        "openrequest_hook called: {:?} {:?} {:?}",
        CString::from_vec_unchecked(verb.as_bytes().to_vec()),
        CString::from_vec_unchecked(object_name.as_bytes().to_vec()),
        flags
    );
    let config = CONFIG.get().unwrap();

    if config.main.force_insecure {
        flags &= !INTERNET_FLAG_SECURE;
    } else {
        if flags == 0 {
            flags = INTERNET_FLAG_SECURE;
        }
        if flags == INTERNET_FLAG_RELOAD {
            flags = INTERNET_FLAG_RELOAD | INTERNET_FLAG_SECURE;
        }
    }

    trace!("new OpenRequest flags: {:?}", flags);

    call_original!(
        hconnect,
        verb,
        object_name,
        version,
        referrer,
        accept_types,
        flags,
        context
    )
}

pub fn init_hooks() -> anyhow::Result<()> {
    crochet::enable!(connect_hook)?;
    crochet::enable!(openrequest_hook)?;

    Ok(())
}
