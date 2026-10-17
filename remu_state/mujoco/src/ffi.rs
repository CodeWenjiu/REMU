//! MuJoCo C ABI: opaque handles, state-signature flags, and the dlopen2 table.
//!
//! Only primitives cross the boundary (pointers, ints, `f64` buffers); state is
//! read/written through the state-signature API (`mj_stateSize` / `mj_getState`
//! / `mj_setState`), so no `mjModel`/`mjData` layout is assumed. The flag
//! constants come from `mjtype.h` (`mjtState` / `mjtObj`); `session.rs` tests
//! guard them against drift via engine-reported sizes.

// The table mirrors C signatures verbatim; their arity is not ours to choose.
#![allow(clippy::too_many_arguments)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

use dlopen2::wrapper::WrapperApi;

/// Opaque model handle (`mjModel*` in C).
pub(crate) enum MjModel {}
/// Opaque simulation data handle (`mjData*` in C).
pub(crate) enum MjData {}

// `mjtState` bits (mjtype.h).
pub(crate) const STATE_TIME: c_int = 1 << 0;
pub(crate) const STATE_QPOS: c_int = 1 << 1;
pub(crate) const STATE_QVEL: c_int = 1 << 2;
pub(crate) const STATE_CTRL: c_int = 1 << 6;

// `mjtObj` values (mjtype.h).
pub(crate) const OBJ_BODY: c_int = 1;
pub(crate) const OBJ_JOINT: c_int = 3;

// ── Viewer (mjv/mjr) FFI ──
//
// The scene/context structs are passed by pointer to the renderer, so their
// layout must match the engine's exactly: they are transcribed field-by-field
// from the pinned mujoco 3.14.0 headers (mjvisualize.h / mjrender.h). The
// session tests guard the version via `mj_version`.

// Sizes from the same headers (mjNGROUP/mjMAXLIGHT in mjvisualize.h;
// mjNAUX/mjMAXMATERIAL/mjMAXTEXTURE in mjrender.h; mjNTEXROLE: count of
// `mjtTextureRole`; mjNVISFLAG/mjNRNDFLAG: counts of flag enums).
pub(crate) const NGROUP: usize = 6;
pub(crate) const MAXLIGHT: usize = 100;
pub(crate) const NVISFLAG: usize = 31;
pub(crate) const NRNDFLAG: usize = 11;
pub(crate) const NAUX: usize = 10;
pub(crate) const MAXMATERIAL: usize = 1000;
pub(crate) const MAXTEXTURE: usize = 1000;
pub(crate) const NTEXROLE: usize = 10;

// `mjtCamera` (mjvisualize.h).
pub(crate) const CAMERA_FREE: c_int = 0;
// `mjtCatBit` (mjvisualize.h).
pub(crate) const CAT_ALL: c_int = 7;
// `mjtGeom` values (mjtype.h).
pub(crate) const GEOM_LINE: c_int = 103;
// `mjtSameFrame` value (mjtype.h).
pub(crate) const SAMEFRAME_NONE: u8 = 0;
// `mjtMouse` (mjvisualize.h).
pub(crate) const MOUSE_ROTATE_V: c_int = 1;
pub(crate) const MOUSE_ROTATE_H: c_int = 2;
pub(crate) const MOUSE_MOVE_V: c_int = 3;
pub(crate) const MOUSE_MOVE_H: c_int = 4;
pub(crate) const MOUSE_ZOOM: c_int = 5;
// `mjtRndFlag` (mjvisualize.h).
pub(crate) const RND_SHADOW: usize = 0;
pub(crate) const RND_REFLECTION: usize = 2;
pub(crate) const RND_SKYBOX: usize = 4;
// `mjtFont` / `mjtGridPos` / `mjtFontScale` (mjrender.h).
pub(crate) const FONT_SHADOW: c_int = 1;
pub(crate) const GRID_TOPLEFT: c_int = 0;
pub(crate) const FONTSCALE_150: c_int = 150;
// `mjtFrameBuffer` (mjrender.h).
pub(crate) const FB_WINDOW: c_int = 0;
pub(crate) const FB_OFFSCREEN: c_int = 1;

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
/// Opaque perturbation state (`mjvPerturb`); the viewer passes null.
pub(crate) enum MjvPerturb {}

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

#[derive(WrapperApi)]
pub(crate) struct MujocoApi {
    #[dlopen2_name = "mj_version"]
    version: unsafe extern "C" fn() -> c_int,

    #[dlopen2_name = "mj_loadXML"]
    load_xml: unsafe extern "C" fn(
        filename: *const c_char,
        vfs: *const c_void,
        error: *mut c_char,
        error_sz: c_int,
    ) -> *mut MjModel,

