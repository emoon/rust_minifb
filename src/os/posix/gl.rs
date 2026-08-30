//! EGL + OpenGL ES 2.0 presentation path, shared by the X11 and Wayland
//! backends.
//!
//! This path uploads the caller's buffer untouched and
//! lets the sampler magnify it, so the cost tracks the *buffer* size instead.
//!
//! EGL rather than GLX because EGL binds to both X11 and Wayland; the only
//! per-backend code is obtaining the native window handle, which the caller
//! passes to [`GlContext::new`].
//!
//! Everything is loaded with `dlopen` and every failure is recoverable: the
//! caller falls back to the software path. GL must never become a link-time
//! dependency of minifb.

#![allow(non_camel_case_types, non_snake_case)]

use std::convert::TryFrom;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};

use crate::ScaleMode;

// ---------------------------------------------------------------------------
// EGL
// ---------------------------------------------------------------------------

pub type EGLDisplay = *mut c_void;
pub type EGLConfig = *mut c_void;
pub type EGLContext = *mut c_void;
pub type EGLSurface = *mut c_void;
/// On X11 this is an XID (`unsigned long`), not a pointer. It is passed
/// pointer-sized either way, so the backend casts its handle to this.
pub type EGLNativeWindowType = *mut c_void;
pub type EGLNativeDisplayType = *mut c_void;
/// Which windowing system the native handles came from. Naming it is what
/// keeps EGL from guessing: the unnamed `eglGetDisplay` picks a platform by
/// implementation-defined means (Mesa uses the first configured one, or
/// `$EGL_PLATFORM`) and can misinterpret a `wl_display*` on a build whose
/// default is X11.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Only the Wayland backend calls into this module so far.
pub enum Platform {
    Wayland,
    X11,
}

impl Platform {
    fn egl_enum(self) -> EGLenum {
        match self {
            Platform::Wayland => EGL_PLATFORM_WAYLAND,
            Platform::X11 => EGL_PLATFORM_X11,
        }
    }
}

pub type EGLint = i32;
pub type EGLBoolean = c_uint;
pub type EGLenum = c_uint;

const EGL_NONE: EGLint = 0x3038;
const EGL_FALSE: EGLBoolean = 0;
const EGL_SURFACE_TYPE: EGLint = 0x3033;
const EGL_WINDOW_BIT: EGLint = 0x0004;
const EGL_RENDERABLE_TYPE: EGLint = 0x3040;
const EGL_OPENGL_ES2_BIT: EGLint = 0x0004;
const EGL_RED_SIZE: EGLint = 0x3024;
const EGL_GREEN_SIZE: EGLint = 0x3023;
const EGL_BLUE_SIZE: EGLint = 0x3022;
const EGL_ALPHA_SIZE: EGLint = 0x3021;
const EGL_CONTEXT_CLIENT_VERSION: EGLint = 0x3098;
const EGL_OPENGL_ES_API: EGLenum = 0x30A0;
const EGL_PLATFORM_WAYLAND: EGLenum = 0x31D8;
const EGL_PLATFORM_X11: EGLenum = 0x31D5;

dlib::dlopen_external_library!(Egl,
functions:
    // `eglGetPlatformDisplay` is deliberately *not* bound here: it is EGL 1.5,
    // and dlib fails the whole library on its first missing symbol, so binding
    // it would cost the GPU path outright on an EGL 1.4 driver. It is resolved
    // through `eglGetProcAddress` in `platform_display` instead.
    fn eglGetProcAddress(*const c_char) -> *mut c_void,
    fn eglInitialize(EGLDisplay, *mut EGLint, *mut EGLint) -> EGLBoolean,
    fn eglTerminate(EGLDisplay) -> EGLBoolean,
    fn eglBindAPI(EGLenum) -> EGLBoolean,
    fn eglChooseConfig(EGLDisplay, *const EGLint, *mut EGLConfig, EGLint, *mut EGLint) -> EGLBoolean,
    fn eglCreateContext(EGLDisplay, EGLConfig, EGLContext, *const EGLint) -> EGLContext,
    fn eglDestroyContext(EGLDisplay, EGLContext) -> EGLBoolean,
    fn eglCreateWindowSurface(EGLDisplay, EGLConfig, EGLNativeWindowType, *const EGLint) -> EGLSurface,
    fn eglDestroySurface(EGLDisplay, EGLSurface) -> EGLBoolean,
    fn eglMakeCurrent(EGLDisplay, EGLSurface, EGLSurface, EGLContext) -> EGLBoolean,
    fn eglSwapBuffers(EGLDisplay, EGLSurface) -> EGLBoolean,
    fn eglSwapInterval(EGLDisplay, EGLint) -> EGLBoolean,
    fn eglGetError() -> EGLint,
    fn eglGetConfigAttrib(EGLDisplay, EGLConfig, EGLint, *mut EGLint) -> EGLBoolean,
);

// ---------------------------------------------------------------------------
// OpenGL ES 2.0
// ---------------------------------------------------------------------------

pub type GLenum = c_uint;
pub type GLuint = c_uint;
pub type GLint = c_int;
pub type GLsizei = c_int;
pub type GLbitfield = c_uint;
pub type GLboolean = u8;
pub type GLfloat = f32;
pub type GLchar = c_char;

const GL_TEXTURE_2D: GLenum = 0x0DE1;
const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
const GL_TEXTURE_WRAP_S: GLenum = 0x2802;
const GL_TEXTURE_WRAP_T: GLenum = 0x2803;
const GL_NEAREST: GLint = 0x2600;
const GL_CLAMP_TO_EDGE: GLint = 0x812F;
const GL_UNSIGNED_BYTE: GLenum = 0x1401;
const GL_COLOR_BUFFER_BIT: GLbitfield = 0x0000_4000;
const GL_TRIANGLES: GLenum = 0x0004;
const GL_FLOAT: GLenum = 0x1406;
const GL_FALSE_GL: GLboolean = 0;
const GL_VERTEX_SHADER: GLenum = 0x8B31;
const GL_FRAGMENT_SHADER: GLenum = 0x8B30;
const GL_COMPILE_STATUS: GLenum = 0x8B81;
const GL_LINK_STATUS: GLenum = 0x8B82;
const GL_EXTENSIONS: GLenum = 0x1F03;
const GL_RENDERER: GLenum = 0x1F01;
const GL_UNPACK_ALIGNMENT: GLenum = 0x0CF5;
const GL_MAX_TEXTURE_SIZE: GLenum = 0x0D33;
const GL_NO_ERROR: GLenum = 0;

/// `GL_BGRA_EXT` from `GL_EXT_texture_format_BGRA8888`, which is the same
/// value as desktop GL's core `GL_BGRA`.
const GL_BGRA: GLenum = 0x80E1;

dlib::dlopen_external_library!(Gles2,
functions:
    fn glGetString(GLenum) -> *const u8,
    fn glGetError() -> GLenum,
    fn glGetIntegerv(GLenum, *mut GLint) -> (),
    fn glViewport(GLint, GLint, GLsizei, GLsizei) -> (),
    fn glClearColor(GLfloat, GLfloat, GLfloat, GLfloat) -> (),
    fn glClear(GLbitfield) -> (),
    fn glPixelStorei(GLenum, GLint) -> (),
    fn glGenTextures(GLsizei, *mut GLuint) -> (),
    fn glDeleteTextures(GLsizei, *const GLuint) -> (),
    fn glBindTexture(GLenum, GLuint) -> (),
    fn glTexParameteri(GLenum, GLenum, GLint) -> (),
    fn glTexImage2D(GLenum, GLint, GLint, GLsizei, GLsizei, GLint, GLenum, GLenum, *const c_void) -> (),
    fn glTexSubImage2D(GLenum, GLint, GLint, GLint, GLsizei, GLsizei, GLenum, GLenum, *const c_void) -> (),
    fn glCreateShader(GLenum) -> GLuint,
    fn glDeleteShader(GLuint) -> (),
    fn glShaderSource(GLuint, GLsizei, *const *const GLchar, *const GLint) -> (),
    fn glCompileShader(GLuint) -> (),
    fn glGetShaderiv(GLuint, GLenum, *mut GLint) -> (),
    fn glGetShaderInfoLog(GLuint, GLsizei, *mut GLsizei, *mut GLchar) -> (),
    fn glCreateProgram() -> GLuint,
    fn glDeleteProgram(GLuint) -> (),
    fn glAttachShader(GLuint, GLuint) -> (),
    fn glLinkProgram(GLuint) -> (),
    fn glGetProgramiv(GLuint, GLenum, *mut GLint) -> (),
    fn glUseProgram(GLuint) -> (),
    fn glGetAttribLocation(GLuint, *const GLchar) -> GLint,
    fn glGetUniformLocation(GLuint, *const GLchar) -> GLint,
    fn glUniform1i(GLint, GLint) -> (),
    fn glEnableVertexAttribArray(GLuint) -> (),
    fn glVertexAttribPointer(GLuint, GLint, GLenum, GLboolean, GLsizei, *const c_void) -> (),
    fn glDrawArrays(GLenum, GLint, GLsizei) -> (),
);

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

