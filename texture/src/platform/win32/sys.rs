#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_void;

use once_cell::sync::OnceCell;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopTextureRegistrar {
    _unused: [u8; 0],
}
pub type FlutterDesktopTextureRegistrarRef = *mut FlutterDesktopTextureRegistrar;
pub const FlutterDesktopTextureType_kFlutterDesktopPixelBufferTexture: FlutterDesktopTextureType =
    0;
pub const FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture: FlutterDesktopTextureType = 1;
pub type FlutterDesktopTextureType = ::std::os::raw::c_uint;
pub const FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeNone:
    FlutterDesktopGpuSurfaceType = 0;
pub const FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeDxgiSharedHandle:
    FlutterDesktopGpuSurfaceType = 1;
pub const FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeD3d11Texture2D:
    FlutterDesktopGpuSurfaceType = 2;
pub type FlutterDesktopGpuSurfaceType = ::std::os::raw::c_uint;
pub const FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatNone: FlutterDesktopPixelFormat = 0;
pub const FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatRGBA8888: FlutterDesktopPixelFormat =
    1;
pub const FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatBGRA8888: FlutterDesktopPixelFormat =
    2;
pub type FlutterDesktopPixelFormat = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopPixelBuffer {
    pub buffer: *const u8,
    pub width: usize,
    pub height: usize,
    pub release_callback:
        ::std::option::Option<unsafe extern "C" fn(release_context: *mut ::std::os::raw::c_void)>,
    pub release_context: *mut ::std::os::raw::c_void,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopGpuSurfaceDescriptor {
    pub struct_size: usize,
    pub handle: *mut ::std::os::raw::c_void,
    pub width: usize,
    pub height: usize,
    pub visible_width: usize,
    pub visible_height: usize,
    pub format: FlutterDesktopPixelFormat,
    pub release_callback:
        ::std::option::Option<unsafe extern "C" fn(release_context: *mut ::std::os::raw::c_void)>,
    pub release_context: *mut ::std::os::raw::c_void,
}
pub type FlutterDesktopPixelBufferTextureCallback = ::std::option::Option<
    unsafe extern "C" fn(
        width: usize,
        height: usize,
        user_data: *mut ::std::os::raw::c_void,
    ) -> *const FlutterDesktopPixelBuffer,
>;
pub type FlutterDesktopGpuSurfaceTextureCallback = ::std::option::Option<
    unsafe extern "C" fn(
        width: usize,
        height: usize,
        user_data: *mut ::std::os::raw::c_void,
    ) -> *const FlutterDesktopGpuSurfaceDescriptor,
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopPixelBufferTextureConfig {
    pub callback: FlutterDesktopPixelBufferTextureCallback,
    pub user_data: *mut ::std::os::raw::c_void,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlutterDesktopGpuSurfaceTextureConfig {
    pub struct_size: usize,
    pub type_: FlutterDesktopGpuSurfaceType,
    pub callback: FlutterDesktopGpuSurfaceTextureCallback,
    pub user_data: *mut ::std::os::raw::c_void,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FlutterDesktopTextureInfo {
    pub type_: FlutterDesktopTextureType,
    pub __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union FlutterDesktopTextureInfo__bindgen_ty_1 {
    pub pixel_buffer_config: FlutterDesktopPixelBufferTextureConfig,
    pub gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig,
}

// Can't link to Flutter engine directly.
pub struct Functions {
    pub RegisterExternalTexture: unsafe extern "C" fn(
        texture_registrar: FlutterDesktopTextureRegistrarRef,
        info: *const FlutterDesktopTextureInfo,
    ) -> i64,
    pub UnregisterExternalTexture: unsafe extern "C" fn(
        texture_registrar: FlutterDesktopTextureRegistrarRef,
        texture_id: i64,
        callback: ::std::option::Option<
            unsafe extern "C" fn(user_data: *mut ::std::os::raw::c_void),
        >,
        user_data: *mut ::std::os::raw::c_void,
    ),
    pub MarkExternalTextureFrameAvailable: unsafe extern "C" fn(
        texture_registrar: FlutterDesktopTextureRegistrarRef,
        texture_id: i64,
    ) -> bool,
}

static FUNCTIONS: OnceCell<Functions> = OnceCell::new();

type LPCSTR = *const i8;
type HINSTANCE = isize;
type HMODULE = isize;

use cstr::cstr;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleA(lpmodulename: LPCSTR) -> HINSTANCE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> *mut c_void;
}

impl Functions {
    pub fn get() -> &'static Self {
        FUNCTIONS.get_or_init(Self::new)
    }

    fn new() -> Self {
        unsafe {
            let module = GetModuleHandleA(cstr!("flutter_windows.dll").as_ptr());
            #[allow(clippy::missing_transmute_annotations)]
            Self {
                RegisterExternalTexture: std::mem::transmute(GetProcAddress(
                    module,
                    cstr!("FlutterDesktopTextureRegistrarRegisterExternalTexture").as_ptr(),
                )),
                UnregisterExternalTexture: std::mem::transmute(GetProcAddress(
                    module,
                    cstr!("FlutterDesktopTextureRegistrarUnregisterExternalTexture").as_ptr(),
                )),
                MarkExternalTextureFrameAvailable: std::mem::transmute(GetProcAddress(
                    module,
                    cstr!("FlutterDesktopTextureRegistrarMarkExternalTextureFrameAvailable")
                        .as_ptr(),
                )),
            }
        }
    }
}
