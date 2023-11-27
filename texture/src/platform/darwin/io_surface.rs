use std::{
    ffi::{c_int, c_void},
    slice,
};

use core_foundation::{
    base::{CFRelease, CFRetain, CFType, CFTypeRef, TCFType},
    dictionary::{CFDictionary, CFDictionaryRef},
    mach_port::CFTypeID,
    string::{CFString, CFStringRef},
};

#[repr(C)]
pub struct __IOSurface(c_void);

type IOReturn = c_int;
pub type IOSurfaceRef = *const __IOSurface;

pub struct IOSurface {
    pub obj: IOSurfaceRef,
}

unsafe impl Send for IOSurface {}

impl Drop for IOSurface {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

pub type IOSurfaceID = u32;

impl Clone for IOSurface {
    #[inline]
    fn clone(&self) -> IOSurface {
        unsafe { TCFType::wrap_under_get_rule(self.obj) }
    }
}

impl TCFType for IOSurface {
    type Ref = IOSurfaceRef;

    #[inline]
    #[allow(non_snake_case)]
    fn as_concrete_TypeRef(&self) -> IOSurfaceRef {
        self.obj
    }

    #[inline]
    unsafe fn wrap_under_create_rule(obj: IOSurfaceRef) -> IOSurface {
        assert!(!obj.is_null(), "Attempted to create a NULL object.");
        IOSurface { obj }
    }

    #[inline]
    fn type_id() -> CFTypeID {
        unsafe { IOSurfaceGetTypeID() }
    }

    #[inline]
    #[allow(non_snake_case)]
    fn as_CFTypeRef(&self) -> CFTypeRef {
        self.as_concrete_TypeRef() as CFTypeRef
    }

    #[inline]
    unsafe fn wrap_under_get_rule(reference: IOSurfaceRef) -> IOSurface {
        assert!(!reference.is_null(), "Attempted to create a NULL object.");
        let reference = CFRetain(reference as *const c_void) as IOSurfaceRef;
        TCFType::wrap_under_create_rule(reference)
    }
}

impl IOSurface {
    pub fn new(properties: &CFDictionary<CFString, CFType>) -> IOSurface {
        unsafe {
            TCFType::wrap_under_create_rule(IOSurfaceCreate(properties.as_concrete_TypeRef()))
        }
    }

    pub fn upload(&self, data: &[u8]) {
        unsafe {
            let surface = self.as_concrete_TypeRef();
            let mut seed = 0;

            IOSurfaceLock(surface, 0, &mut seed);

            let height = IOSurfaceGetHeight(surface);
            let stride = IOSurfaceGetBytesPerRow(surface);
            let size = height * stride;
            let address = IOSurfaceGetBaseAddress(surface) as *mut u8;
            let dest: &mut [u8] = slice::from_raw_parts_mut(address, size);
            dest.clone_from_slice(data);

            // FIXME(pcwalton): RAII
            IOSurfaceUnlock(surface, 0, &mut seed);
        }
    }
}

#[link(name = "IOSurface", kind = "framework")]
extern "C" {
    pub static kIOSurfaceAllocSize: CFStringRef;
    pub static kIOSurfaceWidth: CFStringRef;
    pub static kIOSurfaceHeight: CFStringRef;
    pub static kIOSurfaceBytesPerRow: CFStringRef;
    pub static kIOSurfaceBytesPerElement: CFStringRef;
    pub static kIOSurfaceElementWidth: CFStringRef;
    pub static kIOSurfaceElementHeight: CFStringRef;
    pub static kIOSurfaceOffset: CFStringRef;

    pub static kIOSurfacePlaneInfo: CFStringRef;
    pub static kIOSurfacePlaneWidth: CFStringRef;
    pub static kIOSurfacePlaneHeight: CFStringRef;
    pub static kIOSurfacePlaneBytesPerRow: CFStringRef;
    pub static kIOSurfacePlaneOffset: CFStringRef;
    pub static kIOSurfacePlaneSize: CFStringRef;

    pub static kIOSurfacePlaneBase: CFStringRef;
    pub static kIOSurfacePlaneBytesPerElement: CFStringRef;
    pub static kIOSurfacePlaneElementWidth: CFStringRef;
    pub static kIOSurfacePlaneElementHeight: CFStringRef;

    pub static kIOSurfaceCacheMode: CFStringRef;
    pub static kIOSurfaceIsGlobal: CFStringRef;
    pub static kIOSurfacePixelFormat: CFStringRef;

    pub fn IOSurfaceCreate(properties: CFDictionaryRef) -> IOSurfaceRef;
    pub fn IOSurfaceLookup(csid: IOSurfaceID) -> IOSurfaceRef;
    pub fn IOSurfaceGetID(buffer: IOSurfaceRef) -> IOSurfaceID;

    pub fn IOSurfaceGetTypeID() -> CFTypeID;

    pub fn IOSurfaceLock(buffer: IOSurfaceRef, options: u32, seed: *mut u32) -> IOReturn;
    pub fn IOSurfaceUnlock(buffer: IOSurfaceRef, options: u32, seed: *mut u32) -> IOReturn;

    pub fn IOSurfaceGetHeight(buffer: IOSurfaceRef) -> usize;
    pub fn IOSurfaceGetWidth(buffer: IOSurfaceRef) -> usize;
    pub fn IOSurfaceGetBytesPerRow(buffer: IOSurfaceRef) -> usize;
    pub fn IOSurfaceGetBaseAddress(buffer: IOSurfaceRef) -> *mut c_void;
}
