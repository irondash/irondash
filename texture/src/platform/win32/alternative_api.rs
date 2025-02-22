use std::{mem::ManuallyDrop, ops::Deref, sync::{Arc, Mutex}};

use irondash_engine_context::EngineContext;

use crate::{ID3D11Texture2D, PixelFormat, TextureDescriptor};

use super::{sys::*, PayloadHolder};


pub trait TextureDescriptorProvider<HandleType> {
    fn get_current_texture(&self) -> Box<TextureDescriptor<HandleType>>;
}


pub fn create_texture_info<T>(texture_provider: Arc<dyn TextureDescriptorProvider<T>>) -> FlutterDesktopTextureInfo 
{

    let wrapper: = Box::new(Box::new(Arc::into_raw(texture_provider)));

    let box_raw = Box::into_raw(wrapper);
    FlutterDesktopTextureInfo {
        type_: FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
        __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
            gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig {
                struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceTextureConfig>(),
                type_: FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeD3d11Texture2D,
                callback: Some(d3d11texture2d_callback),
                user_data: box_raw as  *mut std::ffi::c_void,
            },
        },
    }
}

unsafe extern "C" fn release_payload_holder<Type, FlutterType>(
    user_data: *mut ::std::os::raw::c_void,
) {
    let _user_data: Box<PayloadHolder<Type, FlutterType>> = Box::from_raw(user_data as *mut _);
}




unsafe extern "C" fn d3d11texture2d_callback(
    _width: usize,
    _height: usize,
    user_data: *mut std::os::raw::c_void,
) -> *const FlutterDesktopGpuSurfaceDescriptor {
    

    let texture_provider: Box<Box<Arc<dyn TextureDescriptorProvider<ID3D11Texture2D>>>> = Box::from_raw(user_data as *mut _);

    let texture_provider = (*texture_provider).deref();

    let texture2d = texture_provider.get_current_texture();

    let holder = Box::new(PayloadHolder {
        flutter_payload: FlutterDesktopGpuSurfaceDescriptor {
            struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceDescriptor>(),
            handle: texture2d.handle.0,
            width: texture2d.width as usize,
            height: texture2d.height as usize,
            visible_width: texture2d.visible_width as usize,
            visible_height: texture2d.visible_height as usize,
            format: match texture2d.pixel_format {
                PixelFormat::BGRA => FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatBGRA8888,
                PixelFormat::RGBA => FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatRGBA8888,
            },
            release_callback: Some(
                release_payload_holder::<ID3D11Texture2D, FlutterDesktopPixelBuffer>,
            ),
            release_context: std::ptr::null_mut(),
        },
        _payload: texture2d,
    });
    // make sure not to leak the holder
    let holder = Box::into_raw(holder);
    let holder_deref = &mut *holder;
    holder_deref.flutter_payload.release_context = holder as *mut _;
    let flutter_descriptor = &mut holder_deref.flutter_payload;
    flutter_descriptor as *mut _
}



pub fn destroy_texture<T, F>(texture_id: i64, engine_handle: i64,  user_data: Box<T>) -> crate::Result<()> 
where  F: FnOnce(*mut T)
{
    
    extern "C" fn release_callback_impl<T>(user_data: *mut T) {
        let _ = unsafe { Box::from_raw(user_data) };
    }

    let registrar = EngineContext::get()?.get_texture_registry(engine_handle)?;
        unsafe {
            (Functions::get().UnregisterExternalTexture)(
                registrar as *mut _,
                texture_id,
                Some(release_callback_impl),
                &user_data as *const _ as *mut _,
            )
        }
        Ok(())
}