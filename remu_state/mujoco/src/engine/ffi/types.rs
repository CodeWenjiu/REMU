//! MuJoCo structs whose layout crosses the FFI boundary.
//!
//! Transcribed field-by-field from the pinned mujoco 3.14.0 headers
//! (mjvisualize.h / mjrender.h); the layout is load-bearing.

use std::os::raw::{c_char, c_int};

use super::constants::{
    MAXLIGHT, MAXMATERIAL, MAXTEXTURE, NAUX, NGROUP, NRNDFLAG, NTEXROLE, NVISFLAG,
};

/// Opaque model handle (`mjModel*` in C).
pub(crate) enum MjModel {}
/// Opaque simulation data handle (`mjData*` in C).
pub(crate) enum MjData {}
/// `mjvGeom` — abstract geom; the scene owns the buffer. Transcribed
/// field-by-field from the pinned mujoco 3.14.0 headers (mjvisualize.h) —
/// the layout is load-bearing for indexing `mjvScene.geoms`.
#[repr(C)]
#[allow(dead_code, reason = "some fields exist for layout fidelity only")]
pub(crate) struct MjvGeom {
    pub type_: c_int,
    pub dataid: c_int,
    pub objtype: c_int,
    pub objid: c_int,
    pub category: c_int,
    pub matid: c_int,
    pub texid: c_int,
    pub texuniform: c_int,
    pub texcoord: c_int,
    pub segid: c_int,
    pub size: [f32; 3],
    pub pos: [f32; 3],
    pub mat: [f32; 9],
    pub rgba: [f32; 4],
    pub emission: f32,
    pub specular: f32,
    pub shininess: f32,
    pub reflectance: f32,
    pub texrepeat: [f32; 2],
    pub label: [c_char; 100],
    pub camdist: f32,
    pub modelrbound: f32,
    pub transparent: u8,
}
/// `mjvPerturb` — object selection and perturbation state. Transcribed
/// field-by-field from the pinned mujoco 3.14.0 headers (mjvisualize.h).
#[repr(C)]
#[derive(Clone, Copy, Default)]
#[allow(dead_code, reason = "fields are read and written by the C side")]
pub(crate) struct MjvPerturb {
    pub select: c_int,
    pub flexselect: c_int,
    pub skinselect: c_int,
    pub active: c_int,
    pub active2: c_int,
    pub refpos: [f64; 3],
    pub refquat: [f64; 4],
    pub refselpos: [f64; 3],
    pub localpos: [f64; 3],
    pub localmass: f64,
    pub scale: f64,
}

/// `mjvCamera` — abstract camera.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MjvCamera {
    pub type_: c_int,
    pub fixedcamid: c_int,
    pub trackbodyid: c_int,
    pub lookat: [f64; 3],
    pub distance: f64,
    pub azimuth: f64,
    pub elevation: f64,
    pub orthographic: c_int,
}

/// `mjvGLCamera` — OpenGL camera frame and projection.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MjvGLCamera {
    pub pos: [f32; 3],
    pub forward: [f32; 3],
    pub up: [f32; 3],
    pub frustum_center: f32,
    pub frustum_width: f32,
    pub frustum_bottom: f32,
    pub frustum_top: f32,
    pub frustum_near: f32,
    pub frustum_far: f32,
    pub orthographic: c_int,
}

/// `mjvLight` — OpenGL light.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MjvLight {
    pub id: c_int,
    pub pos: [f32; 3],
    pub dir: [f32; 3],
    pub type_: c_int,
    pub texid: c_int,
    pub attenuation: [f32; 3],
    pub cutoff: f32,
    pub exponent: f32,
    pub ambient: [f32; 3],
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub headlight: u8,
    pub castshadow: u8,
    pub bulbradius: f32,
    pub intensity: f32,
    pub range: f32,
    pub softness: f32,
}

/// `mjvOption` — abstract visualization options.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MjvOption {
    pub label: c_int,
    pub frame: c_int,
    pub geomgroup: [u8; NGROUP],
    pub sitegroup: [u8; NGROUP],
    pub jointgroup: [u8; NGROUP],
    pub tendongroup: [u8; NGROUP],
    pub actuatorgroup: [u8; NGROUP],
    pub flexgroup: [u8; NGROUP],
    pub skingroup: [u8; NGROUP],
    pub flags: [u8; NVISFLAG],
    pub bvh_depth: c_int,
    pub flex_layer: c_int,
}

