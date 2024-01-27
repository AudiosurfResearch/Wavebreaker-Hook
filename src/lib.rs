use std::ffi::c_void;
use std::os::windows::raw::HANDLE;

use tracing::info;

unsafe fn main() -> anyhow::Result<()> {
    Ok(())
}

#[no_mangle]
#[allow(non_snake_case, unused_variables, unreachable_patterns)]
unsafe extern "system" fn DllMain(_hinst: HANDLE, reason: u32, _reserved: *mut c_void) -> bool {
    match reason {
        DLL_PROCESS_ATTACH => {
            let file_appender = tracing_appender::rolling::daily("./logs", "wavebreaker_client.log");
            tracing_subscriber::fmt().with_writer(file_appender).init();
            info!("Attaching.");
            unsafe { main().unwrap() }
        }
        DLL_PROCESS_DETACH => {
            info!("Detaching.");
        }
        DLL_THREAD_ATTACH => {}
        DLL_THREAD_DETACH => {}
        _ => {}
    };
    true
}
