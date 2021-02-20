use std::cell::UnsafeCell;

pub struct Monitor {
    pub ptr: UnsafeCell<*mut ::capi::ETEXT_DESC>,
}

unsafe impl Sync for Monitor {}
unsafe impl Send for Monitor {}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe { ::capi::TessMonitorDelete(*self.ptr.get()) };
    }
}

impl Default for Monitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            ptr: UnsafeCell::new(unsafe { ::capi::TessMonitorCreate() }),
        }
    }

    pub fn get(&self) -> *mut ::capi::ETEXT_DESC {
        unsafe { *self.ptr.get() }
    }

    pub fn get_progress(&self) -> i32 {
        unsafe { ::capi::TessMonitorGetProgress(self.get()) }
    }
}
