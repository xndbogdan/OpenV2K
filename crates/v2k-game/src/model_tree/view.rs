//! Scene camera input for model command selection and painter ordering.

use v2k_formats::models::ModelViewSelection;
use v2k_render::{Camera, ModelDepthFade};

use super::ModelTreePainterComposition;

/// The actual installed scene camera, in the same world coordinates as the
/// submitted model. View-X projection reflection does not change its origin.
#[derive(Clone, Copy, Debug)]
pub struct ModelTreeView {
    pub position: [f32; 3],
    pub forward: [f32; 3],
}

impl From<&Camera> for ModelTreeView {
    fn from(camera: &Camera) -> Self {
        Self {
            position: camera.position,
            forward: camera.forward(),
        }
    }
}

impl ModelTreeView {
    /// `464E60` rejects the entire node at equality, before `46D580` and
    /// command execution. A header-0x20 parent bypasses its own test only;
    /// each child receives its own header, radius and transformed origin.
    pub(super) fn admits_model(
        self,
        model_flags: u8,
        radius_raw: u16,
        position: [f32; 3],
        scale: f32,
        far_plane: Option<f32>,
    ) -> bool {
        if model_flags & 0x20 != 0 {
            return true;
        }
        let Some(far) = far_plane else {
            return true;
        };
        let origin_depth: f32 = (0..3)
            .map(|axis| (position[axis] - self.position[axis]) * self.forward[axis])
            .sum();
        let radius = f32::from(radius_raw) * scale.abs() / 100.0;
        origin_depth - radius < far
    }

    pub(super) fn selection(
        self,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        scale: f32,
    ) -> ModelViewSelection {
        // 466160 produces model-origin-minus-camera in the node's local
        // raw axes. Invert ModelTree's /100 world scale and orthogonal basis
        // (including reflected actor axes) before 46D3F0 adds its raw anchor.
        // This adapts the existing float scene pose. It is not a bit-exact
        // replacement for retail's separately shifted Q31 matrix products.
        let local_origin_from_camera_raw = std::array::from_fn(|axis| {
            let local: f64 = (0..3)
                .map(|world_axis| {
                    (f64::from(position[world_axis]) - f64::from(self.position[world_axis])) * 100.0
                        / f64::from(scale)
                        * f64::from(orientation[world_axis][axis])
                })
                .sum();
            local.trunc() as i64 as i32
        });
        ModelViewSelection::Retail {
            local_origin_from_camera_raw,
        }
    }
}

/// The explicit submission owns its planes even when scene fog is disabled.
/// Frontend fixed planes use /100; ordinary linear planes already use world
/// view units. Model scale never selects this coordinate policy implicitly.
pub(super) fn model_far_plane(
    depth_fade: ModelDepthFade,
    world_fog_planes: Option<[f32; 2]>,
) -> Option<f32> {
    match depth_fade {
        ModelDepthFade::InheritWorldFog => world_fog_planes.map(|planes| planes[1]),
        ModelDepthFade::Disabled => None,
        ModelDepthFade::WorldRaw { fog, .. } => Some(fog.planes.far_raw as f32 / 256.0),
        ModelDepthFade::Linear { far, .. } => Some(far),
        ModelDepthFade::FrontendFixedPointLinear { far_raw, .. } => Some(far_raw as f32 / 100.0),
    }
}

#[derive(Clone, Copy)]
pub(super) enum ModelTreeViewPolicy {
    Intrinsic,
    Geometry(ModelTreeView),
    Painter {
        view: ModelTreeView,
        composition: ModelTreePainterComposition,
    },
}

impl ModelTreeViewPolicy {
    pub(super) fn view(self) -> Option<ModelTreeView> {
        match self {
            Self::Intrinsic => None,
            Self::Geometry(view) | Self::Painter { view, .. } => Some(view),
        }
    }

    pub(super) fn selection(
        self,
        orientation: [[f32; 3]; 3],
        position: [f32; 3],
        scale: f32,
    ) -> ModelViewSelection {
        self.view()
            .map(|view| view.selection(orientation, position, scale))
            .unwrap_or(ModelViewSelection::IntrinsicAllBranches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_far_radius_rejection_is_inclusive_and_header_bypass_is_local() {
        let view = ModelTreeView {
            position: [0.0; 3],
            forward: [0.0, 0.0, -1.0],
        };
        let scale = 100.0 / 256.0;
        assert!(view.admits_model(0, 256, [0.0, 0.0, -22.0 + 1.0 / 256.0], scale, Some(21.0)));
        assert!(!view.admits_model(0, 256, [0.0, 0.0, -22.0], scale, Some(21.0)));
        assert!(!view.admits_model(0, 256, [0.0, 0.0, -23.0], scale, Some(21.0)));
        assert!(view.admits_model(0x20, 256, [0.0, 0.0, -23.0], scale, Some(21.0)));
        assert!(view.admits_model(0, 256, [0.0, 0.0, -23.0], scale, None));
        assert!(view.admits_model(0, 512, [0.0, 0.0, -22.0], -scale, Some(21.0)));
    }

    #[test]
    fn model_far_plane_uses_explicit_units_and_does_not_normalize_reversed_planes() {
        let world = Some([13.0, 21.0]);
        assert_eq!(
            model_far_plane(ModelDepthFade::InheritWorldFog, world),
            Some(21.0)
        );
        assert_eq!(model_far_plane(ModelDepthFade::Disabled, world), None);
        assert_eq!(
            model_far_plane(
                ModelDepthFade::Linear {
                    near: 7.0,
                    far: 6.0,
                    color: [0.0; 3]
                },
                world
            ),
            Some(6.0)
        );
        assert_eq!(
            model_far_plane(
                ModelDepthFade::FrontendFixedPointLinear {
                    near_raw: 700,
                    far_raw: 600,
                    color: [0.0; 3]
                },
                None
            ),
            Some(6.0)
        );
    }

    #[test]
    fn view_origin_undoes_scale_translation_rotation_and_local_reflection() {
        let view = ModelTreeView {
            position: [10.0, 20.0, 30.0],
            forward: [0.0, 0.0, -1.0],
        };
        let origin = [11.0, 22.0, 34.0];
        for (orientation, expected) in [
            (
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [256, 512, 1024],
            ),
            (
                [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
                [-1024, 512, 256],
            ),
            (
                [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                [-256, 512, 1024],
            ),
        ] {
            assert_eq!(
                view.selection(orientation, origin, 100.0 / 256.0),
                ModelViewSelection::Retail {
                    local_origin_from_camera_raw: expected,
                }
            );
        }
    }
}
