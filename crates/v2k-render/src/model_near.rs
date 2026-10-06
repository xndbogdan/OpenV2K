//! Original-primitive admission before hardware clipping. Retail projectors
//! mark raw signed Z < 0x40, and the constructor drops the complete face.
//! The float camera adapter is retained; coordinate units belong to the scene,
//! independently of model size, lighting and near/far fog-table selection.

use v2k_formats::models::{ModelFaceVertices, ModelVertexProjection};

use crate::renderer::ModelNearClip;

impl ModelNearClip {
    pub(crate) fn accepts_view_depth(self, depth: f32) -> bool {
        if !depth.is_finite() {
            return false;
        }
        let units = match self {
            Self::Camera => return true,
            Self::RetailWorld => 256.0,
            Self::RetailFrontend => 100.0,
        };
        // Match the existing CPU projector's rounding at the float/raw
        // boundary. This is not a replacement for retail's Q31 view matrix.
        (depth * units).round() >= 64.0
    }
}

pub(crate) fn world_view_depth(point: [f32; 3], camera: [f32; 3], back: [f32; 3]) -> f32 {
    -(0..3)
        .map(|i| (point[i] - camera[i]) * back[i])
        .sum::<f32>()
}

/// Resolve the projector's dependency graph separately from model-space
/// geometry. FUN_0046DC00's screen midpoint inherits rejection from either
/// source; tf5's ordinary 3-D midpoint instead projects its own position.
pub(crate) fn vertex_admission(
    policy: ModelNearClip,
    points: &[Option<[f32; 3]>],
    projections: &[ModelVertexProjection],
    camera: [f32; 3],
    back: [f32; 3],
) -> Vec<bool> {
    let direct = points
        .iter()
        .map(|point| {
            point.is_some_and(|point| {
                policy.accepts_view_depth(world_view_depth(point, camera, back))
            })
        })
        .collect::<Vec<_>>();
    if policy == ModelNearClip::Camera {
        return direct;
    }
    fn resolve(
        index: usize,
        direct: &[bool],
        projections: &[ModelVertexProjection],
        visiting: &mut [bool],
        cache: &mut [Option<bool>],
    ) -> bool {
        let Some(cached) = cache.get(index) else {
            return false;
        };
        if let Some(value) = *cached {
            return value;
        }
        if visiting[index] {
            return false;
        }
        visiting[index] = true;
        let value = match projections.get(index) {
            Some(ModelVertexProjection::ScreenMidpoint([a, b])) => {
                resolve(usize::from(*a), direct, projections, visiting, cache)
                    && resolve(usize::from(*b), direct, projections, visiting, cache)
            }
            Some(ModelVertexProjection::Position | ModelVertexProjection::WorldPoint(_)) | None => {
                direct[index]
            }
        };
        visiting[index] = false;
        cache[index] = Some(value);
        value
    }
    let mut visiting = vec![false; points.len()];
    let mut cache = vec![None; points.len()];
    (0..points.len())
        .map(|i| resolve(i, &direct, projections, &mut visiting, &mut cache))
        .collect()
}