/// `v_uv` is `highp` in every stage. `mediump` guarantees only 2^-10 relative
/// precision, which cannot resolve adjacent texels of a texture wider than
/// ~1024 -- and `GL_MAX_TEXTURE_SIZE` is 2048 or more on any real driver, so
/// falling back to it would sample the wrong texel on buffers this path
/// happily accepts. `highp` is mandatory in the vertex language.
const VERTEX_SHADER: &str = "\
attribute vec2 a_pos;
attribute vec2 a_uv;
varying highp vec2 v_uv;
void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
";

/// The buffer's top byte is `X`, not alpha, and callers leave it zero, so
/// sampling it would blend an opaque window away against the desktop.
///
/// `highp` is optional in the fragment language, so a driver without it is
/// rejected here rather than silently sampling the wrong texels. `#error`
/// makes the shader ill-formed, which surfaces as [`GlError::Shader`] and
/// sends the window down the software path like any other setup failure.
const FRAGMENT_SHADER_OPAQUE: &str = "\
#ifndef GL_FRAGMENT_PRECISION_HIGH
#error minifb needs highp in the fragment language to sample texels exactly
#endif
precision highp float;
varying highp vec2 v_uv;
uniform sampler2D u_tex;
void main() {
    gl_FragColor = vec4(texture2D(u_tex, v_uv).rgb, 1.0);
}
";

/// Used only when the caller asked for `WindowOptions::transparency`, where
/// the top byte really is alpha.
const FRAGMENT_SHADER_ALPHA: &str = "\
#ifndef GL_FRAGMENT_PRECISION_HIGH
#error minifb needs highp in the fragment language to sample texels exactly
#endif
precision highp float;
varying highp vec2 v_uv;
uniform sampler2D u_tex;
void main() {
    gl_FragColor = texture2D(u_tex, v_uv);
}
";

// ---------------------------------------------------------------------------
// ScaleMode geometry
// ---------------------------------------------------------------------------

/// Destination rectangle in pixels, top-down with the origin at the window's
/// top-left -- the same orientation as the buffer and the shm framebuffer.
/// `x1`/`y1` are the far edge, not a size. [`quad_vertices`] does the single
/// flip into GL's y-up clip space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DestRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

/// Where each [`ScaleMode`] puts the buffer inside the window.
///
/// This mirrors the C scalers in `src/native/posix/scalar.c`, integer
/// truncation included, because those are what this path replaces: toggling
/// `use_gpu` must not move the image within the window. That means the
/// odd-remainder case resolves top-down -- a 2-row buffer centred in a 5-row
/// window leaves 1 row above and 2 below. The macOS `calculate_scaling()` computes the same
/// layout in Cocoa's y-up space and so rounds the other way; matching the
/// backend a caller can actually switch between wins over matching that.
pub fn dest_rect(
    scale_mode: ScaleMode,
    buf_width: i32,
    buf_height: i32,
    win_width: i32,
    win_height: i32,
) -> DestRect {
    match scale_mode {
        ScaleMode::Stretch => DestRect {
            x0: 0,
            y0: 0,
            x1: win_width,
            y1: win_height,
        },

        ScaleMode::AspectRatioStretch => {
            let buffer_aspect = buf_width as f32 / buf_height as f32;
            let win_aspect = win_width as f32 / win_height as f32;

            if buffer_aspect > win_aspect {
                // `.max(1)` mirrors `image_resize_linear_aspect_fill`, which
                // clamps a collapsed axis to one pixel. Without it an
                // extreme-aspect buffer (10000x1 in a 640x480 window) draws a
                // zero-height quad here and a one-pixel strip there.
                let new_height = ((win_width as f32 / buffer_aspect) as i32).max(1);
                let offset = (win_height - new_height) / 2;
                DestRect {
                    x0: 0,
                    y0: offset,
                    x1: win_width,
                    y1: offset + new_height,
                }
            } else {
                let new_width = ((win_height as f32 * buffer_aspect) as i32).max(1);
                let offset = (win_width - new_width) / 2;
                DestRect {
                    x0: offset,
                    y0: 0,
                    x1: offset + new_width,
                    y1: win_height,
                }
            }
        }

        // An oversized buffer hangs off both edges by the same truncated half,
        // which is what `image_center` expresses as a crop into the source.
        ScaleMode::Center => {
            let pos_y = (win_height - buf_height) / 2;
            let pos_x = (win_width - buf_width) / 2;
            DestRect {
                x0: pos_x,
                y0: pos_y,
                x1: buf_width + pos_x,
                y1: buf_height + pos_y,
            }
        }

        ScaleMode::UpperLeft => DestRect {
            x0: 0,
            y0: 0,
            x1: buf_width,
            y1: buf_height,
        },
    }
}

/// Whether a zero-dimension buffer leaves the previous frame up rather than
/// clearing: `image_resize_linear` returns without writing, the other three
/// scalers clear to `bg_color` first.
fn preserves_frame_when_empty(scale_mode: ScaleMode) -> bool {
    matches!(scale_mode, ScaleMode::Stretch)
}

