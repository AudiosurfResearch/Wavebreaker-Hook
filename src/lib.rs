mod config;
mod hooking;
mod q3d_bindings;
mod state;

use std::{
    ffi::{c_void, CString},
    thread,
};

use config::Config;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use tracing::{error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use windows::{
    core::{s, PCSTR, PCWSTR},
    Win32::{
        Foundation::{BOOL, HMODULE, HWND, TRUE},
        System::{
            LibraryLoader::{
                DisableThreadLibraryCalls, GetModuleHandleA, GetModuleHandleExW,
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_PIN,
            },
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
        UI::WindowsAndMessaging::{MessageBoxA, MB_ICONERROR, MB_OK},
    },
};

use crate::hooking::{deinit_hooks, init_hooks};

unsafe fn main() -> anyhow::Result<()> {
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("wavebreaker_client")
        .filename_suffix("log")
        .build("./logs")?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wavebreaker_client=info".into()),
        )
        .with(fmt::layer().with_writer(file_appender))
        .init();
    info!("Initializing...");

    let config: Config = Figment::new()
        .merge(Toml::file("Wavebreaker.toml"))
        .merge(Env::prefixed("WAVEBREAKER_"))
        .extract()?;
    let _ = config::CONFIG.set(config);

    while GetModuleHandleA(s!("17C5B19F-4273-423C-A158-CA6F73046D43.dll")).is_err()
        || GetModuleHandleA(s!("HTTP_Fetch_Unicode.dll")).is_err()
        || GetModuleHandleA(s!("bass.dll")).is_err()
        || GetModuleHandleA(s!("BASS_PreCalcSong.dll")).is_err()
    {
        thread::sleep(std::time::Duration::from_millis(150));
    }
    info!("Necessary DLLs loaded, attaching hooks");
    init_hooks()?;

    loop {
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[no_mangle]
#[allow(non_snake_case, unused_variables, unreachable_patterns)]
unsafe extern "system" fn DllMain(hinst: HMODULE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        let _ = DisableThreadLibraryCalls(hinst);

        // Bump the reference count so we don't get unloaded
        let mut handle = HMODULE(0);
        let _ = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_PIN | GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            PCWSTR::from_raw(DllMain as *const () as *const u16),
            &mut handle as *mut HMODULE,
        );

        // TODO: Use WinAPI thread functions directly and properly clean up on detach!
        thread::spawn(|| {
            match main() {
                Ok(_) => (),
                Err(e) => {
                    error!("{:?}", e);
                    let error_cstr = CString::new(format!(
                        "{:?}\r\nThe client will be unloaded.\r\nPlease report this issue!",
                        e
                    ))
                    .unwrap();
                    let error_pcstr = PCSTR::from_raw(error_cstr.as_bytes_with_nul().as_ptr());

                    MessageBoxA(
                        HWND(0),
                        error_pcstr,
                        s!("Wavebreaker client fatal error"),
                        MB_OK | MB_ICONERROR,
                    );
                }
            };
        });
    }

    if reason == DLL_PROCESS_DETACH {
        info!("Detaching.");
        deinit_hooks().unwrap();
    }

    TRUE
}
