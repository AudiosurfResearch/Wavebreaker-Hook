mod config;
mod hooking;

use config::Config;
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use std::ffi::c_void;
use std::thread;
use tracing::info;
use windows::{
    core::{s, PCWSTR},
    Win32::{
        Foundation::{BOOL, FALSE, HMODULE, HWND, TRUE},
        System::SystemServices::DLL_PROCESS_ATTACH,
        System::{
            LibraryLoader::{
                DisableThreadLibraryCalls, GetModuleHandleA, GetModuleHandleExW,
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_PIN,
            },
            SystemServices::DLL_PROCESS_DETACH,
        },
        UI::WindowsAndMessaging::{MessageBoxA, MB_ICONERROR, MB_ICONINFORMATION, MB_OK},
    },
};

unsafe fn main() -> anyhow::Result<()> {
    MessageBoxA(
        HWND(0),
        s!("Thread running!"),
        s!("Testing"),
        MB_OK | MB_ICONERROR,
    );

    let file_appender = tracing_appender::rolling::daily("./logs", "wavebreaker_client.log");
    tracing_subscriber::fmt().with_writer(file_appender).init();
    info!("Attached!");

    info!("Initializing...");

    let config: Config = Figment::new()
        .merge(Toml::file("Wavebreaker.toml"))
        .merge(Env::prefixed("WAVEBREAKER_"))
        .extract()?;
    let _ = config::CONFIG.set(config);

    info!("{:?}", GetModuleHandleA(s!("asdfasdfasdf.dll")));

    loop {
        thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[no_mangle]
#[allow(non_snake_case, unused_variables, unreachable_patterns)]
unsafe extern "system" fn DllMain(hinst: HMODULE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        let _ = DisableThreadLibraryCalls(hinst);

        MessageBoxA(
            HWND(0),
            s!("Attaching"),
            s!("Testing"),
            MB_OK | MB_ICONINFORMATION,
        );

        //Bump the reference count so we don't get unloaded
        let mut handle = HMODULE(0);
        let _ = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_PIN | GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            PCWSTR::from_raw(DllMain as *const () as *const u16),
            &mut handle as *mut HMODULE,
        );
        // TODO: Use WinAPI thread functions directly!
        thread::spawn(|| main);

        return TRUE;
    }

    if reason == DLL_PROCESS_DETACH {
        MessageBoxA(
            HWND(0),
            s!("Detaching!"),
            s!("Testing"),
            MB_OK | MB_ICONERROR,
        );
        return TRUE;
    }

    FALSE
}
