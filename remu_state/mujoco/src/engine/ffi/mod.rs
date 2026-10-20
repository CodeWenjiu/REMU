//! MuJoCo C ABI: state flags, transcribed structs, and the dlopen2 table.

remu_macro::mod_prv!(api, constants, types);

pub(crate) use api::MujocoApi;
pub(crate) use constants::{
    CAT_ALL, FB_WINDOW, FONT_SHADOW, FONTSCALE_150, GEOM_LINE, GRID_TOPLEFT, MOUSE_MOVE_H,
    MOUSE_ROTATE_H, MOUSE_ROTATE_V, MOUSE_ZOOM, OBJ_BODY, OBJ_JOINT, PERT_TRANSLATE,
    RND_REFLECTION, RND_SHADOW, RND_SKYBOX, SAMEFRAME_NONE, STATE_CTRL, STATE_MOCAP_POS,
    STATE_QPOS, STATE_QVEL, STATE_TIME,
};
#[cfg(test)]
pub(crate) use constants::FB_OFFSCREEN;
pub(crate) use types::{
    MjData, MjModel, MjrContext, MjrRect, MjvCamera, MjvOption, MjvPerturb, MjvScene,
};
#[cfg(test)]
pub(crate) use types::MjvGeom;
