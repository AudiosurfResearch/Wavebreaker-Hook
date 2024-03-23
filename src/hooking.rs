use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    mem,
    path::Path,
};

use lofty::{ItemKey, ItemValue, TaggedFileExt};
use tracing::{debug, error, info, trace};
use url_encoded_data::UrlEncodedData;
use windows::{
    core::PCSTR,
    Win32::Networking::WinInet::{
        InternetQueryOptionA, INTERNET_FLAG_RELOAD, INTERNET_FLAG_SECURE, INTERNET_OPTION_URL,
    },
};

use crate::{
    config::CONFIG,
    q3d_bindings::{A3d_Channel, Aco_FloatChannel, Aco_StringChannel_GetString},
    state,
};

#[crochet::hook("BASS_PreCalcSong.dll", "?CallChannel@Aco_BASS_PreCalcSong@@UAEXXZ")]
unsafe extern "thiscall" fn precalcsong_call_hook(this: *mut A3d_Channel) {
    let channel = this.as_mut().unwrap();
    let song_source = channel
        .GetChild(1)
        .cast::<Aco_FloatChannel>()
        .as_mut()
        .unwrap()
        .channelFloat_;

    debug!("PreCalc with source {}", song_source);

    // 0 = File
    // 1 = CD
    // 2 = Buffer
    if song_source != 0.0 {
        let mut global_data = state::GLOBAL_DATA.lock().unwrap();
        global_data.current_mbid = None;
        global_data.current_release_mbid = None;
        return call_original!(this);
    }

    let song_path = CStr::from_ptr(Aco_StringChannel_GetString(channel.GetChild(4).cast()))
        .to_str()
        .unwrap();
    let song_path = Path::new(&song_path);

    debug!("PreCalc from file path: {}", song_path.display());

    let tagged_file = match lofty::read_from_path(song_path) {
        Ok(res) => res,
        Err(e) => {
            error!("lofty::read_from_path failed {:?}", e);
            return call_original!(this);
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

    call_original!(this);
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
        trace!(
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
    trace!(
        "send_hook called: {:?} {:?}",
        CString::from_vec_unchecked(headers.as_bytes().to_vec()),
        data
    );
    let form_data = UrlEncodedData::parse_str(data);
    let mut new_form_data = form_data.clone();

    let mut global_data = state::GLOBAL_DATA.lock().unwrap();

    // store ticket for our own uses
    if form_data.exists("ticket") && global_data.ticket.is_none() {
        let ticket = form_data.get_first("ticket").unwrap();
        global_data.ticket = Some(ticket.to_string());
        debug!("Ticket found in data: {:?}", ticket);
    }

    // Add client version when:
    // - fetching song ID
    // - attempting to login
    // - fetching custom news
    // - submitting a score
    if url.ends_with("/as_steamlogin/game_fetchsongid_unicode.php")
        || url.ends_with("/as_steamlogin/game_AttemptLoginSteamVerified.php")
        || url.ends_with("//as_steamlogin/game_CustomNews.php")
        || url.ends_with("/as_steamlogin/game_SendRideSteamVerified.php")
    {
        new_form_data.set_one("wvbrclientversion", env!("CARGO_PKG_VERSION"));
    }

    // Add Steam auth ticket when fetching song ID
    if url.ends_with("/as_steamlogin/game_fetchsongid_unicode.php") && global_data.ticket.is_some()
    {
        new_form_data.set_one("ticket", global_data.ticket.as_ref().unwrap());
    }

    // Add recording and release MBIDs (if present), when fetching song ID and submitting a score
    if url.ends_with("/as_steamlogin/game_fetchsongid_unicode.php")
        || url.ends_with("/as_steamlogin/game_SendRideSteamVerified.php")
    {
        if global_data.current_mbid.is_some() {
            new_form_data.set_one("mbid", global_data.current_mbid.as_ref().unwrap());
        }
        if global_data.current_release_mbid.is_some() {
            new_form_data.set_one(
                "releasembid",
                global_data.current_release_mbid.as_ref().unwrap(),
            );
        }
    }

    // if nothing changed, we don't need to allocate a new string
    if new_form_data.to_string_of_original_order() == form_data.to_string_of_original_order() {
        call_original!(hrequest, headers, headers_len, optional, optional_len)
    } else {
        let new_data_string = new_form_data.to_string_of_original_order();
        let new_data = malloc_c_string(&new_data_string) as *mut c_void; // allocate new string

        call_original!(
            hrequest,
            headers,
            headers_len,
            new_data,
            new_data_string.len() as u32
        )
    }
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
    debug!("Server rewritten to: {:?}", &new_str);

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
    trace!("gettargetserver_hook original result: {:?}", c_str.to_str());

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

pub fn init_hooks() -> anyhow::Result<()> {
    crochet::enable!(connect_hook)?;
    crochet::enable!(openrequest_hook)?;
    crochet::enable!(gettargetserver_unicode_hook)?;
    crochet::enable!(gettargetserver_hook)?;
    crochet::enable!(send_hook)?;
    crochet::enable!(precalcsong_call_hook)?;

    Ok(())
}

pub fn deinit_hooks() -> anyhow::Result<()> {
    crochet::disable!(connect_hook)?;
    crochet::disable!(openrequest_hook)?;
    crochet::disable!(gettargetserver_unicode_hook)?;
    crochet::disable!(gettargetserver_hook)?;
    crochet::disable!(send_hook)?;
    crochet::disable!(precalcsong_call_hook)?;

    Ok(())
}
