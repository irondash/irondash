#[allow(non_camel_case_types)]
pub mod glib {
    use std::os::raw::{c_int, c_uint, c_void};
    pub type gboolean = c_int;
    pub type gpointer = *mut c_void;
    pub type GSourceFunc = Option<unsafe extern "C" fn(gpointer) -> gboolean>;
    pub type GDestroyNotify = Option<unsafe extern "C" fn(gpointer)>;
    pub const GFALSE: c_int = 0;
    pub const GTRUE: c_int = 1;
    pub const G_SOURCE_REMOVE: gboolean = GFALSE;

    #[repr(C)]
    pub struct GSource(c_void);

    #[repr(C)]
    pub struct GMainContext(c_void);

    #[repr(C)]
    pub struct GMainLoop(c_void);

    #[link(name = "glib-2.0")]
    extern "C" {
        pub fn g_main_loop_new(context: *mut GMainContext, is_running: gboolean) -> *mut GMainLoop;
        pub fn g_main_loop_unref(loop_: *mut GMainLoop);
        pub fn g_main_loop_run(loop_: *mut GMainLoop);
        pub fn g_main_loop_quit(loop_: *mut GMainLoop);
        pub fn g_main_context_push_thread_default(context: *mut GMainContext);
        pub fn g_main_context_pop_thread_default(context: *mut GMainContext);

        pub fn g_timeout_source_new(interval: c_uint) -> *mut GSource;
        pub fn g_source_set_callback(
            source: *mut GSource,
            func: GSourceFunc,
            data: gpointer,
            notify: GDestroyNotify,
        );
        pub fn g_source_attach(source: *mut GSource, context: *mut GMainContext) -> c_uint;
        pub fn g_source_unref(source: *mut GSource);
        pub fn g_source_destroy(source: *mut GSource);
        pub fn g_main_context_find_source_by_id(
            context: *mut GMainContext,
            source_id: c_uint,
        ) -> *mut GSource;
        pub fn g_main_context_ref(context: *mut GMainContext) -> *mut GMainContext;
        pub fn g_main_context_unref(context: *mut GMainContext);
        pub fn g_main_context_invoke_full(
            context: *mut GMainContext,
            priority: c_int,
            function: GSourceFunc,
            data: gpointer,
            notify: GDestroyNotify,
        );
        pub fn g_main_context_default() -> *mut GMainContext;
        pub fn g_main_context_new() -> *mut GMainContext;
        pub fn g_main_context_get_thread_default() -> *mut GMainContext;
        pub fn g_main_context_is_owner(context: *mut GMainContext) -> gboolean;
    }
    #[link(name = "gtk-3")]
    extern "C" {
        pub fn gtk_main();
        pub fn gtk_main_iteration();
        pub fn gtk_main_quit();
    }
}

#[allow(non_camel_case_types)]
pub mod libc {
    extern "C" {
        pub fn pthread_self() -> usize;
    }
}
