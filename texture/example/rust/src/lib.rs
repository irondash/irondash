use std::{
    mem::{forget, ManuallyDrop},
    ptr::slice_from_raw_parts_mut,
    rc::Rc,
    slice,
    time::Duration,
};

use irondash_engine_context::EngineContext;
use irondash_jni_context::JniContext;
use irondash_run_loop::RunLoop;
use jni::objects::JObject;
use log::{info, warn};
use ndk_sys::{
    AHardwareBuffer_Format, ANativeWindow, ANativeWindow_Buffer, ANativeWindow_fromSurface,
    ANativeWindow_lock, ANativeWindow_release, ANativeWindow_setBuffersGeometry,
    ANativeWindow_unlockAndPost, ASurfaceTexture_fromSurfaceTexture,
};
use rand::Rng;

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Debug)
            .with_tag("flutter"),
    );
}

struct Texturer {
    win: *mut ANativeWindow,
}

impl Texturer {
    fn new(win: *mut ANativeWindow) -> Self {
        unsafe {
            ANativeWindow_setBuffersGeometry(
                win,
                100,
                100,
                AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM.0 as i32,
            );
        }
        Self { win }
    }

    fn frame(self: &Rc<Self>) {
        unsafe {
            let mut buf: ANativeWindow_Buffer = std::mem::zeroed();
            ANativeWindow_lock(self.win, &mut buf as *mut _, std::ptr::null_mut());
            let data = slice::from_raw_parts_mut(
                buf.bits as *mut u8,
                (buf.height * buf.stride * 4) as usize,
            );
            let mut rand = rand::thread_rng();
            for i in data {
                *i = rand.gen();
            }
            ANativeWindow_unlockAndPost(self.win);
        }
        let self_clone = self.clone();
        RunLoop::current()
            .schedule(Duration::from_millis(100), move || {
                self_clone.frame();
            })
            .detach();
    }
}

fn init_on_main_thread(engine_id: i64) -> jni::errors::Result<i64> {
    let context = match EngineContext::get() {
        Ok(context) => context,
        Err(err) => {
            warn!("{:?}", err);
            panic!("BYE");
        }
    };
    let texture_registry = context.get_texture_registry(engine_id);
    let texture_registry = match texture_registry {
        Ok(registry) => registry,
        Err(err) => {
            warn!("{}", err);
            panic!("BYE");
        }
    };
    let java_vm = JniContext::get().unwrap().java_vm();
    let env = java_vm.attach_current_thread()?;
    let texture_entry = env
        .call_method(
            texture_registry.as_obj(),
            "createSurfaceTexture",
            "()Lio/flutter/view/TextureRegistry$SurfaceTextureEntry;",
            &[],
        )?
        .l()?;
    let surface_texture = env
        .call_method(
            texture_entry,
            "surfaceTexture",
            "()Landroid/graphics/SurfaceTexture;",
            &[],
        )?
        .l()?;

    let surface_class = env.find_class("android/view/Surface")?;

    env.push_local_frame(16)?;

    let surface = env.new_object(
        surface_class,
        "(Landroid/graphics/SurfaceTexture;)V",
        &[surface_texture.into()],
    )?;

    let native_window =
        unsafe { ANativeWindow_fromSurface(env.get_native_interface(), surface.into_inner()) };

    let surface = env.new_global_ref(surface)?;
    forget(surface);

    env.pop_local_frame(JObject::null())?;

    info!("NativeWindow {:?}", native_window);
    let texturer = ManuallyDrop::new(Rc::new(Texturer::new(native_window)));
    texturer.frame();

    let id = env.call_method(texture_entry, "id", "()J", &[])?.j()?;

    let e = env.new_global_ref(texture_entry)?;
    forget(e);

    Ok(id)
}

#[no_mangle]
pub extern "C" fn init_texture_example(engine_id: i64) -> i64 {
    init_logging();
    let runner = RunLoop::sender_for_main_thread();
    runner.send_and_wait(move || match init_on_main_thread(engine_id) {
        Ok(res) => res,
        Err(e) => {
            warn!("JniError: {:?}", e);
            0
        }
    })
}
