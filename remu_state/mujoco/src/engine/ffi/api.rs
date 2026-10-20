//! The dlopen2 table. Signatures mirror the C ABI verbatim.

// The table mirrors C signatures verbatim; their arity is not ours to choose.
#![allow(clippy::too_many_arguments)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

use dlopen2::wrapper::WrapperApi;

use super::types::{
    MjData, MjModel, MjrContext, MjrRect, MjvCamera, MjvGeom, MjvOption, MjvPerturb, MjvScene,
};
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

    #[dlopen2_name = "mj_ray"]
    ray: unsafe extern "C" fn(
        model: *const MjModel,
        data: *const MjData,
        pnt: *const f64,
        vec: *const f64,
        geomgroup: *const u8,
        flg_static: bool,
        bodyexclude: c_int,
        geomid: *mut c_int,
        normal: *mut f64,
    ) -> f64,

    #[dlopen2_name = "mj_multiRay"]
    multi_ray: unsafe extern "C" fn(
        model: *const MjModel,
        data: *mut MjData,
        pnt: *const f64,
        vec: *const f64,
        geomgroup: *const u8,
        flg_static: bool,
        bodyexclude: c_int,
        geomid: *mut c_int,
        dist: *mut f64,
        normal: *mut f64,
        nray: c_int,
        cutoff: f64,
    ),

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

    #[dlopen2_name = "mjv_select"]
    select: unsafe extern "C" fn(
        model: *const MjModel,
        data: *const MjData,
        opt: *const MjvOption,
        aspectratio: f64,
        relx: f64,
        rely: f64,
        scene: *const MjvScene,
        selpnt: *mut f64,
        geomid: *mut c_int,
        flexid: *mut c_int,
        skinid: *mut c_int,
    ) -> c_int,

    #[dlopen2_name = "mjv_initPerturb"]
    init_perturb: unsafe extern "C" fn(
        model: *const MjModel,
        data: *mut MjData,
        scene: *const MjvScene,
        pert: *mut MjvPerturb,
    ),

    #[dlopen2_name = "mjv_movePerturb"]
    move_perturb: unsafe extern "C" fn(
        model: *const MjModel,
        data: *const MjData,
        action: c_int,
        reldx: f64,
        reldy: f64,
        scene: *const MjvScene,
        pert: *mut MjvPerturb,
    ),

    #[dlopen2_name = "mjv_applyPerturbPose"]
    apply_perturb_pose: unsafe extern "C" fn(
        model: *const MjModel,
        data: *mut MjData,
        pert: *const MjvPerturb,
        flg_paused: c_int,
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
