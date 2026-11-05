//! `FUN_00413BE0`: the transform-only model pass for an actor that has queued
//! shots without muzzles.
//!
//! `FUN_00413BE0` copies the view, clears it to an identity view at the world
//! origin (identity word zero, Q31 one on the basis diagonal) and runs the
//! actor's current model with its position, body basis and type callbacks
//! through `FUN_00458B10`/`FUN_00458B40`. With that view `FUN_00464E60` takes
//! the transform-only table `0x004D44E0` for every node, with no node
//! rejection: its face, edge and billboard handlers resolve each vertex they
//! name, culled or not, so every reached type-14 vertex runs the actor's
//! external-frame callback (`FUN_0040A9F0`: Sub-H, then Sub-E's
//! `FUN_00424F20` muzzle stamp, then the terminal relation/self suffix).
//! Nothing is drawn. The callbacks are the draw's own owners, so Sub-H records
//! and the shared random words advance exactly as a draw's would.
//!
//! The table's handlers are the materializer's vertex resolution with every
//! face admitted: faces `FUN_004580C0`, `FUN_004582D0`, `FUN_00458170`,
//! `FUN_004583B0`, `FUN_00458580`, `FUN_00458790`, `FUN_00458630`,
//! `FUN_00458870`; edges `FUN_00457FC0`, `FUN_00458040`; the anchor
//! `FUN_00458A40` and group vertex `FUN_00458AA0`; and `FUN_00458AF0`,
//! `FUN_00458A90`, `FUN_00458B00`, which skip their operands.

use super::external_frame::ModelTreeExternalFrame;
use super::{
    build_child_linked_slots, inherited_child_vars, ModelJitterOwner, ModelTreeRootLinkPolicy,
    ModelTreeView, WorldModelVertexResolver,
};
use crate::actor_emitter_external_frame::{
    ActorEmitterExternalFrameBoundary, ActorEmitterExternalFramePoint,
    ActorEmitterExternalFramePresentation,
};
use crate::actor_external_frame_fallback::{
    ActorExternalFrameFallback, ActorExternalFrameFallbackBoundary,
};
use crate::native_model_frame::{NativeModelFrame, NativeWorldViewport};
use crate::resource_cache::ResourceCache;
use crate::sub_h_external_frame::{SubHPresentation, SubHRuntimeState};
use std::cell::RefCell;
use std::collections::HashMap;
use v2k_formats::collision::{ProjectileEmitterDescriptor, SubHExternalFrameDescriptor};
use v2k_formats::models::{AnimVars, LinkedModelSlots, ModelMaterializationContext};
use v2k_render::{ModelTransform, WorldSurfaceProjection};

/// `FUN_00413BE0`'s view: identity word zero, origin zero, Q31 identity axes.
const IDENTITY_VIEWPORT: NativeWorldViewport = NativeWorldViewport {
    origin_raw: [0; 3],
    axes_q31: [[i32::MAX, 0, 0], [0, i32::MAX, 0], [0, 0, i32::MAX]],
    identity: true,
};

/// The draw's hierarchy depth for a world actor.
const ACTOR_MODEL_DEPTH: u8 = 8;

/// One actor's inputs to `FUN_00413BE0`.
pub struct ActorTransformPass<'a> {
    pub cache: &'a ResourceCache,
    /// The model `FUN_00413BE0` selects from `+A8..+AE` (the drawn one).
    pub model_id: usize,
    pub vars: &'a AnimVars,
    pub origin_raw: [i16; 3],
    /// Body basis rows `+0C`, `+18`, `+24`: lateral, up, forward.
    pub body_axes_q31: [[i32; 3]; 3],
    /// Terrain for Sub-H feet and world-pinned vertices.
    pub surface: Option<WorldSurfaceProjection<'a>>,
    pub retail_tick: u32,
    /// The live Sub-H records and their descriptor, when the type has Sub-H.
    pub sub_h: Option<(&'a mut SubHRuntimeState, &'a SubHExternalFrameDescriptor)>,
    pub emitter: Option<ProjectileEmitterDescriptor>,
    /// The terminal suffix (`FUN_0040A980`), when its domain is proven.
    pub fallback: Option<ActorExternalFrameFallback<'a>>,
}

/// A selector the pass reached but the port cannot resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorTransformPassBoundary {
    ModelMissing { model_id: usize },
    Emitter(ActorEmitterExternalFrameBoundary),
    Fallback(ActorExternalFrameFallbackBoundary),
}

