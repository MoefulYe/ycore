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
        info!("[loader] load {}th app", app_id);
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
}
