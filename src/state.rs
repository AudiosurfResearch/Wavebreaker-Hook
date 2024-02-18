use std::sync::Mutex;

pub struct GlobalData {
    pub current_mbid: Option<String>,
}

pub static GLOBAL_DATA: Mutex<GlobalData> = Mutex::new(GlobalData { current_mbid: None });