/// The original quad's fourth corner remains an admission dependency even
/// for its first GL triangle. Diagnostics without authored provenance keep
/// their submitted triangle as the primitive boundary.
pub(crate) fn face_admitted(
    original: Option<&ModelFaceVertices>,
    triangle: &[u16; 3],
    vertex_admitted: &[bool],
) -> bool {
    original
        .map(ModelFaceVertices::indices)
        .unwrap_or(triangle)
        .iter()
        .all(|&index| vertex_admitted.get(usize::from(index)) == Some(&true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_63_and_64_use_explicit_scene_units_even_without_fog() {
        for (policy, units) in [
            (ModelNearClip::RetailWorld, 256.0),
            (ModelNearClip::RetailFrontend, 100.0),
        ] {
            assert!(!policy.accepts_view_depth(63.0 / units));
            assert!(policy.accepts_view_depth(64.0 / units));
            assert!(!policy.accepts_view_depth(-1.0));
        }
        assert!(ModelNearClip::Camera.accepts_view_depth(0.1));
        assert!(ModelNearClip::RetailWorld.accepts_view_depth(0.5));
        assert!(!ModelNearClip::RetailFrontend.accepts_view_depth(0.5));
    }

    #[test]
    fn original_quad_is_atomic_and_its_mirror_is_independent() {
        let a = ModelFaceVertices::Quad([0, 1, 2, 3]);
        let b = ModelFaceVertices::Quad([4, 5, 6, 7]);
        let accepted = [true, true, true, false, true, true, true, true];
        assert!(!face_admitted(Some(&a), &[0, 1, 2], &accepted));
        assert!(!face_admitted(Some(&a), &[0, 2, 3], &accepted));
        assert!(face_admitted(Some(&b), &[4, 5, 6], &accepted));
        assert!(face_admitted(Some(&b), &[4, 6, 7], &accepted));
        assert!(face_admitted(None, &[0, 1, 2], &accepted));
        assert!(!face_admitted(Some(&a), &[0, 1, 2], &accepted[..3]));
    }

    #[test]
    fn a_surface_clipped_fourth_corner_rejects_both_triangles_in_every_scene() {
        let points = [Some([0.0, 0.0, -2.0]); 3]
            .into_iter()
            .chain([None])
            .collect::<Vec<_>>();
        let quad = ModelFaceVertices::Quad([0, 1, 2, 3]);
        for policy in [
            ModelNearClip::Camera,
            ModelNearClip::RetailWorld,
            ModelNearClip::RetailFrontend,
        ] {
            let accepted = vertex_admission(policy, &points, &[], [0.0; 3], [0.0, 0.0, 1.0]);
            assert_eq!(accepted, [true, true, true, false]);
            assert!(!face_admitted(Some(&quad), &[0, 1, 2], &accepted));
            assert!(!face_admitted(Some(&quad), &[0, 2, 3], &accepted));
        }
    }

    #[test]
    fn admission_uses_true_world_position_and_full_pitched_view_depth() {
        // Horizontal distance would admit this point, but the pitched view
        // places it only 0.12 units in front of the eye.
        let camera = [4.0, 2.0, -3.0];
        let point = [4.0, 1.0, -2.1];
        let back = [0.0, -0.6, -0.8];
        let depth = world_view_depth(point, camera, back);
        assert!((depth - 0.12).abs() < 0.00001);
        assert!(!ModelNearClip::RetailWorld.accepts_view_depth(depth));
        let near = [
            camera[0],
            camera[1] + 0.6 * 63.0 / 256.0,
            camera[2] + 0.8 * 63.0 / 256.0,
        ];
        assert!(
            !ModelNearClip::RetailWorld.accepts_view_depth(world_view_depth(near, camera, back))
        );
    }

    #[test]
    fn projected_midpoints_inherit_both_sources_but_spatial_midpoints_do_not() {
        use ModelVertexProjection::{Position, ScreenMidpoint};
        let points = [32.0, 160.0, 96.0, 96.0, 128.0].map(|raw| Some([0.0, 0.0, -raw / 256.0]));
        let projections = [
            Position,
            Position,
            ScreenMidpoint([0, 1]),
            Position,
            ScreenMidpoint([1, 2]),
        ];
        assert_eq!(
            vertex_admission(
                ModelNearClip::RetailWorld,
                &points,
                &projections,
                [0.0; 3],
                [0.0, 0.0, 1.0]
            ),
            [false, true, false, true, false]
        );
        assert_eq!(
            vertex_admission(
                ModelNearClip::Camera,
                &points,
                &projections,
                [0.0; 3],
                [0.0, 0.0, 1.0]
            ),
            [true; 5]
        );
        let invalid = [
            ScreenMidpoint([1, 1]),
            ScreenMidpoint([0, 0]),
            ScreenMidpoint([0, 8]),
            Position,
            Position,
        ];
        assert_eq!(
            vertex_admission(
                ModelNearClip::RetailWorld,
                &points,
                &invalid,
                [0.0; 3],
                [0.0, 0.0, 1.0]
            ),
            [false, false, false, true, true]
        );
    }
}