/// Six interleaved `x, y, u, v` vertices - two triangles - in clip space.
///
/// The uv range is the whole texture, so `GL_NEAREST` reads the texel under
/// each fragment centre -- `floor((j + 0.5) * src / dst)`, as on macOS, which
/// is up to half a destination pixel later than the C scalers' index.
///
/// The rect is top-down and clip space is y-up, so `y` is flipped; texture row
/// 0 lands at the *top*, which is `v = 0` at the larger clip-space `y`.
fn quad_vertices(rect: DestRect, win_width: i32, win_height: i32) -> [GLfloat; 24] {
    let to_ndc_x = |px: i32| (px as f32 / win_width as f32) * 2.0 - 1.0;
    let to_ndc_y = |px: i32| 1.0 - (px as f32 / win_height as f32) * 2.0;

    let (l, r) = (to_ndc_x(rect.x0), to_ndc_x(rect.x1));
    // `y0` is the rect's top edge, so it maps to the larger clip-space y.
    let (t, b) = (to_ndc_y(rect.y0), to_ndc_y(rect.y1));

    [
        // triangle 1: bottom-left, bottom-right, top-right
        l, b, 0.0, 1.0, //
        r, b, 1.0, 1.0, //
        r, t, 1.0, 0.0, //
        // triangle 2: bottom-left, top-right, top-left
        l, b, 0.0, 1.0, //
        r, t, 1.0, 0.0, //
        l, t, 0.0, 0.0,
    ]
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Why a GL context could not be created. Every variant is a reason to fall
/// back to the software path rather than to fail window creation.
#[derive(Debug)]
pub enum GlError {
    /// `libEGL.so.1`, `libGLESv2.so.2` or `libwayland-egl.so.1` is not
    /// installed.
    LibraryMissing(&'static str),
    /// The library loaded, but a call that builds the native window handle EGL
    /// draws into returned nothing.
    NativeWindow(&'static str),
    /// An EGL call failed; the `EGLint` is `eglGetError`'s code.
    Egl(&'static str, EGLint),
    /// The driver has no `GL_EXT_texture_format_BGRA8888`, so it cannot take
    /// minifb's buffer without a conversion this path deliberately avoids.
    NoBgra,
    /// A shader failed to compile or link.
    Shader(String),
    /// A GL call reported an error; the `GLenum` is `glGetError`'s code.
    Gl(&'static str, GLenum),
    /// The buffer is wider or taller than `GL_MAX_TEXTURE_SIZE`. The software
    /// scaler has no such limit, so this is a reason to hand the frame back to
    /// it rather than to present a black window.
    TextureTooLarge { width: i32, height: i32, max: i32 },
    /// `GL_RENDERER` names a CPU rasteriser, so there is no GPU to accelerate
    /// with. See [`is_software_renderer`].
    SoftwareRenderer(String),
    /// The target is big-endian, where this path cannot describe minifb's
    /// pixel layout to GL. See [`GlContext::new`].
    BigEndian,
}

impl std::fmt::Display for GlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlError::LibraryMissing(lib) => write!(f, "could not load {}", lib),
            GlError::NativeWindow(call) => write!(f, "{} failed", call),
            GlError::Egl(call, code) => write!(f, "{} failed (EGL error 0x{:x})", call, code),
            GlError::NoBgra => {
                write!(f, "driver lacks GL_EXT_texture_format_BGRA8888")
            }
            GlError::Shader(log) => write!(f, "shader error: {}", log),
            GlError::Gl(call, code) => write!(f, "{} failed (GL error 0x{:x})", call, code),
            GlError::TextureTooLarge { width, height, max } => write!(
                f,
                "buffer is {}x{} but GL_MAX_TEXTURE_SIZE is {}",
                width, height, max
            ),
            GlError::SoftwareRenderer(name) => {
                write!(f, "{} is a software renderer, not a GPU", name)
            }
            GlError::BigEndian => {
                write!(f, "GL_BGRA upload needs a little-endian target")
            }
        }
    }
}

/// Owns the EGL objects while `GlContext::new` is still assembling them.
///
/// Every failure below has to unwind fully before the `Egl`/`Gles2` handles
/// go out of scope: dropping those `dlclose`s libEGL while a context may still
/// be current, which faults inside the driver. A guard gets that right for the
/// `?` paths too, which is where the shader errors return from.
struct EglBootstrap<'a> {
    egl: &'a Egl,
    display: EGLDisplay,
    surface: EGLSurface,
    context: EGLContext,
    armed: bool,
}

impl<'a> EglBootstrap<'a> {
    fn new(egl: &'a Egl, display: EGLDisplay) -> Self {
        Self {
            egl,
            display,
            surface: std::ptr::null_mut(),
            context: std::ptr::null_mut(),
            armed: true,
        }
    }

    /// Hand ownership to the caller; teardown becomes `GlContext`'s job.
    fn release(mut self) -> (EGLDisplay, EGLSurface, EGLContext) {
        self.armed = false;
        (self.display, self.surface, self.context)
    }
}

impl Drop for EglBootstrap<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        unsafe { teardown_egl(self.egl, self.display, self.surface, self.context) };
    }
}

/// Release a context and its display in the order EGL requires. GL object
/// deletion must happen while the context is still current, so callers that
/// own GL objects delete them before calling this.
///
/// The `eglTerminate` below is only safe because no two windows ever share an
/// `EGLDisplay`: EGL does not reference-count `eglInitialize`/`eglTerminate`,
/// so terminating one would tear down the other's contexts and surfaces.
/// `EGLDisplay`s are keyed on the native display pointer, and both backends
/// open a fresh connection per window (`Connection::connect_to_env` in
/// `wayland.rs`, `XOpenDisplay` in `x11.rs`), so the pointers differ. Caching
/// or sharing a connection across windows would make this a use-after-free.
unsafe fn teardown_egl(egl: &Egl, display: EGLDisplay, surface: EGLSurface, context: EGLContext) {
    let null = std::ptr::null_mut();
    (egl.eglMakeCurrent)(display, null, null, null);
    if !surface.is_null() {
        (egl.eglDestroySurface)(display, surface);
    }
    if !context.is_null() {
        (egl.eglDestroyContext)(display, context);
    }
    (egl.eglTerminate)(display);
}

pub struct GlContext {
    egl: Egl,
    gl: Gles2,

    display: EGLDisplay,
    surface: EGLSurface,
    context: EGLContext,

    program: GLuint,
    texture: GLuint,
    a_pos: GLuint,
    a_uv: GLuint,

    /// Dimensions the texture was last allocated at. A buffer of the same size
    /// re-uploads with `glTexSubImage2D`; a different size reallocates.
    tex_width: i32,
    tex_height: i32,

    /// Scratch used to tighten a padded buffer before upload. GLES2 has no
    /// `GL_UNPACK_ROW_LENGTH`, and the surface cannot fall back to shm once a
    /// `wl_egl_window` is attached to it, so repacking is the only option.
    repack: Vec<u32>,

    /// Whether the caller asked for `WindowOptions::transparency`, which
    /// decides both the fragment shader and the clear alpha.
    transparent: bool,

    /// `GL_MAX_TEXTURE_SIZE`. A buffer past this is rejected rather than
    /// uploaded, because a failed allocation leaves the texture incomplete and
    /// GLES2 samples an incomplete texture as opaque black.
    max_texture_size: i32,

    /// Whether anything has been swapped yet. A Wayland surface with no
    /// buffer attached is not mapped, so the first frame cannot be skipped.
    presented: bool,
}