    #[dlopen2_name = "mj_deleteModel"]
    delete_model: unsafe extern "C" fn(model: *mut MjModel),

    #[dlopen2_name = "mj_makeData"]
    make_data: unsafe extern "C" fn(model: *const MjModel) -> *mut MjData,

    #[dlopen2_name = "mj_deleteData"]
    delete_data: unsafe extern "C" fn(data: *mut MjData),

    #[dlopen2_name = "mj_step"]
    step: unsafe extern "C" fn(model: *const MjModel, data: *mut MjData),

    #[dlopen2_name = "mj_forward"]
    forward: unsafe extern "C" fn(model: *const MjModel, data: *mut MjData),

    #[dlopen2_name = "mj_resetData"]
    reset_data: unsafe extern "C" fn(model: *const MjModel, data: *mut MjData),

    #[dlopen2_name = "mj_stateSize"]
    state_size: unsafe extern "C" fn(model: *const MjModel, sig: c_int) -> c_int,

    #[dlopen2_name = "mj_getState"]
    get_state: unsafe extern "C" fn(
        model: *const MjModel,
        data: *const MjData,
        state: *mut f64,
        sig: c_int,
    ),

    #[dlopen2_name = "mj_setState"]
    set_state: unsafe extern "C" fn(
        model: *const MjModel,
        data: *mut MjData,
        state: *const f64,
        sig: c_int,
    ),

    #[dlopen2_name = "mj_name2id"]
    name2id:
        unsafe extern "C" fn(model: *const MjModel, obj_type: c_int, name: *const c_char) -> c_int,

    #[dlopen2_name = "mj_local2Global"]
    local2_global: unsafe extern "C" fn(
        data: *mut MjData,
        xpos: *mut f64,
        xmat: *mut f64,
        pos: *const f64,
        quat: *const f64,
        body: c_int,
        sameframe: u8,
    ),

    #[dlopen2_name = "mj_id2name"]
    id2name:
        unsafe extern "C" fn(model: *const MjModel, obj_type: c_int, id: c_int) -> *const c_char,

    #[dlopen2_name = "mjv_defaultFreeCamera"]
    default_free_camera: unsafe extern "C" fn(model: *const MjModel, camera: *mut MjvCamera),

    #[dlopen2_name = "mjv_moveCamera"]
    move_camera: unsafe extern "C" fn(
        model: *const MjModel,
        action: c_int,
        reldx: f64,
        reldy: f64,
        camera: *mut MjvCamera,
    ),

    #[dlopen2_name = "mjv_defaultOption"]
    default_option: unsafe extern "C" fn(opt: *mut MjvOption),

    #[dlopen2_name = "mjv_makeScene"]
    make_scene: unsafe extern "C" fn(model: *const MjModel, scene: *mut MjvScene, maxgeom: c_int),

    #[dlopen2_name = "mjv_freeScene"]
    free_scene: unsafe extern "C" fn(scene: *mut MjvScene),

    #[dlopen2_name = "mjv_updateScene"]
    update_scene: unsafe extern "C" fn(
        model: *const MjModel,
        data: *mut MjData,
        opt: *const MjvOption,
        perturb: *const MjvPerturb,
        camera: *mut MjvCamera,
        catmask: c_int,
        scene: *mut MjvScene,
    ),

    #[dlopen2_name = "mjv_initGeom"]
    init_geom: unsafe extern "C" fn(
        geom: *mut MjvGeom,
        type_: c_int,
        size: *const f64,
        pos: *const f64,
        mat: *const f64,
        rgba: *const f32,
    ),

    #[dlopen2_name = "mjv_connector"]
    connector: unsafe extern "C" fn(
        geom: *mut MjvGeom,
        type_: c_int,
        width: f64,
        from: *const f64,
        to: *const f64,
    ),

    #[dlopen2_name = "mjr_makeContext"]
    make_context:
        unsafe extern "C" fn(model: *const MjModel, con: *mut MjrContext, fontscale: c_int),

    #[dlopen2_name = "mjr_freeContext"]
    free_context: unsafe extern "C" fn(con: *mut MjrContext),

    #[dlopen2_name = "mjr_setBuffer"]
    set_buffer: unsafe extern "C" fn(framebuffer: c_int, con: *mut MjrContext),

    #[dlopen2_name = "mjr_render"]
    render: unsafe extern "C" fn(viewport: MjrRect, scene: *mut MjvScene, con: *const MjrContext),

    #[dlopen2_name = "mjr_overlay"]
    overlay: unsafe extern "C" fn(
        font: c_int,
        gridpos: c_int,
        viewport: MjrRect,
        overlay: *const c_char,
        overlay2: *const c_char,
        con: *const MjrContext,
    ),
}
