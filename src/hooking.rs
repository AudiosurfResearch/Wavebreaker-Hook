use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::OsString;
use std::mem;
use std::os::windows::prelude::*;
use std::path::Path;

use lofty::ItemKey;
use lofty::ItemValue;
use lofty::TaggedFileExt;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::trace;
use url_encoded_data::UrlEncodedData;
use windows::core::PCSTR;
use windows::Win32::Networking::WinInet::InternetQueryOptionA;
use windows::Win32::Networking::WinInet::INTERNET_FLAG_RELOAD;
use windows::Win32::Networking::WinInet::INTERNET_FLAG_SECURE;
use windows::Win32::Networking::WinInet::INTERNET_OPTION_URL;

use crate::config::CONFIG;
use crate::state;

#[crochet::hook("bass.dll", "BASS_StreamCreateFile")]
unsafe fn songfilestream_hook(
    mem: bool,
    file: *const c_void,
    offset: u64,
    length: u64,
    flags: u32,
) -> *mut c_void {
    if mem {
        trace!(
            "songfilestream_hook called on memory: {:?} {:?} {:?}",
            file,
            offset,
            length
        );
    } else {
        // file is a pointer to a string
        // WARNING: IT'S IN UTF-16
        let file_path = u16_ptr_to_string(file as *const u16);
        let file_path = Path::new(&file_path);

        debug!(
            "songfilestream_hook called with file: {:?} {:?} {:?}",
            file_path, offset, length
        );

        if file_path.is_absolute() {
            let tagged_file = match lofty::read_from_path(file_path) {
                Ok(res) => res,
                Err(e) => {
                    error!("lofty::read_from_path failed {:?}", e);
                    return call_original!(mem, file, offset, length, flags);
                }
            };

            let tag = tagged_file.primary_tag();
            match tag {
                Some(tag) => {
                    match tag.get(&ItemKey::MusicBrainzRecordingId) {
                        Some(item) => match item.value() {
                            ItemValue::Text(mbid) => {
                                let mut global_data = state::GLOBAL_DATA.lock().unwrap();
                                global_data.current_mbid = Some(mbid.to_string());
                                info!("Recording MBID tag found: {:?}", mbid);
                            }
                            _ => {
                                error!("Recording MBID tag is an invalid data type...?");
                            }
                        },
                        _ => {
                            debug!("File has no recording MBID");
                        }
                    };

                    match tag.get(&ItemKey::MusicBrainzReleaseId) {
                        Some(item) => match item.value() {
                            ItemValue::Text(mbid) => {
                                let mut global_data = state::GLOBAL_DATA.lock().unwrap();
                                global_data.current_release_mbid = Some(mbid.to_string());
                                info!("Release MBID tag found: {:?}", mbid);
                            }
                            _ => {
                                error!("Release MBID tag is an invalid data type...?");
                            }
                        },
                        _ => {
                            debug!("File has no release MBID");
                        }
                    };
                }
                None => {
                    debug!("File has no tags");
                }
            }
        }
    }

    call_original!(mem, file, offset, length, flags)
}

#[crochet::hook(compile_check, "Wininet.dll", "HttpSendRequestA")]
unsafe fn send_hook(
    hrequest: *const c_void,
    headers: PCSTR,
    headers_len: u32,
    optional: *mut c_void,
    optional_len: u32,
) -> c_int {
    if optional.is_null() || optional_len == 0 {
        debug!(
            "send_hook called without data {:?}",
            CString::from_vec_unchecked(headers.as_bytes().to_vec()),
        );
        return call_original!(hrequest, headers, headers_len, optional, optional_len);
    }

    // Get the URL so we know where the request is going
    let mut lpbuffer = [0u8; 1024];
    let mut lpdwbufferlength = mem::size_of_val(&lpbuffer) as u32;
    let _ = InternetQueryOptionA(
        Some(hrequest),
        INTERNET_OPTION_URL,
        Some(lpbuffer.as_mut_ptr() as *mut c_void),
        &mut lpdwbufferlength as *mut u32,
    );

    // Cut the buffer and convert to string
    let url = CString::from_vec_unchecked(lpbuffer[..lpdwbufferlength as usize].to_vec());
    let url = url.to_str().unwrap();

    //with length and pointer, read the optional data
    let data = std::slice::from_raw_parts(optional as *const u8, optional_len as usize);
    let data = std::str::from_utf8(data).unwrap();
    debug!(
        "send_hook called: {:?} {:?}",
        CString::from_vec_unchecked(headers.as_bytes().to_vec()),
        data
    );
    let mut data = UrlEncodedData::parse_str(data);

    let mut global_data = state::GLOBAL_DATA.lock().unwrap();

    // store ticket for our own uses
    if data.exists("ticket") && global_data.ticket.is_none() {
        let ticket = data.get_first("ticket").unwrap();
        global_data.ticket = Some(ticket.to_string());
        debug!("Ticket found in data: {:?}", ticket);
    }

    if url.ends_with("/as_steamlogin/game_AttemptLoginSteamVerified.php")
        || url.ends_with("//as_steamlogin/game_CustomNews.php")
    {
        data.set_one("wvbrclientversion", env!("CARGO_PKG_VERSION"));
        let new_data_string = data.to_string_of_original_order();

        // allocate new string
        let new_data = malloc_c_string(&new_data_string) as *mut c_void;
        return call_original!(
            hrequest,
            headers,
            headers_len,
            new_data,
            new_data_string.len() as u32
        );
    }

    if url.ends_with("/as_steamlogin/game_fetchsongid_unicode.php") && global_data.ticket.is_some()
    {
        data.set_one("ticket", global_data.ticket.as_ref().unwrap());
        let new_data_string = data.to_string_of_original_order();

        // allocate new string
        let new_data = malloc_c_string(&new_data_string) as *mut c_void;
        return call_original!(
            hrequest,
            headers,
            headers_len,
            new_data,
            new_data_string.len() as u32
        );
    }

    if url.ends_with("/as_steamlogin/game_SendRideSteamVerified.php")
        && global_data.current_mbid.is_some()
    {
        data.set_one("mbid", global_data.current_mbid.as_ref().unwrap());
        if global_data.current_release_mbid.is_some() {
            data.set_one(
                "releasembid",
                global_data.current_release_mbid.as_ref().unwrap(),
            );
        }
        let new_data_string = data.to_string_of_original_order();
        debug!("New score submission form data: {:?}", new_data_string);

        // allocate new string
        let new_data = malloc_c_string(&new_data_string) as *mut c_void;
        return call_original!(
            hrequest,
            headers,
            headers_len,
            new_data,
            new_data_string.len() as u32
        );
    }

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
        verb.to_string().unwrap(),
        object_name.to_string().unwrap(),
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

    // reset MBIDs when a new song is loaded
    // a bit hacky, but we have to do this so we don't submit an old ID when someone starts playing a Radio song
    // Radio mode songs are loaded via memory so trying to see what file it loads the song from won't work
    // We could just look at the song it loaded into memory, but the MBID doesn't matter for Radio songs anyway
    if object_name.to_string().unwrap() == "/as_steamlogin/game_fetchsongid_unicode.php" {
        let mut global_data = state::GLOBAL_DATA.lock().unwrap();
        global_data.current_mbid = None;
        global_data.current_release_mbid = None;
    }

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