impl GlContext {
    /// `native_display` is a `Display*` on X11 and a `wl_display*` on Wayland.
    /// `native_window` is an X11 `Window` xid cast to a pointer, or a
    /// `wl_egl_window*`.
    ///
    /// # Safety
    ///
    /// Both handles must be valid, must match `platform`, and must outlive the
    /// returned context.
    pub unsafe fn new(
        platform: Platform,
        native_display: EGLNativeDisplayType,
        native_window: EGLNativeWindowType,
        transparent: bool,
    ) -> Result<Self, GlError> {
        let egl = Egl::open("libEGL.so.1").map_err(|_| GlError::LibraryMissing("libEGL.so.1"))?;
        // `glTexImage2D` is told `GL_BGRA`/`GL_UNSIGNED_BYTE`, which consumes
        // the texture as a byte stream: B, G, R, A in ascending address order.
        // minifb's pixel is a `u32` `0x00RRGGBB`, whose bytes land in that
        // order only on a little-endian target. On a big-endian one they are
        // `00 RR GG BB`, so GL would read the padding byte as blue and the
        // blue channel as alpha. GLES2 has no swizzle and no packed 8888
        // format to say this with, and byte-swapping every frame would spend
        // exactly the CPU pass this path exists to avoid -- so hand these
        // targets to the software scaler, which writes the `u32` straight
        // through and is correct either way.
        if cfg!(target_endian = "big") {
            return Err(GlError::BigEndian);
        }

        // The GLES2 soname is .so.2 - there is no libGLESv2.so.1.
        let gl =
            Gles2::open("libGLESv2.so.2").map_err(|_| GlError::LibraryMissing("libGLESv2.so.2"))?;

        let display = platform_display(&egl, platform.egl_enum(), native_display);
        if display.is_null() {
            return Err(GlError::Egl("eglGetPlatformDisplay", (egl.eglGetError)()));
        }

        if (egl.eglInitialize)(display, std::ptr::null_mut(), std::ptr::null_mut()) == EGL_FALSE {
            // Nothing to tear down: the display never initialised.
            return Err(GlError::Egl("eglInitialize", (egl.eglGetError)()));
        }

        // From here on every exit has to unwind through the guard.
        let mut boot = EglBootstrap::new(&egl, display);

        if (egl.eglBindAPI)(EGL_OPENGL_ES_API) == EGL_FALSE {
            return Err(GlError::Egl("eglBindAPI", (egl.eglGetError)()));
        }

        // No depth or stencil: this draws one unlit quad.
        let config_attrs = [
            EGL_SURFACE_TYPE,
            EGL_WINDOW_BIT,
            EGL_RENDERABLE_TYPE,
            EGL_OPENGL_ES2_BIT,
            EGL_RED_SIZE,
            8,
            EGL_GREEN_SIZE,
            8,
            EGL_BLUE_SIZE,
            8,
            // An alpha channel the caller did not ask for is not harmless:
            // the compositor would blend the window against the desktop using
            // the buffer's unused top byte.
            EGL_ALPHA_SIZE,
            if transparent { 8 } else { 0 },
            EGL_NONE,
        ];

        // eglChooseConfig treats sizes as *minimums* and sorts larger ones
        // first, so asking for EGL_ALPHA_SIZE 0 still returns 8-bit-alpha
        // configs. Enumerate and match the alpha size exactly.
        let mut configs: [EGLConfig; 32] = [std::ptr::null_mut(); 32];
        let mut num_config: EGLint = 0;
        if (egl.eglChooseConfig)(
            display,
            config_attrs.as_ptr(),
            configs.as_mut_ptr(),
            configs.len() as EGLint,
            &mut num_config,
        ) == EGL_FALSE
            || num_config == 0
        {
            return Err(GlError::Egl("eglChooseConfig", (egl.eglGetError)()));
        }

        let want_alpha: EGLint = if transparent { 8 } else { 0 };
        let mut config: EGLConfig = std::ptr::null_mut();
        for &candidate in configs.iter().take(num_config as usize) {
            let mut alpha: EGLint = -1;
            if (egl.eglGetConfigAttrib)(display, candidate, EGL_ALPHA_SIZE, &mut alpha) != EGL_FALSE
                && alpha == want_alpha
            {
                config = candidate;
                break;
            }
        }
        if config.is_null() {
            config = configs[0];
        }

        let ctx_attrs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
        boot.context =
            (egl.eglCreateContext)(display, config, std::ptr::null_mut(), ctx_attrs.as_ptr());
        if boot.context.is_null() {
            return Err(GlError::Egl("eglCreateContext", (egl.eglGetError)()));
        }

        boot.surface =
            (egl.eglCreateWindowSurface)(display, config, native_window, std::ptr::null());
        if boot.surface.is_null() {
            return Err(GlError::Egl("eglCreateWindowSurface", (egl.eglGetError)()));
        }

        if (egl.eglMakeCurrent)(display, boot.surface, boot.surface, boot.context) == EGL_FALSE {
            return Err(GlError::Egl("eglMakeCurrent", (egl.eglGetError)()));
        }

        if !has_extension(&gl, "GL_EXT_texture_format_BGRA8888") {
            return Err(GlError::NoBgra);
        }

        // `UseGPU::Auto` promises a *GPU*, and the only reason to want this
        // path is that scaling stops costing window-sized work on the CPU. A
        // software rasteriser breaks that trade twice over: it rasterises the
        // window-sized quad on the CPU anyway, without the C scalers' tuning.
        let renderer = renderer_string(&gl);
        if is_software_renderer(&renderer) {
            return Err(GlError::SoftwareRenderer(renderer));
        }

        // Do not block on vblank. `wl_surface::commit` on the shm path returns
        // immediately and pacing comes from `set_target_fps`/`UpdateRate`, so
        // an interval of 1 would make `eglSwapBuffers` wait on top of that and
        // add up to a frame of input latency -- events are only dispatched in
        // `update()`, after the present.
        (egl.eglSwapInterval)(display, 0);

        let program = build_program(&gl, transparent)?;
        (gl.glUseProgram)(program);

        let a_pos = attrib(&gl, program, "a_pos")?;
        let a_uv = attrib(&gl, program, "a_uv")?;

        let u_tex = CString::new("u_tex").expect("literal has no interior nul");
        let loc = (gl.glGetUniformLocation)(program, u_tex.as_ptr());
        (gl.glUniform1i)(loc, 0);

        let mut texture: GLuint = 0;
        (gl.glGenTextures)(1, &mut texture);
        (gl.glBindTexture)(GL_TEXTURE_2D, texture);
        (gl.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        (gl.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        // Nearest, not linear. The sampler could filter for free, but the
        // software scaler is nearest-neighbour and the macOS Metal shader asks
        // for `mag_filter::nearest`, so anything else would blur upscaled
        // pixel art on this backend alone.
        (gl.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        (gl.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);

        // Rows are u32, so they are always 4-byte aligned.
        (gl.glPixelStorei)(GL_UNPACK_ALIGNMENT, 4);

        let mut max_texture_size: GLint = 0;
        (gl.glGetIntegerv)(GL_MAX_TEXTURE_SIZE, &mut max_texture_size);
        // A driver that fails the query leaves this at zero, which would
        // reject every buffer. GLES2 mandates at least 64.
        let max_texture_size = max_texture_size.max(64);

        let (display, surface, context) = boot.release();

        Ok(GlContext {
            egl,
            gl,
            display,
            surface,
            context,
            program,
            texture,
            a_pos,
            a_uv,
            tex_width: 0,
            tex_height: 0,
            repack: Vec::new(),
            transparent,
            max_texture_size,
            presented: false,
        })
    }

    /// Upload `buffer` and present it, scaled per `scale_mode`.
    ///
    /// `buf_stride` may exceed `buf_width`; the extra columns are dropped by
    /// repacking, since the surface cannot be handed back to the shm path once
    /// EGL owns it.
    ///
    /// An `Err` means nothing was presented and the caller must fall back to
    /// the software path, which -- because the shm path cannot attach to a
    /// surface EGL owns -- also means tearing this context down, including for
    /// [`GlError::TextureTooLarge`]. That one is worth coming back from
    /// though: it is checked before any GL call, so the context was still
    /// healthy when it went. `max` is reported so the caller can rebuild once
    /// a buffer fits again rather than lose the GPU for the whole session.
    ///
    /// # Safety
    ///
    /// `buffer` must hold at least `buf_stride.max(buf_width) * buf_height`
    /// pixels.
    // One blit's worth of geometry and format; grouping it into a struct would
    // just move the same eight values behind another name.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn present(
        &mut self,
        buffer: &[u32],
        buf_width: usize,
        buf_height: usize,
        buf_stride: usize,
        win_width: i32,
        win_height: i32,
        scale_mode: ScaleMode,
        bg_color: u32,
    ) -> Result<(), GlError> {
        // Not swapping leaves the last frame up, which is what the scaler
        // this replaces shows. Ahead of the size checks on purpose: nothing
        // here touches the texture, so a degenerate size -- which
        // `check_buffer_size` accepts, it multiplies by the height -- must not
        // cost the GL context.
        if (buf_width == 0 || buf_height == 0)
            && preserves_frame_when_empty(scale_mode)
            && self.presented
        {
            return Ok(());
        }

        // The buffer dimensions arrive as `usize` and every GL entry point
        // takes `GLsizei`. Casting would wrap a dimension past `i32::MAX` to a
        // negative one, which slips under the size check below and then skips
        // the upload, presenting a cleared window and reporting success --
        // i.e. silently losing the frame instead of falling back. Such a size
        // is reachable from safe code: `check_buffer_size` multiplies by the
        // height, so a zero height lets any width through.
        //
        // Anything that does not fit is past every real `GL_MAX_TEXTURE_SIZE`
        // anyway, so it reports as oversized and takes that recovery path.
        let too_large = |max| GlError::TextureTooLarge {
            width: buf_width.min(i32::MAX as usize) as i32,
            height: buf_height.min(i32::MAX as usize) as i32,
            max,
        };
        let (buf_width, buf_height, buf_stride) = match (
            i32::try_from(buf_width),
            i32::try_from(buf_height),
            i32::try_from(buf_stride),
        ) {
            (Ok(w), Ok(h), Ok(s)) => (w, h, s),
            _ => return Err(too_large(self.max_texture_size)),
        };

        // Checked before touching GL at all. `glTexImage2D` would reject an
        // oversized buffer with GL_INVALID_VALUE and leave the texture with no
        // storage, and GLES2 samples an incomplete texture as opaque black --
        // so without this the window would go black and stay black, silently,
        // instead of reaching the software scaler, which has no size limit.
        if buf_width > self.max_texture_size || buf_height > self.max_texture_size {
            return Err(GlError::TextureTooLarge {
                width: buf_width,
                height: buf_height,
                max: self.max_texture_size,
            });
        }

        if (self.egl.eglMakeCurrent)(self.display, self.surface, self.surface, self.context)
            == EGL_FALSE
        {
            return Err(GlError::Egl("eglMakeCurrent", (self.egl.eglGetError)()));
        }

        let gl = &self.gl;

        (gl.glViewport)(0, 0, win_width, win_height);

        let (r, g, b) = (
            ((bg_color >> 16) & 0xff) as f32 / 255.0,
            ((bg_color >> 8) & 0xff) as f32 / 255.0,
            (bg_color & 0xff) as f32 / 255.0,
        );
        // The software path writes `bg_color` straight into the shm buffer,
        // and `set_background_color` leaves its top byte zero -- so on a
        // transparent window the bars around the image are fully transparent
        // there. Clearing to alpha 1.0 would make them opaque here instead.
        let a = if self.transparent {
            ((bg_color >> 24) & 0xff) as f32 / 255.0
        } else {
            1.0
        };
        (gl.glClearColor)(r, g, b, a);
        (gl.glClear)(GL_COLOR_BUFFER_BIT);

        // Only modes whose scaler clears reach here with an empty buffer, and
        // the clear above is already the frame they want.
        if buf_width > 0 && buf_height > 0 {
            self.upload(buffer, buf_width, buf_height, buf_stride)?;
            self.draw_quad(buf_width, buf_height, win_width, win_height, scale_mode);
        }

        if (self.egl.eglSwapBuffers)(self.display, self.surface) == EGL_FALSE {
            return Err(GlError::Egl("eglSwapBuffers", (self.egl.eglGetError)()));
        }
        self.presented = true;

        Ok(())
    }

    /// # Safety
    ///
    /// As [`GlContext::present`]: `buffer` must hold at least
    /// `buf_stride.max(buf_width) * buf_height` pixels.
    unsafe fn upload(
        &mut self,
        buffer: &[u32],
        buf_width: i32,
        buf_height: i32,
        buf_stride: i32,
    ) -> Result<(), GlError> {
        let gl = &self.gl;

        let pixels = if buf_stride > buf_width {
            repack_rows(
                &mut self.repack,
                buffer,
                buf_width as usize,
                buf_height as usize,
                buf_stride as usize,
            );
            self.repack.as_ptr()
        } else {
            buffer.as_ptr()
        };

        (gl.glBindTexture)(GL_TEXTURE_2D, self.texture);

        if buf_width != self.tex_width || buf_height != self.tex_height {
            (gl.glTexImage2D)(
                GL_TEXTURE_2D,
                0,
                GL_BGRA as GLint,
                buf_width,
                buf_height,
                0,
                GL_BGRA,
                GL_UNSIGNED_BYTE,
                pixels as *const c_void,
            );
            let err = (gl.glGetError)();
            if err != GL_NO_ERROR {
                // The texture has no storage now. Clearing the cached size
                // forces a full reallocation next time rather than a
                // `glTexSubImage2D` into storage that does not exist.
                self.tex_width = 0;
                self.tex_height = 0;
                return Err(GlError::Gl("glTexImage2D", err));
            }
            self.tex_width = buf_width;
            self.tex_height = buf_height;
        } else {
            (gl.glTexSubImage2D)(
                GL_TEXTURE_2D,
                0,
                0,
                0,
                buf_width,
                buf_height,
                GL_BGRA,
                GL_UNSIGNED_BYTE,
                pixels as *const c_void,
            );
            let err = (gl.glGetError)();
            if err != GL_NO_ERROR {
                return Err(GlError::Gl("glTexSubImage2D", err));
            }
        }

        Ok(())
    }

    unsafe fn draw_quad(
        &self,
        buf_width: i32,
        buf_height: i32,
        win_width: i32,
        win_height: i32,
        scale_mode: ScaleMode,
    ) {
        let gl = &self.gl;

        let rect = dest_rect(scale_mode, buf_width, buf_height, win_width, win_height);
        let verts = quad_vertices(rect, win_width, win_height);

        (gl.glUseProgram)(self.program);

        // GLES2 permits client-side arrays, and `verts` outlives the
        // glDrawArrays below that reads it.
        let stride = (4 * std::mem::size_of::<GLfloat>()) as GLsizei;
        let base = verts.as_ptr() as *const c_void;
        (gl.glVertexAttribPointer)(self.a_pos, 2, GL_FLOAT, GL_FALSE_GL, stride, base);
        (gl.glEnableVertexAttribArray)(self.a_pos);
        (gl.glVertexAttribPointer)(
            self.a_uv,
            2,
            GL_FLOAT,
            GL_FALSE_GL,
            stride,
            base.add(2 * std::mem::size_of::<GLfloat>()),
        );
        (gl.glEnableVertexAttribArray)(self.a_uv);

        (gl.glDrawArrays)(GL_TRIANGLES, 0, 6);
    }
}

impl Drop for GlContext {
    fn drop(&mut self) {
        unsafe {
            // Deleting GL objects needs the context current; `teardown_egl`
            // unbinds afterwards. If the bind fails -- a lost context, which is
            // exactly what brings most teardowns here -- skip the deletes
            // rather than calling into the driver unbound. Destroying the
            // context reclaims them anyway.
            if (self.egl.eglMakeCurrent)(self.display, self.surface, self.surface, self.context)
                != EGL_FALSE
            {
                (self.gl.glDeleteTextures)(1, &self.texture);
                (self.gl.glDeleteProgram)(self.program);
            }
            teardown_egl(&self.egl, self.display, self.surface, self.context);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// `eglGetPlatformDisplay`, resolved at runtime. Null means no display.
///
/// Naming the platform avoids `eglGetDisplay`'s implementation-defined guess,
/// which can read a `wl_display*` as an X11 `Display*` and fault inside the
/// driver rather than returning an error. That call is EGL 1.5;
/// `EGL_EXT_platform_base` spells the same thing `eglGetPlatformDisplayEXT` on
/// EGL 1.4, so both are tried. Binding either as a plain `dlib` symbol would
/// instead cost the whole GPU path on any driver that lacks it.
///
/// There is deliberately no `eglGetDisplay` fallback. A stack with neither
/// entry point is EGL 1.4 without `EGL_EXT_platform_base`, which has not
/// shipped alongside a working `libwayland-egl` in a decade; taking the
/// ambiguous call for it would trade a guaranteed software fallback for a
/// possible fault, on the path `UseGPU::Auto` puts everyone by default.
unsafe fn platform_display(
    egl: &Egl,
    platform: EGLenum,
    native_display: EGLNativeDisplayType,
) -> EGLDisplay {
    // The two entry points differ only in the pointee type of the attribute
    // list (`EGLAttrib *` against `EGLint *`), which is immaterial here: it is
    // always null and passed as a plain pointer either way.
    type GetPlatformDisplay =
        unsafe extern "C" fn(EGLenum, *mut c_void, *const c_void) -> EGLDisplay;

    for name in [
        b"eglGetPlatformDisplay\0".as_ref(),
        b"eglGetPlatformDisplayEXT\0".as_ref(),
    ] {
        let sym = (egl.eglGetProcAddress)(name.as_ptr() as *const c_char);
        if sym.is_null() {
            continue;
        }
        let get: GetPlatformDisplay = std::mem::transmute(sym);
        let display = get(platform, native_display, std::ptr::null());
        if !display.is_null() {
            return display;
        }
    }

    std::ptr::null_mut()
}

unsafe fn renderer_string(gl: &Gles2) -> String {
    let p = (gl.glGetString)(GL_RENDERER);
    if p.is_null() {
        return String::new();
    }
    std::ffi::CStr::from_ptr(p as *const c_char)
        .to_string_lossy()
        .into_owned()
}

/// Whether a `GL_RENDERER` string names a CPU rasteriser.
///
/// Substring matching is the only option -- GLES2 exposes no "is this real
/// hardware" query -- so this errs towards missing one rather than rejecting a
/// real GPU: a false positive silently costs someone their acceleration, a
/// false negative only leaves things as they were before this path existed.
/// Every name here is a Mesa/Google software driver with no hardware namesake.
fn is_software_renderer(renderer: &str) -> bool {
    const SOFTWARE: [&str; 5] = [
        "llvmpipe",
        "softpipe",
        "swiftshader",
        "software rasterizer",
        "mesa offscreen",
    ];
    let lower = renderer.to_ascii_lowercase();
    SOFTWARE.iter().any(|name| lower.contains(name))
}

unsafe fn has_extension(gl: &Gles2, name: &str) -> bool {
    let p = (gl.glGetString)(GL_EXTENSIONS);
    if p.is_null() {
        return false;
    }
    std::ffi::CStr::from_ptr(p as *const c_char)
        .to_string_lossy()
        .split_whitespace()
        .any(|e| e == name)
}

/// Copy `height` rows of `width` pixels out of a `stride`-padded `src` into
/// `dst`, dropping the padding.
///
/// GLES2 has no `GL_UNPACK_ROW_LENGTH`, so a padded buffer has to be tightened
/// before upload. `src` must hold at least `stride * height` pixels, which is
/// what `check_buffer_size` already guarantees for every caller.
fn repack_rows(dst: &mut Vec<u32>, src: &[u32], width: usize, height: usize, stride: usize) {
    dst.clear();
    dst.reserve(width * height);
    for row in src.chunks(stride).take(height) {
        dst.extend_from_slice(&row[..width]);
    }
}

unsafe fn attrib(gl: &Gles2, program: GLuint, name: &str) -> Result<GLuint, GlError> {
    let c = CString::new(name).expect("literal has no interior nul");
    let loc = (gl.glGetAttribLocation)(program, c.as_ptr());
    if loc < 0 {
        // Collapsing -1 to 0 would alias the two attributes onto one slot and
        // draw garbage geometry instead of failing.
        return Err(GlError::Shader(format!("attribute {} not found", name)));
    }
    Ok(loc as GLuint)
}

unsafe fn compile(gl: &Gles2, kind: GLenum, source: &str) -> Result<GLuint, GlError> {
    let shader = (gl.glCreateShader)(kind);
    let ptr = source.as_ptr() as *const GLchar;
    // Pass the length rather than relying on a terminator: `&str` carries no
    // NUL of its own, so a source built at runtime would otherwise have the
    // driver read past the end of its allocation.
    let len = source.len() as GLint;
    (gl.glShaderSource)(shader, 1, &ptr, &len);
    (gl.glCompileShader)(shader);

    let mut status: GLint = 0;
    (gl.glGetShaderiv)(shader, GL_COMPILE_STATUS, &mut status);
    if status == 0 {
        let mut log = vec![0u8; 1024];
        let mut len: GLsizei = 0;
        (gl.glGetShaderInfoLog)(
            shader,
            log.len() as GLsizei,
            &mut len,
            log.as_mut_ptr() as *mut GLchar,
        );
        log.truncate(len.max(0) as usize);
        (gl.glDeleteShader)(shader);
        return Err(GlError::Shader(String::from_utf8_lossy(&log).into_owned()));
    }

    Ok(shader)
}

unsafe fn build_program(gl: &Gles2, transparent: bool) -> Result<GLuint, GlError> {
    let fragment = if transparent {
        FRAGMENT_SHADER_ALPHA
    } else {
        FRAGMENT_SHADER_OPAQUE
    };
    let vs = compile(gl, GL_VERTEX_SHADER, VERTEX_SHADER)?;
    let fs = match compile(gl, GL_FRAGMENT_SHADER, fragment) {
        Ok(fs) => fs,
        Err(e) => {
            (gl.glDeleteShader)(vs);
            return Err(e);
        }
    };

    let program = (gl.glCreateProgram)();
    (gl.glAttachShader)(program, vs);
    (gl.glAttachShader)(program, fs);
    (gl.glLinkProgram)(program);

    // Attached shaders live until the program is deleted.
    (gl.glDeleteShader)(vs);
    (gl.glDeleteShader)(fs);

    let mut status: GLint = 0;
    (gl.glGetProgramiv)(program, GL_LINK_STATUS, &mut status);
    if status == 0 {
        (gl.glDeleteProgram)(program);
        return Err(GlError::Shader("program link failed".to_owned()));
    }

    Ok(program)
}

// ---------------------------------------------------------------------------
// Wayland
// ---------------------------------------------------------------------------

// A `wl_surface` is not an `EGLNativeWindowType` on its own -- EGL needs a
// `wl_egl_window`, which also carries the size and so has to be resized
// alongside the surface.
#[cfg(feature = "wayland")]
dlib::dlopen_external_library!(WaylandEgl,
functions:
    fn wl_egl_window_create(*mut c_void, c_int, c_int) -> *mut c_void,
    fn wl_egl_window_destroy(*mut c_void) -> (),
    fn wl_egl_window_resize(*mut c_void, c_int, c_int, c_int, c_int) -> (),
);

/// The GL path on Wayland: an EGL context on a `wl_egl_window` wrapping the
/// window's `wl_surface`.
///
/// The context holds an `EGLSurface` built on the `wl_egl_window`, so it has
/// to be torn down first. Rust runs a type's own `Drop::drop` *before*
/// dropping its fields, so field order alone would not give that ordering --
/// `ManuallyDrop` puts the teardown under this type's control instead.
#[cfg(feature = "wayland")]
pub struct WaylandGl {
    context: std::mem::ManuallyDrop<GlContext>,
    egl_window: *mut c_void,
    lib: WaylandEgl,
}

#[cfg(feature = "wayland")]
impl WaylandGl {
    /// # Safety
    ///
    /// `wl_display` and `wl_surface` must be valid and outlive the result.
    pub unsafe fn new(
        wl_display: *mut c_void,
        wl_surface: *mut c_void,
        width: i32,
        height: i32,
        transparent: bool,
    ) -> Result<Self, GlError> {
        let lib = WaylandEgl::open("libwayland-egl.so.1")
            .map_err(|_| GlError::LibraryMissing("libwayland-egl.so.1"))?;

        let egl_window = (lib.wl_egl_window_create)(wl_surface, width, height);
        if egl_window.is_null() {
            return Err(GlError::NativeWindow("wl_egl_window_create"));
        }

        match GlContext::new(Platform::Wayland, wl_display, egl_window, transparent) {
            Ok(context) => Ok(WaylandGl {
                context: std::mem::ManuallyDrop::new(context),
                egl_window,
                lib,
            }),
            Err(e) => {
                (lib.wl_egl_window_destroy)(egl_window);
                Err(e)
            }
        }
    }

    pub fn context(&mut self) -> &mut GlContext {
        &mut self.context
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        unsafe { (self.lib.wl_egl_window_resize)(self.egl_window, width, height, 0, 0) };
    }
}

#[cfg(feature = "wayland")]
impl Drop for WaylandGl {
    fn drop(&mut self) {
        unsafe {
            // Tear the EGLSurface down before the wl_egl_window it was
            // created on; the reverse order is a use-after-free in the driver.
            std::mem::ManuallyDrop::drop(&mut self.context);
            (self.lib.wl_egl_window_destroy)(self.egl_window);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The texel `GL_NEAREST` reads for destination pixel `i` along one axis,
    /// derived from the quad's interpolated uv exactly as the rasterizer would:
    /// interpolate to the fragment centre, scale by the texture size, floor,
    /// and clamp (`GL_CLAMP_TO_EDGE`).
    ///
    /// Host `f32`: GLES2 guarantees `highp` only 2^-16 relative precision, so
    /// this pins down the mapping, not any particular driver.
    fn gl_texel(uv_at_near_edge: f32, uv_at_far_edge: f32, span: i32, src: i32, i: i32) -> i32 {
        let f = (i as f32 + 0.5) / span as f32;
        let uv = uv_at_near_edge + (uv_at_far_edge - uv_at_near_edge) * f;
        ((uv * src as f32).floor() as i32).clamp(0, src - 1)
    }

    /// The source index `image_resize_linear` picks for each destination
    /// index along one axis, read back out of the C scaler rather than
    /// restated here: a one-pixel-thick source whose pixels are their own
    /// index comes back as the indices it chose.
    fn scalar_axis(src: i32, dst: i32, vertical: bool) -> Vec<u32> {
        let source: Vec<u32> = (0..src as u32).collect();
        let mut out = vec![u32::MAX; dst as usize];
        let (src_w, src_h) = if vertical { (1, src) } else { (src, 1) };
        let (dst_w, dst_h) = if vertical { (1, dst) } else { (dst, 1) };

        // SAFETY: `source` holds `src_w * src_h` pixels at stride `src_w`, and
        // `out` holds `dst_w * dst_h`.
        unsafe {
            crate::os::posix::common::image_resize_linear(
                out.as_mut_ptr(),
                dst_w as u32,
                dst_h as u32,
                source.as_ptr(),
                src_w as u32,
                src_h as u32,
                src_w as u32,
            );
        }
        out
    }

    /// One axis of a stretch: never earlier than the C scaler's pixel, never
    /// more than half a destination pixel after it, and exactly it on a
    /// whole-number scale.
    fn assert_axis_tracks_the_scaler(
        uv_at_near_edge: f32,
        uv_at_far_edge: f32,
        span: i32,
        src: i32,
        vertical: bool,
    ) {
        let what = if vertical { "row" } else { "column" };
        let reference = scalar_axis(src, span, vertical);
        // ceil(src / (2 * span)): half a destination pixel, in source pixels.
        let budget = (src as i64 + 2 * span as i64 - 1) / (2 * span as i64);
        let exact = span % src == 0;

        for i in 0..span {
            let gl = gl_texel(uv_at_near_edge, uv_at_far_edge, span, src, i) as i64;
            let sw = reference[i as usize] as i64;

            assert!(
                gl >= sw && gl - sw <= budget,
                "{} {} of {} -> {}: GL reads {}, the scaler reads {}, budget {}",
                what,
                i,
                src,
                span,
                gl,
                sw,
                budget
            );
            if exact {
                assert_eq!(
                    gl, sw,
                    "{} {} of {} -> {} is a whole-number scale",
                    what, i, src, span
                );
            }
        }
    }

    /// The sizes reach past what a window can hold on purpose: everything this
    /// path got wrong before lived at an extreme, and a sweep that stops at
    /// 1280 sees none of it.
    #[test]
    fn nearest_sampling_tracks_the_software_scaler() {
        for (src_w, src_h, win_w, win_h) in [
            (2, 2, 3, 3),         // the 2-to-3 case, where the phase shows
            (3, 3, 2, 2),         // downscale
            (320, 240, 320, 240), // 1:1, where every sample sits on a texel edge
            (320, 240, 1280, 960),
            (320, 240, 641, 481), // odd, non-integer ratio both ways
            (7, 5, 13, 11),
            (13, 11, 7, 5),
            (1, 1, 640, 480),
            (1920, 1080, 3840, 2160), // 1080p into 4K, where an exact phase is sub-ULP
            (2560, 1440, 3840, 2160),
            (3839, 2159, 3840, 2160), // a scale factor barely above 1:1
            (4096, 4096, 3840, 2160), // GL_MAX_TEXTURE_SIZE-sized, downscaled
            (100, 100, 2200000, 1),   // an upscale no window can hold
            (2200000, 1, 100, 100),   // and the downscale back
        ] {
            let rect = dest_rect(ScaleMode::Stretch, src_w, src_h, win_w, win_h);
            let v = quad_vertices(rect, win_w, win_h);

            // Vertex 0 is bottom-left (u0, v1), vertex 2 is top-right (u1, v0).
            let (u0, v_bottom) = (v[2], v[3]);
            let (u1, v_top) = (v[10], v[11]);

            assert_axis_tracks_the_scaler(u0, u1, win_w, src_w, false);
            // Rows run top-down, and `v_top` is the uv at the quad's top edge.
            assert_axis_tracks_the_scaler(v_top, v_bottom, win_h, src_h, true);
        }
    }

    /// Which modes clear on an empty buffer is asked of the C scalers, not
    /// assumed: nothing else would catch `present` drifting from them.
    #[test]
    fn empty_frames_follow_the_scalers() {
        type Scaler = unsafe extern "C" fn(*mut u32, u32, u32, *const u32, u32, u32, u32, u32);

        // `image_resize_linear` takes no `bg_color`; it never clears.
        unsafe extern "C" fn stretch(
            dst: *mut u32,
            dst_width: u32,
            dst_height: u32,
            src: *const u32,
            src_width: u32,
            src_height: u32,
            src_stride: u32,
            _bg_color: u32,
        ) {
            crate::os::posix::common::image_resize_linear(
                dst, dst_width, dst_height, src, src_width, src_height, src_stride,
            )
        }

        let scalers: [(ScaleMode, Scaler); 4] = [
            (ScaleMode::Stretch, stretch),
            (
                ScaleMode::AspectRatioStretch,
                crate::os::posix::common::image_resize_linear_aspect_fill,
            ),
            (ScaleMode::Center, crate::os::posix::common::image_center),
            (
                ScaleMode::UpperLeft,
                crate::os::posix::common::image_upper_left,
            ),
        ];

        for (mode, scaler) in scalers {
            for (src_w, src_h) in [(0, 4), (4, 0), (0, 0)] {
                let source = [0u32; 4];
                let mut dst = [0xAAAA_AAAAu32; 4];

                // SAFETY: `dst` holds 2x2 pixels and `source` covers any of the
                // degenerate source sizes above.
                unsafe {
                    scaler(
                        dst.as_mut_ptr(),
                        2,
                        2,
                        source.as_ptr(),
                        src_w,
                        src_h,
                        src_w,
                        0,
                    );
                }

                let untouched = dst == [0xAAAA_AAAA; 4];
                assert_eq!(
                    untouched,
                    preserves_frame_when_empty(mode),
                    "{:?} with a {}x{} buffer: dst {:x?}",
                    mode,
                    src_w,
                    src_h,
                    dst
                );
            }
        }
    }

    #[test]
    fn stretch_fills_the_window() {
        let r = dest_rect(ScaleMode::Stretch, 320, 240, 1280, 960);
        assert_eq!(
            r,
            DestRect {
                x0: 0,
                y0: 0,
                x1: 1280,
                y1: 960
            }
        );
    }

    /// A 4:3 buffer in a 16:9 window pillarboxes: full height, centred
    /// horizontally, with equal bars either side.
    #[test]
    fn aspect_ratio_pillarboxes_a_narrow_buffer() {
        let r = dest_rect(ScaleMode::AspectRatioStretch, 320, 240, 1920, 1080);
        assert_eq!(r.y0, 0);
        assert_eq!(r.y1, 1080);
        assert_eq!(r.x1 - r.x0, 1440); // 1080 * 4/3
        assert_eq!(r.x0, 1920 / 2 - 1440 / 2);
    }

    /// A 7:3 buffer in a 4:3 window letterboxes instead.
    #[test]
    fn aspect_ratio_letterboxes_a_wide_buffer() {
        let r = dest_rect(ScaleMode::AspectRatioStretch, 2100, 900, 800, 600);
        assert_eq!(r.x0, 0);
        assert_eq!(r.x1, 800);
        // 800 / (2100/900) = 342, centred in 600 leaves 129 above and below.
        assert_eq!(r.y1 - r.y0, 342);
        assert_eq!(r.y0, 129);
        assert_eq!(r.y1, 471);
    }

    /// An odd letterbox gap: `image_resize_linear_aspect_fill` truncates the
    /// *top* margin, so the spare row lands at the bottom.
    #[test]
    fn aspect_ratio_puts_the_odd_letterbox_row_at_the_bottom() {
        let r = dest_rect(ScaleMode::AspectRatioStretch, 2100, 900, 800, 601);
        let height = r.y1 - r.y0;
        assert_eq!(r.y0, (601 - height) / 2, "top margin is the truncated half");
        assert_eq!(601 - r.y1, 601 - height - r.y0, "the spare row is below");
        assert!(601 - r.y1 > r.y0, "{:?}", r);
    }

    #[test]
    fn center_places_a_small_buffer_in_the_middle() {
        let r = dest_rect(ScaleMode::Center, 320, 240, 1280, 960);
        assert_eq!(r.x0, (1280 - 320) / 2);
        assert_eq!(r.y0, (960 - 240) / 2);
        assert_eq!(r.x1 - r.x0, 320);
        assert_eq!(r.y1 - r.y0, 240);
    }

    /// `image_center` truncates the *top* margin, so an odd gap leaves the
    /// extra row below the image, not above it.
    #[test]
    fn center_puts_the_odd_row_below_the_image() {
        let r = dest_rect(ScaleMode::Center, 2, 2, 5, 5);
        assert_eq!(r.y0, 1, "one row above");
        assert_eq!(5 - r.y1, 2, "two rows below");
        assert_eq!(r.x0, 1);
        assert_eq!(5 - r.x1, 2);
    }

    /// An oversized buffer hangs off both edges symmetrically and is clipped.
    /// `image_center` crops `(src - dst) / 2` from the top, so the rect starts
    /// that far above the window.
    #[test]
    fn center_overhangs_a_large_buffer() {
        let r = dest_rect(ScaleMode::Center, 1920, 1080, 640, 480);
        assert_eq!(r.x0, -((1920 - 640) / 2));
        assert_eq!(r.y0, -((1080 - 480) / 2));
        assert_eq!(r.x1 - r.x0, 1920);
        assert_eq!(r.y1 - r.y0, 1080);

        // Odd overhang: the crop is the truncated half of the difference.
        let r = dest_rect(ScaleMode::Center, 5, 5, 2, 2);
        assert_eq!(r.y0, -1);
        assert_eq!(r.x0, -1);
    }

    /// The rect is top-down, so anchoring to the upper left is simply the
    /// origin -- and stays there whether or not the buffer fits.
    #[test]
    fn upper_left_anchors_to_the_top_left() {
        let r = dest_rect(ScaleMode::UpperLeft, 320, 240, 1280, 960);
        assert_eq!(
            r,
            DestRect {
                x0: 0,
                y0: 0,
                x1: 320,
                y1: 240
            }
        );
    }

    #[test]
    fn upper_left_keeps_the_top_left_visible_when_oversized() {
        let r = dest_rect(ScaleMode::UpperLeft, 1920, 1080, 640, 480);
        assert_eq!(r.x0, 0);
        assert_eq!(r.y0, 0, "top edge stays pinned to the window top");
        assert_eq!(r.y1, 1080, "the rest hangs off the bottom");
    }

    /// Row 0 of the buffer must land at the top of the destination, or the
    /// image presents upside down.
    #[test]
    fn quad_puts_texture_row_zero_at_the_top() {
        let rect = dest_rect(ScaleMode::Stretch, 320, 240, 640, 480);
        let v = quad_vertices(rect, 640, 480);

        // Vertex 0 is bottom-left: NDC (-1, -1), and the larger v.
        assert_eq!(&v[0..2], &[-1.0, -1.0]);
        // Vertex 2 is top-right: NDC (1, 1), and the smaller v.
        assert_eq!(&v[8..10], &[1.0, 1.0]);
        assert!(v[11] < v[3], "row 0 must be at the top: {:?}", v);
    }

    /// `check_buffer_size` computes `width * height * 4` as the requirement,
    /// so a zero height makes that zero and any non-empty buffer passes. A
    /// degenerate size therefore reaches this code from safe API use, and
    /// must not panic -- the C scalers guard the same case.
    #[test]
    fn dest_rect_survives_degenerate_sizes() {
        for mode in [
            ScaleMode::Stretch,
            ScaleMode::AspectRatioStretch,
            ScaleMode::Center,
            ScaleMode::UpperLeft,
        ] {
            for (bw, bh, ww, wh) in [
                (0, 240, 640, 480),
                (320, 0, 640, 480),
                (0, 0, 640, 480),
                (320, 240, 0, 480),
                (320, 240, 640, 0),
                (1, 1, 1, 1),
            ] {
                let r = dest_rect(mode, bw, bh, ww, wh);
                // Every field has to be a real number the caller can use; the
                // NaN from 0/0 in the aspect branch would poison this.
                for v in [r.x0, r.y0, r.x1, r.y1] {
                    assert!(v > i32::MIN && v < i32::MAX, "{:?} {:?}", mode, r);
                }
                let q = quad_vertices(r, ww.max(1), wh.max(1));
                assert!(q.iter().all(|f| f.is_finite()), "{:?} {:?}", mode, r);
            }
        }
    }

    /// The padded case `update_with_buffer_stride` would produce if it were
    /// ever made public. Nothing reaches it through today's API, so this is
    /// the only thing keeping the path honest.
    #[test]
    fn repack_drops_row_padding() {
        // 3x2 image inside a stride-5 buffer; 9 is padding.
        let src = [1, 2, 3, 9, 9, 4, 5, 6, 9, 9];
        let mut dst = Vec::new();
        repack_rows(&mut dst, &src, 3, 2, 5);
        assert_eq!(dst, [1, 2, 3, 4, 5, 6]);
    }

    /// A tail row beyond `height` must not be copied, and the scratch buffer
    /// is reused across frames, so a shorter frame must not leave the previous
    /// one's pixels behind it.
    #[test]
    fn repack_reuses_its_scratch_without_leaking_rows() {
        let mut dst = Vec::new();
        repack_rows(&mut dst, &[1, 2, 9, 3, 4, 9, 5, 6, 9], 2, 3, 3);
        assert_eq!(dst, [1, 2, 3, 4, 5, 6]);

        repack_rows(&mut dst, &[7, 8, 9, 9, 9, 9], 2, 1, 3);
        assert_eq!(dst, [7, 8], "stale rows from the longer frame survived");
    }

    #[test]
    fn repack_is_a_plain_copy_when_there_is_no_padding() {
        let mut dst = Vec::new();
        repack_rows(&mut dst, &[1, 2, 3, 4], 2, 2, 2);
        assert_eq!(dst, [1, 2, 3, 4]);
    }

    /// An extreme aspect ratio collapses one axis to zero before the clamp,
    /// which would draw nothing at all where the software scaler draws a
    /// one-pixel strip.
    #[test]
    fn aspect_ratio_keeps_a_collapsed_axis_visible() {
        let r = dest_rect(ScaleMode::AspectRatioStretch, 10000, 1, 640, 480);
        assert!(r.y1 - r.y0 >= 1, "{:?}", r);
        let r = dest_rect(ScaleMode::AspectRatioStretch, 1, 10000, 640, 480);
        assert!(r.x1 - r.x0 >= 1, "{:?}", r);
    }

    #[test]
    fn gl_error_messages_name_the_failure() {
        assert_eq!(
            GlError::LibraryMissing("libEGL.so.1").to_string(),
            "could not load libEGL.so.1"
        );
        // A create failure is not a missing library, and must not read as one.
        assert_eq!(
            GlError::NativeWindow("wl_egl_window_create").to_string(),
            "wl_egl_window_create failed"
        );
        assert_eq!(
            GlError::Egl("eglInitialize", 0x3001).to_string(),
            "eglInitialize failed (EGL error 0x3001)"
        );
        assert_eq!(
            GlError::NoBgra.to_string(),
            "driver lacks GL_EXT_texture_format_BGRA8888"
        );
        assert_eq!(
            GlError::Shader("bad".to_owned()).to_string(),
            "shader error: bad"
        );
        assert_eq!(
            GlError::Gl("glTexImage2D", 0x0501).to_string(),
            "glTexImage2D failed (GL error 0x501)"
        );
        assert_eq!(
            GlError::TextureTooLarge {
                width: 2560,
                height: 1440,
                max: 2048,
            }
            .to_string(),
            "buffer is 2560x1440 but GL_MAX_TEXTURE_SIZE is 2048"
        );
        assert_eq!(
            GlError::SoftwareRenderer("llvmpipe (LLVM 17.0.6, 256 bits)".to_owned()).to_string(),
            "llvmpipe (LLVM 17.0.6, 256 bits) is a software renderer, not a GPU"
        );
        assert_eq!(
            GlError::BigEndian.to_string(),
            "GL_BGRA upload needs a little-endian target"
        );
    }

    /// Rejecting a real GPU costs a user their acceleration for no reason, so
    /// the match has to stay narrow.
    #[test]
    fn software_renderers_are_told_from_hardware() {
        for name in [
            "llvmpipe (LLVM 17.0.6, 256 bits)",
            "softpipe",
            "SwiftShader Device (Subzero)",
            "Mesa Offscreen",
        ] {
            assert!(is_software_renderer(name), "{}", name);
        }

        for name in [
            "AMD Radeon RX 7900 XTX (radeonsi, navi31, LLVM 17.0.6)",
            "Mesa Intel(R) Arc(tm) A770 Graphics (DG2)",
            "NVIDIA GeForce RTX 4090/PCIe/SSE2",
            "Apple M2",
            "Mali-G78",
            "",
        ] {
            assert!(!is_software_renderer(name), "{}", name);
        }
    }

    #[test]
    fn quad_maps_a_centered_rect_into_clip_space() {
        let rect = DestRect {
            x0: 160,
            y0: 120,
            x1: 480,
            y1: 360,
        };
        let v = quad_vertices(rect, 640, 480);
        // Vertex 0 is bottom-left: x0 = 160/640 * 2 - 1 = -0.5, and the rect's
        // *bottom* is y1, flipped to 1 - 360/480 * 2 = -0.5.
        assert_eq!(v[0], -0.5);
        assert_eq!(v[1], -0.5);
        // Vertex 2 is top-right: x1 = 0.5, and y0 flips to 1 - 120/480 * 2 = 0.5.
        assert_eq!(v[8], 0.5);
        assert_eq!(v[9], 0.5);
    }
}
