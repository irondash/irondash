pub(crate) struct GObjectWrapper {
    gobject: *mut gobject_sys::GObject,
}

impl GObjectWrapper {
    pub fn take_over(gobject: *mut gobject_sys::GObject) -> Self {
        Self { gobject }
    }

    pub unsafe fn retain(gobject: *mut gobject_sys::GObject) -> Self {
        Self {
            gobject: unsafe { gobject_sys::g_object_ref(gobject) },
        }
    }

    pub fn get(&self) -> *mut gobject_sys::GObject {
        self.gobject
    }
}

impl Drop for GObjectWrapper {
    fn drop(&mut self) {
        unsafe { gobject_sys::g_object_unref(self.gobject) }
    }
}

impl Clone for GObjectWrapper {
    fn clone(&self) -> Self {
        unsafe { Self::retain(self.gobject) }
    }
}
