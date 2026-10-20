//! `mjtState` / `mjtObj` / viewer constants, transcribed from the pinned
//! mujoco 3.14.0 headers (mjtype.h / mjvisualize.h / mjrender.h).

use std::os::raw::c_int;
// `mjtState` bits (mjtype.h).
pub(crate) const STATE_TIME: c_int = 1 << 0;
pub(crate) const STATE_QPOS: c_int = 1 << 1;
pub(crate) const STATE_QVEL: c_int = 1 << 2;
pub(crate) const STATE_CTRL: c_int = 1 << 6;
pub(crate) const STATE_MOCAP_POS: c_int = 1 << 10;

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
// `mjtPertBit` (mjvisualize.h).
pub(crate) const PERT_TRANSLATE: c_int = 1;
pub(crate) const PERT_ROTATE: c_int = 2;
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
