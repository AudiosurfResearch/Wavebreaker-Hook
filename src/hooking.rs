use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::CStr;
use std::ffi::CString;

use tracing::trace;
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

#[crochet::hook(
    "HTTP_Fetch_Unicode.dll",
    "?GetTargetServer@HTTP_Fetch_Unicode@@UAEPADXZ"
)]
unsafe extern "thiscall" fn gettargetserver_unicode_hook(this_ptr: c_int) -> *const c_char {
    trace!("gettargetserver_unicode_hook called: {:?}", this_ptr);

    let orig_result = call_original!(this_ptr);
    let c_str = CStr::from_ptr(orig_result);
    trace!(
        "gettargetserver_unicode_hook original result: {:?}",
        c_str.to_str()
    );

    let new_str = rewrite_server(c_str.to_str().unwrap());
    trace!("Server rewritten to: {:?}", &new_str);
    
    malloc_c_string(&new_str) as *const c_char
}

#[crochet::hook(
    "17C5B19F-4273-423C-A158-CA6F73046D43.dll",
    "?GetTargetServer@Aco_HTTP_Fetch@@UAEPADXZ"
)]
unsafe extern "thiscall" fn gettargetserver_hook(this_ptr: c_int) -> *const c_char {
    trace!("gettargetserver_hook called: {:?}", this_ptr);

    let orig_result = call_original!(this_ptr);
    let c_str = CStr::from_ptr(orig_result);
    trace!(
        "gettargetserver_hook original result: {:?}",
        c_str.to_str()
    );

    let new_str = rewrite_server(c_str.to_str().unwrap());
    trace!("Server rewritten to: {:?}", &new_str);
    
    malloc_c_string(&new_str) as *const c_char
}

unsafe fn malloc_c_string(s: &str) -> *mut () {
    // this is an absolute meme
    let c_str = CString::new(s).unwrap();
    let memory = malloc(c_str.as_bytes_with_nul().len());
    std::ptr::copy_nonoverlapping(
        c_str.as_bytes_with_nul().as_ptr(),
        memory as *mut u8,
        c_str.as_bytes_with_nul().len(),
    );
    memory
}

extern "C" {
    fn malloc(n: usize) -> *mut ();
}

fn rewrite_server(server: &str) -> String {
    let config = CONFIG.get().unwrap();
    if server.contains("audiosurfthegame") || server.contains("audio-surf") {
        config.main.server.clone()
    } else {
        server.to_string()
    }
}

pub fn init_hooks() -> anyhow::Result<()> {
    crochet::enable!(connect_hook)?;
    crochet::enable!(openrequest_hook)?;
    crochet::enable!(gettargetserver_unicode_hook)?;
    crochet::enable!(gettargetserver_hook)?;

    Ok(())
}

pub fn deinit_hooks() -> anyhow::Result<()> {
    crochet::disable!(connect_hook)?;
    crochet::disable!(openrequest_hook)?;
    crochet::disable!(gettargetserver_unicode_hook)?;
    crochet::disable!(gettargetserver_hook)?;

    Ok(())
}
