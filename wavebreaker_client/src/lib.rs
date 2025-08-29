#![allow(unsafe_op_in_unsafe_fn)] //Don't want to be warned for unsafe operations in unsafe functions

mod config;
mod hooking;
mod q3d_bindings;
mod state;

use std::{
    ffi::{CString, c_void},
    path::Path,
    thread,
    time::Duration,
};

use anyhow::Context;
use bass_sys::{BASS_ErrorGetCode, BASS_PluginLoad};
use config::Config;
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use tracing::{debug, error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use windows::{
    Win32::{
        Foundation::{HMODULE, HWND, TRUE},
        System::{
            LibraryLoader::{
                DisableThreadLibraryCalls, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                GET_MODULE_HANDLE_EX_FLAG_PIN, GetModuleHandleA, GetModuleHandleExW,
            },
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
        UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxA},
    },
    core::{BOOL, PCSTR, PCWSTR, s},
};

use crate::hooking::{deinit_hooks, init_hooks};

async unsafe fn main() -> anyhow::Result<()> {
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

    if config::CONFIG.get().unwrap().main.auto_update {
        info!("Checking for updates");
        let octocrab = octocrab::instance();
        let repo = octocrab.repos("AudiosurfResearch", "Wavebreaker-Hook");
        let release = repo
            .releases()
            .get_latest()
            .await
            .context("Failed to get releases from repo")?;
        let tag_version =
            semver::Version::parse(&release.tag_name).context("Failed to parse tag")?;
        let current_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
        info!(
            "Current version {}, latest is tag_version {}",
            current_version, tag_version
        );
        if Path::exists(Path::new("./wavebreaker_up.exenew")) {
            debug!("Attempting to replace old updater executable");
            std::thread::sleep(Duration::from_millis(100));
            std::fs::rename("wavebreaker_up.exenew", "wavebreaker_up.exe")
                .context("Failed to replace old updater with new one")?;
        }

        if tag_version > current_version {
            info!("New version {} available!", tag_version);
            open::that_detached("./wavebreaker_up.exe").context("Failed to open updater")?;
            std::process::exit(0); //just kill the game
        }
    }

    unsafe {
        while GetModuleHandleA(s!("17C5B19F-4273-423C-A158-CA6F73046D43.dll")).is_err()
            || GetModuleHandleA(s!("HTTP_Fetch_Unicode.dll")).is_err()
            || GetModuleHandleA(s!("bass.dll")).is_err()
            || GetModuleHandleA(s!("BASS_PreCalcSong.dll")).is_err()
            || GetModuleHandleA(s!("GetFileAttributes.dll")).is_err()
            || GetModuleHandleA(s!("comdlg32.dll")).is_err()
            || GetModuleHandleA(s!("FolderExploder.dll")).is_err()
        {
            thread::sleep(std::time::Duration::from_millis(150));
        }
        info!("Necessary DLLs loaded, attaching hooks");
        init_hooks()?;
    }

    info!("Loading Opus BASS plugin");
    let opus_plugin_name = CString::new("bassopus").unwrap();
    let plugin_handle: *const () =
        BASS_PluginLoad(opus_plugin_name.as_ptr().cast(), 0) as *const ();
    debug!("BASSOPUS handle: {:p}", plugin_handle);
    debug!("BASS error code: {:?}", BASS_ErrorGetCode());

    loop {
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[unsafe(no_mangle)]
#[allow(non_snake_case, unused_variables, unreachable_patterns)]
unsafe extern "system" fn DllMain(hinst: HMODULE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        unsafe {
            let _ = DisableThreadLibraryCalls(hinst);

            // Bump the reference count so we don't get unloaded
            let mut handle = HMODULE(std::ptr::null_mut());
            let _ = GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_PIN | GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                PCWSTR::from_raw(DllMain as *const () as *const u16),
                &mut handle as *mut HMODULE,
            );
        }

        // TODO: Properly clean up on detach!
        thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

            rt.block_on(async {
                unsafe {
                    match main().await {
                        Ok(_) => (),
                        Err(e) => {
                            error!("{:?}", e);
                            let error_cstr = CString::new(format!(
                                "{:?}\r\nThe client will be unloaded.\r\nPlease report this issue!",
                                e
                            ))
                            .unwrap();
                            let error_pcstr =
                                PCSTR::from_raw(error_cstr.as_bytes_with_nul().as_ptr());

                            MessageBoxA(
                                Some(HWND(std::ptr::null_mut())),
                                error_pcstr,
                                s!("Wavebreaker client fatal error"),
                                MB_OK | MB_ICONERROR,
                            );
                        }
                    };
                }
            });
        });
    }

    if reason == DLL_PROCESS_DETACH {
        info!("Detaching.");
        deinit_hooks().unwrap();
    }

    TRUE
}
