use std::{cell::Cell, ffi::c_void, iter::repeat_with, rc::Rc, sync::Arc, time::Duration};

use irondash_dart_ffi::DartValue;
use irondash_engine_context::EngineContext;
use irondash_run_loop::RunLoop;
use irondash_texture::{
    BoxedGLTexture, BoxedPixelData, GLTextureProvider, PayloadProvider, SimplePixelData, Texture,
};
use log::error;

use glow::*;

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

struct Animator<Type> {
    texture: Texture<Type>,
    counter: Cell<u32>,
}

struct PixelBufferSource {}

impl PixelBufferSource {}

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

pub struct MyGdkWrapper(*mut gdk_sys::GdkGLContext);

impl MyGdkWrapper {
    pub fn as_gdk(&self) -> *mut gdk_sys::GdkGLContext {
        self.0
    }
}

unsafe impl Send for MyGdkWrapper {}
unsafe impl Sync for MyGdkWrapper {}

struct GLTextureSource {
    gdk_context: MyGdkWrapper,
    gl_context: glow::Context,
    gl_texture: Option<NativeTexture>,
    gl_framebuffer: Option<NativeFramebuffer>,
    gl_vertexarray: Option<NativeVertexArray>,
    gl_program: Option<NativeProgram>,
}

impl GLTextureSource {
    pub fn init_gl_context_from_gdk(engine_handle: i64) -> Result<Self, String> {
        let engine = EngineContext::get().unwrap();
        let fl_view = engine.get_flutter_view(engine_handle).unwrap();
        let fl_view = unsafe { std::mem::transmute(fl_view) };
        let gtk_widget = unsafe {
            std::mem::transmute(gobject_sys::g_type_check_instance_cast(
                fl_view,
                gtk_sys::gtk_widget_get_type(),
            ))
        };

        let window = unsafe { gtk_sys::gtk_widget_get_parent_window(gtk_widget) };
        let mut error: *mut glib_sys::GError = std::ptr::null_mut();
        let error_ptr: *mut *mut glib_sys::GError = &mut error;
        let gdk_context =
            MyGdkWrapper(unsafe { gdk_sys::gdk_window_create_gl_context(window, error_ptr) });

        unsafe { gdk_sys::gdk_gl_context_make_current(gdk_context.as_gdk()) };

        gl_loader::init_gl();

        let gl_context = unsafe {
            glow::Context::from_loader_function(|s| {
                std::mem::transmute(gl_loader::get_proc_address(s))
            })
        };

        unsafe {
            gdk_sys::gdk_gl_context_clear_current();
        }

        Ok(Self {
            gdk_context,
            gl_context,
            gl_texture: None,
            gl_framebuffer: None,
            gl_vertexarray: None,
            gl_program: None,
        })
    }

    pub fn init_gl_state(&mut self, width: u32, height: u32) -> Result<(), String> {
        unsafe {
            gdk_sys::gdk_gl_context_make_current(self.gdk_context.as_gdk());

            let gl = &self.gl_context;
            self.gl_program = Self::init_shaders(gl);

            self.gl_texture = Some(gl.create_texture()?);
            gl.bind_texture(TEXTURE_2D, self.gl_texture);
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                RGB as i32,
                width as i32,
                height as i32,
                0,
                RGB,
                UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
            gl.bind_texture(TEXTURE_2D, None);

            let gl_framebuffer = gl.create_framebuffer()?;
            gl.bind_framebuffer(FRAMEBUFFER, Some(gl_framebuffer));

            let attachment_point = COLOR_ATTACHMENT0;
            gl.framebuffer_texture_2d(
                FRAMEBUFFER,
                attachment_point,
                TEXTURE_2D,
                self.gl_texture,
                0,
            );

            //let draw_buffers = [attachment_point];
            //gl.draw_buffers(draw_buffers.as_slice());

            if gl.check_framebuffer_status(FRAMEBUFFER) != FRAMEBUFFER_COMPLETE {
                return Err("Framebuffer setup failed".to_string());
            }
            //gl.bind_framebuffer(FRAMEBUFFER, None);

            self.gl_vertexarray = Some(
                gl.create_vertex_array()
                    .expect("Cannot create vertex array"),
            );

            gdk_sys::gdk_gl_context_clear_current();
        }

        Ok(())
    }

    fn init_shaders(gl: &glow::Context) -> Option<NativeProgram> {
        let shader_version = "#version 410";
        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"const vec2 verts[3] = vec2[3](
            vec2(0.5f, 1.0f),
            vec2(0.0f, 0.0f),
            vec2(1.0f, 0.0f)
        );
        out vec2 vert;
        void main() {
            vert = verts[gl_VertexID];
            gl_Position = vec4(vert - 0.5, 0.0, 1.0);
        }"#,
                r#"precision mediump float;
        in vec2 vert;
        out vec4 color;
        void main() {
            color = vec4(vert, 0.5, 1.0);
        }"#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in shader_sources.iter() {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!("{}", gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!("{}", gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            Some(program)
        }
    }
}

struct GLTexture {
    pub target: u32,
    pub name: u32,
    pub width: i32,
    pub height: i32,
}

impl GLTexture {
    fn new(target: u32, name: u32, width: i32, height: i32) -> Self {
        Self {
            target,
            name,
            width,
            height,
        }
    }
}

impl GLTextureProvider for GLTexture {
    fn get(&self) -> irondash_texture::GLTexture {
        irondash_texture::GLTexture {
            target: self.target,
            name: &self.name,
            width: self.width,
            height: self.height,
        }
    }
}

impl PayloadProvider<BoxedGLTexture> for GLTextureSource {
    fn get_payload(&self) -> BoxedGLTexture {
        //let rng = fastrand::Rng::new();
        let width = 100i32;
        let height = 100i32;

        let gl = &self.gl_context;
        unsafe {
            gdk_sys::gdk_gl_context_make_current(self.gdk_context.as_gdk());

            gl.use_program(self.gl_program);

            gl.bind_texture(TEXTURE_2D, self.gl_texture);
            gl.bind_framebuffer(FRAMEBUFFER, self.gl_framebuffer);

            gl.viewport(0, 0, width, height);
            gl.clear_color(0.1, 0.2, 0.3, 1.0);
            gl.clear(COLOR_BUFFER_BIT);

            let colors = [1.0f32, 0.0f32, 0.0f32, 1.0f32];
            gl.clear_buffer_f32_slice(COLOR, 0, colors.as_slice());

            gl.bind_vertex_array(self.gl_vertexarray);
            gl.draw_arrays(TRIANGLES, 0, 3);

            gdk_sys::gdk_gl_context_clear_current();
        }

        Box::new(GLTexture::new(
            TEXTURE_2D,
            self.gl_texture.unwrap().0.get(),
            width,
            height,
        ))
    }

    //fn destroy(&self) {
    //    gl.delete_program(self.gl_program);
    //    gl.delete_vertex_array(self.gl_vertexarray);
    //}
}

impl<Type: 'static> Animator<Type> {
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
    // let provider = if use_gl {
    //     Arc::new(GLTextureSource::new(engine_handle));
    // } else {
    //     Arc::new(PixelBufferSource::new());
    // };
    let mut provider = GLTextureSource::init_gl_context_from_gdk(engine_handle).unwrap();
    provider.init_gl_state(100, 100).unwrap();

    let texture = Texture::new_with_provider(engine_handle, Arc::new(provider))?;
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
