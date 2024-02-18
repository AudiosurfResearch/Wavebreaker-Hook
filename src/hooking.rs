use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::OsString;
use std::os::windows::prelude::*;

use tracing::debug;
use windows::core::PCSTR;
use windows::Win32::Networking::WinInet::INTERNET_FLAG_RELOAD;
use windows::Win32::Networking::WinInet::INTERNET_FLAG_SECURE;

use crate::config::CONFIG;

#[crochet::hook("bass.dll", "BASS_StreamCreateFile")]
unsafe fn songfilestream_hook(
    mem: bool,
    file: *const c_void,
    offset: u64,
    length: u64,
    flags: u32,
) -> *mut c_void {
    if mem {
        debug!(
            "songfilestream_hook called with mem: {:?} {:?} {:?}",
            file, offset, length
        );
    } else {
        // file is a pointer to a string
        // WARNING: IT'S IN UTF-16
        let file = u16_ptr_to_string(file as *const u16);
        let file = file.to_string_lossy();

        debug!(
            "songfilestream_hook called with file: {:?} {:?} {:?}",
            file,
            offset,
            length
        );
    }

    call_original!(mem, file, offset, length, flags)
}

#[crochet::hook(compile_check, "Wininet.dll", "HttpSendRequestA")]
unsafe fn send_hook(
    hrequest: c_int,
    headers: PCSTR,
    headers_len: u32,
    optional: *mut c_void,
    optional_len: u32,
) -> c_int {
    //with length and pointer, read the optional data
    let data = std::slice::from_raw_parts(optional as *const u8, optional_len as usize);
    let data = std::str::from_utf8(data).unwrap();
    debug!(
        "send_hook called: {:?} {:?}",
        CString::from_vec_unchecked(headers.as_bytes().to_vec()),
        data
    );

    call_original!(hrequest, headers, headers_len, optional, optional_len)
}

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
    debug!(
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
    debug!(
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
    debug!("new OpenRequest flags: {:?}", flags);

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
    debug!("gettargetserver_unicode_hook called: {:?}", this_ptr);

    let orig_result = call_original!(this_ptr);
    let c_str = CStr::from_ptr(orig_result);
    debug!(
        "gettargetserver_unicode_hook original result: {:?}",
        c_str.to_str()
    );

    let new_str = rewrite_server(c_str.to_str().unwrap());
    debug!("Server rewritten to: {:?}", &new_str);

    malloc_c_string(&new_str) as *const c_char
}

#[crochet::hook(
    "17C5B19F-4273-423C-A158-CA6F73046D43.dll",
    "?GetTargetServer@Aco_HTTP_Fetch@@UAEPADXZ"
)]
unsafe extern "thiscall" fn gettargetserver_hook(this_ptr: c_int) -> *const c_char {
    debug!("gettargetserver_hook called: {:?}", this_ptr);

    let orig_result = call_original!(this_ptr);
    let c_str = CStr::from_ptr(orig_result);
    debug!("gettargetserver_hook original result: {:?}", c_str.to_str());

    let new_str = rewrite_server(c_str.to_str().unwrap());
    debug!("Server rewritten to: {:?}", &new_str);

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

unsafe fn u16_ptr_to_string(ptr: *const u16) -> OsString {
    let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(ptr, len);

    OsString::from_wide(slice)
}

pub fn init_hooks() -> anyhow::Result<()> {
    crochet::enable!(connect_hook)?;
    crochet::enable!(openrequest_hook)?;
    crochet::enable!(gettargetserver_unicode_hook)?;
    crochet::enable!(gettargetserver_hook)?;
    crochet::enable!(send_hook)?;
    crochet::enable!(songfilestream_hook)?;

    Ok(())
}

pub fn deinit_hooks() -> anyhow::Result<()> {
    crochet::disable!(connect_hook)?;
    crochet::disable!(openrequest_hook)?;
    crochet::disable!(gettargetserver_unicode_hook)?;
    crochet::disable!(gettargetserver_hook)?;
    crochet::disable!(send_hook)?;
    crochet::disable!(songfilestream_hook)?;

    Ok(())
}