/// `mjvScene` — abstract scene passed to the OpenGL renderer.
#[repr(C)]
pub(crate) struct MjvScene {
    pub maxgeom: c_int,
    pub ngeom: c_int,
    pub geoms: *mut MjvGeom,
    pub geomorder: *mut c_int,
    pub nflex: c_int,
    pub flexedgeadr: *mut c_int,
    pub flexedgenum: *mut c_int,
    pub flexvertadr: *mut c_int,
    pub flexvertnum: *mut c_int,
    pub flexfaceadr: *mut c_int,
    pub flexfacenum: *mut c_int,
    pub flexfaceused: *mut c_int,
    pub flexedge: *mut c_int,
    pub flexvert: *mut f32,
    pub flexface: *mut f32,
    pub flexnormal: *mut f32,
    pub flextexcoord: *mut f32,
    pub flexvertopt: u8,
    pub flexedgeopt: u8,
    pub flexfaceopt: u8,
    pub flexskinopt: u8,
    pub nskin: c_int,
    pub skinfacenum: *mut c_int,
    pub skinvertadr: *mut c_int,
    pub skinvertnum: *mut c_int,
    pub skinvert: *mut f32,
    pub skinnormal: *mut f32,
    pub nlight: c_int,
    pub lights: [MjvLight; MAXLIGHT],
    pub camera: [MjvGLCamera; 2],
    pub enabletransform: u8,
    pub translate: [f32; 3],
    pub rotate: [f32; 4],
    pub scale: f32,
    pub stereo: c_int,
    pub flags: [u8; NRNDFLAG],
    pub framewidth: c_int,
    pub framergb: [f32; 3],
    pub status: c_int,
}

/// `mjrRect` — OpenGL rectangle.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct MjrRect {
    pub left: c_int,
    pub bottom: c_int,
    pub width: c_int,
    pub height: c_int,
}

/// `mjrContext` — custom OpenGL context (offscreen/shadow/aux objects, textures,
/// fonts, capabilities). Allocated once per render context and owned by the
/// renderer.
#[repr(C)]
pub(crate) struct MjrContext {
    pub line_width: f32,
    pub shadow_clip: f32,
    pub shadow_scale: f32,
    pub fog_start: f32,
    pub fog_end: f32,
    pub fog_rgba: [f32; 4],
    pub shadow_size: c_int,
    pub off_width: c_int,
    pub off_height: c_int,
    pub off_samples: c_int,
    pub font_scale: c_int,
    pub aux_width: [c_int; NAUX],
    pub aux_height: [c_int; NAUX],
    pub aux_samples: [c_int; NAUX],
    pub off_fbo: u32,
    pub off_fbo_r: u32,
    pub off_color: u32,
    pub off_color_r: u32,
    pub off_depth_stencil: u32,
    pub off_depth_stencil_r: u32,
    pub shadow_fbo: u32,
    pub shadow_tex: u32,
    pub aux_fbo: [u32; NAUX],
    pub aux_fbo_r: [u32; NAUX],
    pub aux_color: [u32; NAUX],
    pub aux_color_r: [u32; NAUX],
    pub mat_texid: [c_int; MAXMATERIAL * NTEXROLE],
    pub mat_texuniform: [c_int; MAXMATERIAL],
    pub mat_texrepeat: [f32; MAXMATERIAL * 2],
    pub ntexture: c_int,
    pub texture_type: [c_int; MAXTEXTURE],
    pub texture: [u32; MAXTEXTURE],
    pub base_plane: u32,
    pub base_mesh: u32,
    pub base_hfield: u32,
    pub base_builtin: u32,
    pub base_font_normal: u32,
    pub base_font_shadow: u32,
    pub base_font_big: u32,
    pub range_plane: c_int,
    pub range_mesh: c_int,
    pub range_hfield: c_int,
    pub range_builtin: c_int,
    pub range_font: c_int,
    pub nskin: c_int,
    pub skinvert_vbo: *mut u32,
    pub skinnormal_vbo: *mut u32,
    pub skintexcoord_vbo: *mut u32,
    pub skinface_vbo: *mut u32,
    pub char_width: [c_int; 127],
    pub char_width_big: [c_int; 127],
    pub char_height: c_int,
    pub char_height_big: c_int,
    pub gl_initialized: c_int,
    pub window_available: c_int,
    pub window_samples: c_int,
    pub window_stereo: c_int,
    pub window_doublebuffer: c_int,
    pub current_buffer: c_int,
    pub read_pixel_format: c_int,
    pub read_depth_map: c_int,
}
