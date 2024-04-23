use std::sync::Mutex;

pub struct GlobalData {
    pub current_mbid: Option<String>,
    pub current_release_mbid: Option<String>,
    pub ticket: Option<String>,
}

pub static GLOBAL_DATA: Mutex<GlobalData> = Mutex::new(GlobalData {
    current_mbid: None,
    ticket: None,
    current_release_mbid: None,
});
