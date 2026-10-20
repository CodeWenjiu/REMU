//! Panel state: obstacle mirroring, selection, and readout.

use super::ArmUi;
use crate::ArmSnapshot;

/// An obstacle beyond this coordinate is parked.
const PARK_THRESHOLD: f64 = 5.0;

/// Obstacle positions mirrored from the scene: the last 3D-dragged obstacle is
/// selected, and park/place edits its position before publishing.
#[derive(Default)]
pub(super) struct DragState {
    /// Flat `x y z` per mocap body, in mocap-index order.
    pub(super) pos: Vec<f64>,
    /// Model-defined pose per obstacle, restored by "place".
    pub(super) home: Vec<[f64; 3]>,
    pub(super) selected: usize,
    pub(super) seeded: bool,
}

impl ArmUi {
    /// Keep the panel's obstacle controls in sync with the model and the UI's
    /// own drag state (the UI is the only writer after the first snapshot).
    pub(super) fn sync_panel(&self, snap: &ArmSnapshot) {
        let mut drag = self.drag.borrow_mut();
        if !drag.seeded && drag.pos.len() != snap.mocap.len() {
            drag.pos.clone_from(&snap.mocap);
            drag.home = if snap.mocap_home.len() == snap.mocap.len() {
                snap.mocap_home.as_chunks::<3>().0.to_vec()
            } else {
                snap.mocap.as_chunks::<3>().0.to_vec()
            };
            drag.seeded = true;
        }
        let count = drag.pos.len() / 3;
        if drag.selected >= count {
            drag.selected = count.saturating_sub(1);
        }
        drop(drag);
        self.refresh_panel();
    }

    /// Adopt positions moved by a 3D drag: select the obstacle that changed
    /// and keep the panel's readout on it.
    pub(crate) fn follow_mocap(&self, pos: &[f64]) {
        let mut drag = self.drag.borrow_mut();
        if drag.pos.len() != pos.len() {
            return;
        }
        let old = drag.pos.as_chunks::<3>().0;
        let new = pos.as_chunks::<3>().0;
        if let Some(changed) = old.iter().zip(new).position(|(a, b)| *a != *b) {
            drag.selected = changed;
        }
        drag.pos.clone_from_slice(pos);
        drag.seeded = true;
        drop(drag);
        self.refresh_panel();
    }

    /// Select the obstacle a 3D click picked.
    pub(crate) fn select_mocap(&self, index: usize) {
        let mut drag = self.drag.borrow_mut();
        if index < drag.pos.len() / 3 {
            drag.selected = index;
        }
        drop(drag);
        self.refresh_panel();
    }

    /// Push the current drag state into the panel properties.
    fn refresh_panel(&self) {
        let drag = self.drag.borrow();
        let count = drag.pos.len() / 3;
        let (x, y, z) = if count == 0 {
            (0.0, 0.0, 0.0)
        } else {
            let i = drag.selected.min(count - 1);
            (
                drag.pos[3 * i],
                drag.pos[3 * i + 1],
                drag.pos[3 * i + 2],
            )
        };
        self.ui.set_count(count as i32);
        self.ui.set_selected(drag.selected as i32);
        self.ui.set_parked(is_parked(x, y));
        self.ui
            .set_position_text(format!("x {x:.2}  y {y:.2}  z {z:.2}").into());
    }
}

/// An obstacle is parked when it sits outside the scene (the UI parks it far
/// away, out of both the scanner's range and the view).
pub(super) fn is_parked(x: f64, y: f64) -> bool {
    x.abs() > PARK_THRESHOLD || y.abs() > PARK_THRESHOLD
}
