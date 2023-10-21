use alloc::vec::Vec;
use log::info;
pub struct Loader;
extern "C" {
    fn _num_app();
}

impl Loader {
    pub fn nth_app_data(app_id: usize) -> &'static [u8] {
        extern "C" {
            fn _num_app();
        }
        let num_app_ptr = _num_app as usize as *const usize;
        let num_app = Self::get_num_app();
        let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
        assert!(app_id < num_app);
        unsafe {
            core::slice::from_raw_parts(
                app_start[app_id] as *const u8,
                app_start[app_id + 1] - app_start[app_id],
            )
        }
    }
    pub fn get_num_app() -> usize {
        unsafe { (_num_app as usize as *const usize).read_volatile() }
    }

    pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
        info!("[loader] load {}", name);
        let num_app = Self::get_num_app();
        (0..num_app)
            .find(|&i| APP_NAMES[i] == name)
            .map(|i| Self::nth_app_data(i))
    }

    pub fn list_apps() {
        info!("[loader] available apps:");
        for (idx, &app) in APP_NAMES.iter().enumerate() {
            info!("{idx}: {app}");
        }
        info!("[loader] total {} apps", APP_NAMES.len());
    }
}

lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = Loader::get_num_app();
        extern "C" {
            fn _app_names();
        }
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != b'\0' {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let str = core::str::from_utf8_unchecked(slice);
                v.push(str);
                start = end.add(1);
            }
        }
        v
    };
}
