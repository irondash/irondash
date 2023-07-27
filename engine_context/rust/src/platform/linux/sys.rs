#[allow(non_camel_case_types)]
pub mod glib {
    use std::ffi::{c_int, c_void};

    pub type gboolean = c_int;
    pub type gpointer = *mut c_void;
    pub type GSourceFunc = Option<unsafe extern "C" fn(gpointer) -> gboolean>;
    pub type GDestroyNotify = Option<unsafe extern "C" fn(gpointer)>;

    pub const GFALSE: c_int = 0;
    pub const G_SOURCE_REMOVE: gboolean = GFALSE;

    #[repr(C)]
    pub struct GMainContext(c_void);

    #[link(name = "glib-2.0")]
    extern "C" {
        pub fn g_main_context_invoke_full(
            context: *mut GMainContext,
            priority: c_int,
            function: GSourceFunc,
            data: gpointer,
            notify: GDestroyNotify,
        );
        pub fn g_main_context_default() -> *mut GMainContext;
    }
}