impl ActorTransformPass<'_> {
    /// Run the pass; returns the muzzle `FUN_00424F20` stamped per Sub-E
    /// selector (the last one reached wins, as it overwrites the queue).
    pub fn run(self) -> Result<[Option<[i16; 3]>; 2], ActorTransformPassBoundary> {
        let Self {
            cache,
            model_id,
            vars,
            origin_raw,
            body_axes_q31,
            surface,
            retail_tick,
            sub_h,
            emitter,
            mut fallback,
        } = self;
        let root = cache
            .global_model(model_id)
            .ok_or(ActorTransformPassBoundary::ModelMissing { model_id })?;
        let frame = NativeModelFrame::from_actor(IDENTITY_VIEWPORT, origin_raw, body_axes_q31);
        let mut stamps = [None; 2];
        let mut emitter_boundary = None;
        // A type without Sub-H reaches Sub-E from selector zero.
        let mut no_records = SubHRuntimeState::new(0).expect("an empty Sub-H is valid");
        let no_descriptor = SubHExternalFrameDescriptor {
            completion_sound_id: None,
            records: Vec::new(),
        };
        let (runtime, descriptor) = sub_h.unwrap_or((&mut no_records, &no_descriptor));
        let fallback_boundary = {
            let mut stamp = |point: ActorEmitterExternalFramePoint| {
                stamps[usize::from(point.emitter_index)] = Some(point.position_words);
            };
            let origin_world = origin_raw.map(|value| f32::from(value) / 256.0);
            let presentation = SubHPresentation {
                runtime,
                descriptor,
                model_records: &root.records,
                retail_tick,
                origin_raw,
                origin_world,
                body_axes_q31: Some(body_axes_q31),
                native_context: Some((frame.clone(), IDENTITY_VIEWPORT)),
                // Reborrowed so the presentation can live on this frame.
                fallback: fallback
                    .as_mut()
                    .map(|fallback| ActorExternalFrameFallback {
                        reservations: fallback.reservations,
                        own_origin_raw: fallback.own_origin_raw,
                        relation_origin_raw: fallback.relation_origin_raw,
                        random_u16: &mut *fallback.random_u16,
                        last_boundary: None,
                    }),
                emitter: emitter.map(|descriptor| ActorEmitterExternalFramePresentation {
                    descriptor,
                    root_model: root,
                    root_vars: vars,
                    frame,
                    viewport: IDENTITY_VIEWPORT,
                    stamp_origin: &mut stamp,
                    last_boundary: &mut emitter_boundary,
                    current_node_owned: true,
                    current_source_points_view: None,
                }),
            };
            let mut walk = TransformWalk {
                cache,
                surface,
                origin_world,
                external: ModelTreeExternalFrame::SubH(presentation),
            };
            walk.node(model_id, ACTOR_MODEL_DEPTH, None, vars)?;
            match walk.external {
                ModelTreeExternalFrame::SubH(presentation) => presentation
                    .fallback
                    .and_then(|fallback| fallback.last_boundary),
                _ => None,
            }
        };
        if let Some(boundary) = emitter_boundary {
            return Err(ActorTransformPassBoundary::Emitter(boundary));
        }
        if let Some(boundary) = fallback_boundary {
            return Err(ActorTransformPassBoundary::Fallback(boundary));
        }
        Ok(stamps)
    }
}

struct TransformWalk<'a> {
    cache: &'a ResourceCache,
    surface: Option<WorldSurfaceProjection<'a>>,
    origin_world: [f32; 3],
    external: ModelTreeExternalFrame<'a>,
}

impl TransformWalk<'_> {
    /// `FUN_00464E60` with the transform-only table: the node's command
    /// stream resolves every vertex it names; `0x0E` children recurse with
    /// their own frames.
    fn node(
        &mut self,
        model_id: usize,
        depth: u8,
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) -> Result<(), ActorTransformPassBoundary> {
        let cache = self.cache;
        let model = cache
            .global_model(model_id)
            .ok_or(ActorTransformPassBoundary::ModelMissing { model_id })?;
        let terrain = self.surface.map(|surface| surface.terrain());
        self.external
            .prepare_node_surface(model, vars, linked, true, terrain);
        let native_view = self.external.native_view_selection();
        let native_context = self.external.native_context();
        let surface = self.surface;
        let owner = RefCell::new(&mut self.external);
        let resolve_external = |parameters, node_view: &mut dyn FnMut(u16) -> Option<[i32; 3]>| {
            owner
                .borrow_mut()
                .resolve_parameter_world(parameters, surface, node_view)
        };
        // Only the callbacks and the native frames are kept; the float
        // placement of the resolved vertices is never used.
        let transform = ModelTransform {
            orientation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            position: self.origin_world,
            scale: 1.0,
        };
        let resolver = WorldModelVertexResolver {
            surface,
            transform,
            view: Some(ModelTreeView {
                position: [0.0; 3],
                forward: [0.0, 0.0, 1.0],
            }),
            native_view,
            native_edges_required: false,
            native_context,
            model,
            external_owner: Some(&resolve_external),
            external_points: RefCell::new(HashMap::new()),
            native_vertex_failures: RefCell::new(Vec::new()),
            admit_every_face: true,
            // FUN_0046DF70 would draw words here; no pass is lent the
            // generator yet, so a reached tf3 slot stops with its type.
            jitter: ModelJitterOwner::new(model_id, None, transform),
            fixed_world_point_raw: None,
        };
        let context = ModelMaterializationContext {
            vars,
            linked,
            view_selection: native_view
                .unwrap_or(v2k_formats::models::ModelViewSelection::IntrinsicAllBranches),
            vertex_resolver: Some(&resolver),
        };
        let materialized = model.materialize_with_context(context);
        let children: Vec<_> = if depth == 0 {
            Vec::new()
        } else {
            materialized
                .instances
                .iter()
                .filter(|instance| usize::from(instance.model_id) != model_id)
                .map(|instance| {
                    let links = build_child_linked_slots(
                        model,
                        linked,
                        instance,
                        vars,
                        ModelTreeRootLinkPolicy::Authored,
                        context.vertex_resolver,
                    );
                    (instance.clone(), links)
                })
                .collect()
        };
        drop(resolver);
        drop(owner);
        for (instance, links) in children {
            let child_vars = inherited_child_vars(vars, &instance);
            let parent = self
                .external
                .begin_child(model, &instance, vars, true, true, terrain);
            if self.external.native_child_is_owned(&parent) {
                self.node(
                    usize::from(instance.model_id),
                    depth - 1,
                    Some(&links),
                    &child_vars,
                )?;
            }
            self.external.end_child(parent);
        }
        Ok(())
    }
}
