use std::{cell::Cell, ffi::c_void, iter::repeat_with, rc::Rc, sync::Arc, time::Duration};

use irondash_dart_ffi::DartValue;
use irondash_run_loop::RunLoop;
use irondash_texture::{BoxedPixelData, PayloadProvider, SimplePixelData, Texture};
use log::error;

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Debug)
            .with_tag("flutter"),
    );
}

#[cfg(target_os = "ios")]
fn init_logging() {
    oslog::OsLogger::new("texture_example")
        .level_filter(::log::LevelFilter::Debug)
        .init()
        .ok();
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn init_logging() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
}

struct Animator {
    texture: Texture<BoxedPixelData>,
    counter: Cell<u32>,
}

struct PixelBufferSource {}

impl PixelBufferSource {
    fn new() -> Self {
        Self {}
    }
}

impl PayloadProvider<BoxedPixelData> for PixelBufferSource {
    fn get_payload(&self) -> BoxedPixelData {
        let rng = fastrand::Rng::new();
        let width = 100i32;
        let height = 100i32;
        let bytes: Vec<u8> = repeat_with(|| rng.u8(..))
            .take((width * height * 4) as usize)
            .collect();
        SimplePixelData::new_boxed(width, height, bytes)
    }
}

impl Animator {
    fn animate(self: &Rc<Self>) {
        self.texture.mark_frame_available().ok();

        let count = self.counter.get();
        self.counter.set(count + 1);

        if count < 120 {
            let self_clone = self.clone();
            RunLoop::current()
                .schedule(Duration::from_millis(100), move || {
                    self_clone.animate();
                })
                .detach();
        }
    }
}

fn init_on_main_thread(engine_handle: i64) -> irondash_texture::Result<i64> {
    let provider = Arc::new(PixelBufferSource::new());
    let texture = Texture::new_with_provider(engine_handle, provider)?;
    let id = texture.id();

    let animator = Rc::new(Animator {
        texture,
        counter: Cell::new(0),
    });
    animator.animate();

    Ok(id)
}

#[no_mangle]
pub extern "C" fn init_texture_example(engine_id: i64, ffi_ptr: *mut c_void, port: i64) {
    init_logging();
    irondash_dart_ffi::irondash_init_ffi(ffi_ptr);
    // Schedule initialization on main thread. When completed return the
    // texture id back to dart through a port.
    RunLoop::sender_for_main_thread().unwrap().send(move || {
        let port = irondash_dart_ffi::DartPort::new(port);
        match init_on_main_thread(engine_id) {
            Ok(id) => {
                port.send(id);
            }
            Err(err) => {
                error!("Error {:?}", err);
                port.send(DartValue::Null);
            }
        }
    });
}
