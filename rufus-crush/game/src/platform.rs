//! The handful of browser things macroquad doesn't cover, supplied by
//! `web/rufus.js`. Native builds get harmless stand-ins.

#[cfg(target_arch = "wasm32")]
mod imp {
    extern "C" {
        fn rufus_save(ptr: *const u8, len: u32);
        fn rufus_load(ptr: *mut u8, cap: u32) -> u32;
        fn rufus_query(ptr: *mut u8, cap: u32) -> u32;
        fn rufus_vibrate(ms: u32);
        fn rufus_inset(side: u32) -> f32;
        fn rufus_now() -> f64;
        fn rufus_ready();
    }

    /// Lets the JS loader check it is talking to the right build.
    #[no_mangle]
    pub extern "C" fn rufus_crate_version() -> u32 {
        1
    }

    fn read(f: unsafe extern "C" fn(*mut u8, u32) -> u32) -> String {
        let mut buf = vec![0u8; 4096];
        let n = unsafe { f(buf.as_mut_ptr(), buf.len() as u32) } as usize;
        buf.truncate(n.min(4096));
        String::from_utf8_lossy(&buf).into_owned()
    }

    pub fn save(s: &str) {
        unsafe { rufus_save(s.as_ptr(), s.len() as u32) }
    }
    pub fn load() -> String {
        read(rufus_load)
    }
    pub fn query() -> String {
        read(rufus_query)
    }
    pub fn vibrate(ms: u32) {
        unsafe { rufus_vibrate(ms) }
    }
    pub fn inset(side: u32) -> f32 {
        unsafe { rufus_inset(side) }
    }
    pub fn now_ms() -> f64 {
        unsafe { rufus_now() }
    }
    pub fn ready() {
        unsafe { rufus_ready() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn save(_: &str) {}
    pub fn load() -> String {
        String::new()
    }
    pub fn query() -> String {
        std::env::args().nth(1).unwrap_or_default()
    }
    pub fn vibrate(_: u32) {}
    pub fn inset(_: u32) -> f32 {
        0.0
    }
    pub fn now_ms() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    }
    pub fn ready() {}
}

pub use imp::*;

/// `?level=7&auto=1` style switches, for testing a build without playing
/// through it. Unknown keys are ignored.
pub struct Query(Vec<(String, String)>);

impl Query {
    pub fn read() -> Query {
        let raw = query();
        Query(
            raw.trim_start_matches('?')
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once('=') {
                    Some((k, v)) => (k.to_string(), v.to_string()),
                    None => (p.to_string(), "1".to_string()),
                })
                .collect(),
        )
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    pub fn num(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(|v| v.parse().ok())
    }
    pub fn flag(&self, key: &str) -> bool {
        self.get(key).is_some_and(|v| v != "0")
    }
}
