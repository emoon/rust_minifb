extern crate libloading;

use self::libloading::{Library, Symbol};
use std::sync::Arc;
use std::os::raw::{c_void, c_ulong};
use Error;
use Result;

const GL_COLOR_BUFFER_BIT: u32 = 0x00004000;
const GL_DEPTH_TEST: u32 = 0x0B71;
const GL_QUADS: u32 = 0x0007;
const GL_CULL_FACE: u32 = 0x0B44;
const GL_TEXTURE_2D: u32 = 0x0DE1;

const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_NEAREST: u32 = 0x2600;

const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;

pub struct GlLib {
    clear_color: unsafe extern "C" fn(r: f32, g: f32, b: f32, a: f32),
    clear: unsafe extern "C" fn(bits: u32),
    disable: unsafe extern "C" fn(bits: u32),
    enable: unsafe extern "C" fn(bits: u32),
    begin: unsafe extern "C" fn(bits: u32),
    end: unsafe extern "C" fn(),
    tex_coord_2f: unsafe extern "C" fn(u: f32, v: f32),
    vertex_2f: unsafe extern "C" fn(x: f32, y: f32),
    color_3f: unsafe extern "C" fn(g: f32, g: f32, b: f32),
    viewport: unsafe extern "C" fn(x: u32, y: u32, width: u32, height: u32),
    // texture functions
    swap_interval: unsafe extern "C" fn(display: *mut c_void, drawable: c_ulong, interval: u32),
    gen_textures: unsafe extern "C" fn(size: u32, out: *mut u32),
    bind_texture: unsafe extern "C" fn(target: u32, id: u32),
    tex_parameteri: unsafe extern "C" fn(target: u32, id: u32, set: u32),
    tex_image_2d: unsafe extern "C" fn(
        target: u32,
        level: u32,
        int_format: u32,
        width: u32,
        height: u32,
        border: u32,
        format: u32,
        type_: u32,
        pixels: *const u32,
    ),

    // texture handles and other things
    texture_handle: u32,
    texture_width: u32,
    texture_height: u32,
    gl_lib: Arc<libloading::Library>,
}

impl GlLib {
    pub fn load(filename: &str) -> Result<GlLib> {
        let lib =
            Arc::new(Library::new(filename).map_err(|e| {
                Error::WindowCreate(format!("failed to load {}: {:?}", filename, e))
            })?);

        let clear: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glClear\0").unwrap() };
        let disable: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glDisable\0").unwrap() };
        let enable: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glEnable\0").unwrap() };
        let begin: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glBegin\0").unwrap() };
        let end: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glEnd\0").unwrap() };
        let color_3f: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glColor3f\0").unwrap() };
        let viewport: Symbol<unsafe extern "C" fn()> = unsafe { lib.get(b"glViewport\0").unwrap() };

        // TODO: get the correct function for windows here
        // TODO: Make this optional
        let swap_interval: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glXSwapIntervalEXT\0").unwrap() };

        let tex_coord_2f: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glTexCoord2f\0").unwrap() };
        let clear_color: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glClearColor\0").unwrap() };
        let vertex_2f: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glVertex2f\0").unwrap() };
        let gen_textures: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glGenTextures\0").unwrap() };
        let bind_texture: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glBindTexture\0").unwrap() };
        let tex_parameteri: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glTexParameteri\0").unwrap() };
        let tex_image_2d: Symbol<unsafe extern "C" fn()> =
            unsafe { lib.get(b"glTexImage2D\0").unwrap() };

        Ok(GlLib {
            swap_interval: unsafe { std::mem::transmute(*swap_interval.into_raw()) },
            clear_color: unsafe { std::mem::transmute(*clear_color.into_raw()) },
            clear: unsafe { std::mem::transmute(*clear.into_raw()) },
            disable: unsafe { std::mem::transmute(*disable.into_raw()) },
            enable: unsafe { std::mem::transmute(*enable.into_raw()) },
            begin: unsafe { std::mem::transmute(*begin.into_raw()) },
            end: unsafe { std::mem::transmute(*end.into_raw()) },
            vertex_2f: unsafe { std::mem::transmute(*vertex_2f.into_raw()) },
            color_3f: unsafe { std::mem::transmute(*color_3f.into_raw()) },
            viewport: unsafe { std::mem::transmute(*viewport.into_raw()) },
            gen_textures: unsafe { std::mem::transmute(*gen_textures.into_raw()) },
            bind_texture: unsafe { std::mem::transmute(*bind_texture.into_raw()) },
            tex_parameteri: unsafe { std::mem::transmute(*tex_parameteri.into_raw()) },
            tex_image_2d: unsafe { std::mem::transmute(*tex_image_2d.into_raw()) },
            tex_coord_2f: unsafe { std::mem::transmute(*tex_coord_2f.into_raw()) },
            texture_handle: 0,
            texture_width: 0,
            texture_height: 0,
            gl_lib: lib.clone(),
        })
    }

    pub fn set_swap_interval(&self, display: *mut c_void, drawable: c_ulong, interval: u32) {
        unsafe { (self.swap_interval)(display, drawable, interval) };
    }

    pub fn setup(&mut self, width: usize, height: usize) {
        unsafe {
            (self.disable)(GL_DEPTH_TEST);
            (self.disable)(GL_CULL_FACE);
            (self.clear_color)(1.0, 0.0, 1.0, 1.0);
            (self.color_3f)(1.0, 1.0, 1.0);
            (self.enable)(GL_TEXTURE_2D);

            // Setup texture
            (self.gen_textures)(1, &mut self.texture_handle);
            (self.bind_texture)(GL_TEXTURE_2D, self.texture_handle);
            (self.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
            (self.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);

            self.texture_width = width as u32;
            self.texture_height = height as u32;
        }
    }

    pub fn update(&self, image_data: &[u32], width: u32, height: u32) {
        unsafe {
            (self.viewport)(0, 0, width, height);

            (self.tex_image_2d)(
                GL_TEXTURE_2D,
                0,
                GL_RGBA,
                self.texture_width,
                self.texture_height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                image_data.as_ptr(),
            );

            // Render quad

            (self.begin)(GL_QUADS);
            // lower left
            (self.tex_coord_2f)(0.0, 1.0);
            (self.vertex_2f)(-1.0, -1.0);
            // lower right
            (self.tex_coord_2f)(1.0, 1.0);
            (self.vertex_2f)(1.0, -1.0);
            // upper right
            (self.tex_coord_2f)(1.0, 0.0);
            (self.vertex_2f)(1.0, 1.0);
            // upper left
            (self.tex_coord_2f)(0.0, 0.0);
            (self.vertex_2f)(-1.0, 1.0);
            (self.end)();
        }
    }
}
