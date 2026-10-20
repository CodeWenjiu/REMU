//! Ray casting over the current data.

use std::os::raw::c_int;

use super::Session;
use crate::MujocoError;

impl Session {
    /// Cast `nray` rays from `pnt` along the flattened unit vectors in `dirs`,
    /// writing the distance to the nearest visible geom into `dists`
    /// (negative = no hit within `range_max`; static geoms included).
    pub(crate) fn multi_ray(
        &mut self,
        pnt: [f64; 3],
        dirs: &[f64],
        dists: &mut [f64],
        range_max: f64,
    ) -> Result<(), MujocoError> {
        let nray = dists.len();
        if dirs.len() != 3 * nray {
            return Err(MujocoError::State(format!(
                "multi_ray: dirs length {} != 3 * {nray}",
                dirs.len()
            )));
        }
        let mut geomid = vec![-1 as c_int; nray];
        // Safety: `pnt`/`dirs` are read-only inputs sized 3 and 3*nray; the
        // engine transiently normalizes `data` but leaves the state intact.
        unsafe {
            self.api.multi_ray(
                self.model,
                self.data,
                pnt.as_ptr(),
                dirs.as_ptr(),
                std::ptr::null(),
                true,
                -1,
                geomid.as_mut_ptr(),
                dists.as_mut_ptr(),
                std::ptr::null_mut(),
                nray as c_int,
                range_max,
            );
        }
        Ok(())
    }
}
