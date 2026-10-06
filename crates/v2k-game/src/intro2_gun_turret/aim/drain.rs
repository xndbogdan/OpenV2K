//! 11400/13BE0 model callbacks ->424F20 muzzle stamps ->14870/4E770 FIFO.

use super::*;
use crate::{
    common_mover::type9_attitude::Type9BodyBasis,
    ordinary_type47_shot_math::{drain_transient_with_method_row, Type47ShotMathError},
    projectile_emitter::projectile_class_row,
    resource_cache::ResourceCache,
    world_fx::{DescriptorParticleRequest, ParticleEnvironment, ParticleOwnerAtBirth},
};
use v2k_formats::models::{
    AnimVars, LinkedModelSlots, MaterializedModel, ModelEntry, ModelFaceCull,
    ModelMaterializationContext, ModelPainterOp,
};

pub use crate::intro2_native_ballistic_aim::NativeBallisticShotDrainOutcome as Intro2GunTurretShotDrainOutcome;

const IMPACT_SUPPRESSION_STATE_BIT: u32 = 0x8000_0000;
const PRESENTATION_DETAIL_STATE_BIT: u32 = 0x0200_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2GunTurretShotDrainError {
    Allocation,
    Queue,
    State,
    BodyBasis,
    ModelUnavailable { model_id: usize },
    ModelBoundary(&'static str),
    MalformedRequest { index: usize },
    ShotMath(Type47ShotMathError),
}

/// Queues outlive the task which authored them. Model callbacks stamp every
/// matching selector before11400 drains in FIFO order; no callback restarts
/// the task or consumes actor RNG. A rejected particle birth still consumes
/// the corresponding command, as in the other native presentation drains.
pub fn drain_intro2_gun_turret_shots(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    id: u32,
    resources: &ResourceCache,
    environment: ParticleEnvironment<'_>,
    retail_tick: u32,
) -> Result<Intro2GunTurretShotDrainOutcome, Intro2GunTurretShotDrainError> {
    if !crate::intro2_gun_turret::intro2_gun_turret_manager_allocation_authenticates(manager, id) {
        return Err(Intro2GunTurretShotDrainError::Allocation);
    }
    let entity = manager
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Intro2GunTurretShotDrainError::Allocation)?;
    let queue = entity
        .intro2_gun_turret_aim_runtime
        .as_ref()
        .ok_or(Intro2GunTurretShotDrainError::Queue)?;
    if !queue.authenticates(entity) {
        return Err(Intro2GunTurretShotDrainError::Queue);
    }
    let descriptor = queue.projectile_descriptor();
    let source_type = entity.entity_type as u8;
    if queue.queued_shot_count() == 0 {
        return Ok(Intro2GunTurretShotDrainOutcome {
            consumed_requests: 0,
            materialized_particle_classes: Vec::new(),
        });
    }
    let suppression = match entity
        .collision
        .state_flags_at_0x08
        .masked(IMPACT_SUPPRESSION_STATE_BIT)
    {
        RetailRuntimeValue::Known(bits) => bits != 0,
        RetailRuntimeValue::Unresolved => return Err(Intro2GunTurretShotDrainError::State),
    };
    // Offline4147A0 initializes every command marker to1.11400 forces13BE0
    // iff current presentation detail lowers its threshold from2 to1. The
    // host publishes the current presentation flags before this drain.
    let muzzles = match entity
        .collision
        .state_flags_at_0x08
        .masked(PRESENTATION_DETAIL_STATE_BIT)
    {
        RetailRuntimeValue::Known(0) => [None; 2],
        RetailRuntimeValue::Known(_) => resolve_muzzles(entity, resources, retail_tick)?,
        RetailRuntimeValue::Unresolved => return Err(Intro2GunTurretShotDrainError::State),
    };
    // EXE4D0380: method12={leading1,speed6000,particle49,entity0}.
    // Type97's command retains its explicit4000 override during the drain.
    let method = projectile_class_row(u32::from(descriptor.projectile_method))
        .ok_or(Intro2GunTurretShotDrainError::Queue)?;
    let mut solutions = Vec::with_capacity(queue.queued_shot_count());
    for (index, request) in queue.transient_shots().iter().copied().enumerate() {
        if request.source_handle != id
            || request.owner_handle != id
            || request.projectile_method != u32::from(descriptor.projectile_method)
            || request.emitter_selector > 1
            || (request.emitter_selector != 0 && descriptor.alternate_emitter_raw == 0)
            || request.speed_field
                != GenericEmitterSpeedField::Explicit(descriptor.speed_override_raw)
            || request.auxiliary
        {
            return Err(Intro2GunTurretShotDrainError::MalformedRequest { index });
        }
        // An unstamped offline command follows11400's explicit marker1
        // source-position branch. Missing/unsupported models on a required
        // detailed traversal remain errors instead of changing that branch.
        let position =
            muzzles[usize::from(request.emitter_selector)].unwrap_or(entity.position_raw());
        solutions.push(
            drain_transient_with_method_row(request, position, entity.velocity_raw(), method)
                .map_err(Intro2GunTurretShotDrainError::ShotMath)?,
        );
    }
    let mut outcome = Intro2GunTurretShotDrainOutcome {
        consumed_requests: 0,
        materialized_particle_classes: Vec::new(),
    };
    for solution in solutions {
        //40A60 cannot invoke an actor callback; the allocated source/queue
        //remains the same across this materialization and list-node removal.
        let particle_request = DescriptorParticleRequest {
            source_class: method.particle_class,
            position_raw: solution.position_raw,
            velocity_raw: solution.velocity_raw,
            owner: Some(ParticleOwnerAtBirth {
                entity_id: id,
                entity_type: source_type,
            }),
            suppresses_impact_damage: suppression,
        };
        //4E770 method16 reaches410B0's unchanged-class50 fallthrough to
        //40A60, with no defensive-bolt underwater descriptor replacement.
        let particle_class = if descriptor.projectile_method == 16 {
            world_fx
                .materialize_descriptor_particle_request(particle_request, environment, retail_tick)
                .map(|_| method.particle_class)
        } else {
            world_fx
                .materialize_combat_projectile_410b0(particle_request, environment, retail_tick)
                .map(|birth| birth.particle_class)
        };
        if let Some(particle_class) = particle_class {
            outcome.materialized_particle_classes.push(particle_class);
        }
        let entity = manager
            .entity_mut(id)
            .ok_or(Intro2GunTurretShotDrainError::Allocation)?;
        entity
            .intro2_gun_turret_aim_runtime
            .as_mut()
            .ok_or(Intro2GunTurretShotDrainError::Queue)?
            .pop_front();
        outcome.consumed_requests += 1;
    }
    Ok(outcome)
}

const IDENTITY: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

/// Intrinsic Section8 traversal, without scene mirroring or GL handedness.
///40D350 wraps callback coordinates relative to the actor context; the final
///signed-word coordinates are equivalent to wrapping the resulting world XYZ.
fn resolve_muzzles(
    entity: &Entity,
    resources: &ResourceCache,
    tick: u32,
) -> Result<[Option<[i16; 3]>; 2], Intro2GunTurretShotDrainError> {
    let RetailRuntimeValue::Known(body) = entity.physical_body_basis_q31() else {
        return Err(Intro2GunTurretShotDrainError::BodyBasis);
    };
    let model = entity
        .model_index
        .ok_or(Intro2GunTurretShotDrainError::ModelBoundary("active model"))?;
    let mut walker = MuzzleWalk {
        resources,
        descriptor: profile_for_entity(entity)
            .ok_or(Intro2GunTurretShotDrainError::Allocation)?
            .emitter(),
        body,
        position: entity.position_raw(),
        muzzles: [None; 2],
        ancestors: Vec::new(),
    };
    walker.visit(
        model,
        [0.; 3],
        IDENTITY,
        None,
        &entity.presentation_anim_vars(tick),
    )?;
    Ok(walker.muzzles)
}

struct MuzzleWalk<'a> {
    resources: &'a ResourceCache,
    descriptor: ProjectileEmitterDescriptor,
    body: Type9BodyBasis,
    position: [i16; 3],
    muzzles: [Option<[i16; 3]>; 2],
    ancestors: Vec<usize>,
}

impl MuzzleWalk<'_> {
    fn visit(
        &mut self,
        model_id: usize,
        origin: [f64; 3],
        basis: [[f64; 3]; 3],
        linked: Option<&LinkedModelSlots>,
        vars: &AnimVars,
    ) -> Result<(), Intro2GunTurretShotDrainError> {
        use Intro2GunTurretShotDrainError as Error;
        if self.ancestors.len() >= 16 || self.ancestors.contains(&model_id) {
            return Err(Error::ModelBoundary("recursive hierarchy"));
        }
        let resources = self.resources;
        let model = resources
            .global_model(model_id)
            .ok_or(Error::ModelUnavailable { model_id })?;
        let context = ModelMaterializationContext::intrinsic(vars, linked);
        let geometry = model.materialize_with_context(context);
        if geometry.has_view_commands {
            return Err(Error::ModelBoundary("view-dependent muzzle traversal"));
        }
        self.ancestors.push(model_id);
        let mut visited = vec![false; geometry.vertices.len()];
        for operation in &geometry.painter_program {
            match operation {
                ModelPainterOp::Face { triangle_range, .. } => {
                    for triangle in triangle_range.clone() {
                        let indices = geometry
                            .triangles
                            .get(triangle)
                            .ok_or(Error::ModelBoundary("face indices"))?;
                        if indices
                            .iter()
                            .any(|&i| geometry.vertex_type_flags.get(usize::from(i)) == Some(&14))
                            && geometry.face_cull.get(triangle)
                                != Some(&ModelFaceCull::AlwaysVisible)
                        {
                            return Err(Error::ModelBoundary("view-dependent external face"));
                        }
                        for &index in indices {
                            self.vertex(
                                model,
                                &geometry,
                                usize::from(index),
                                &mut visited,
                                origin,
                                basis,
                                context,
                            )?;
                        }
                    }
                }
                ModelPainterOp::Edge { edge_index, .. } => {
                    let edge = geometry
                        .edges
                        .get(*edge_index)
                        .ok_or(Error::ModelBoundary("edge index"))?;
                    for index in edge.vertices {
                        self.vertex(
                            model,
                            &geometry,
                            usize::from(index),
                            &mut visited,
                            origin,
                            basis,
                            context,
                        )?;
                    }
                }
                ModelPainterOp::Billboard {
                    billboard_index, ..
                } => {
                    let billboard = geometry
                        .billboards
                        .get(*billboard_index)
                        .ok_or(Error::ModelBoundary("billboard index"))?;
                    self.vertex(
                        model,
                        &geometry,
                        usize::from(billboard.vertex),
                        &mut visited,
                        origin,
                        basis,
                        context,
                    )?;
                }
                ModelPainterOp::Instance { instance_index } => {
                    let child = geometry
                        .instances
                        .get(*instance_index)
                        .ok_or(Error::ModelBoundary("child index"))?;
                    let attach = child
                        .attach_pos
                        .ok_or(Error::ModelBoundary("child attachment"))?;
                    let child_origin = add(origin, apply(basis, attach));
                    let child_basis = multiply(basis, child.orientation);
                    let mut instance_vars = vars.clone();
                    instance_vars.registers = child.registers;
                    let context = ModelMaterializationContext::intrinsic(&instance_vars, linked);
                    let child_linked: LinkedModelSlots = child
                        .linked_slots
                        .iter()
                        .map(|slot| {
                            std::array::from_fn(|odd| {
                                model
                                    .resolve_slot_with_context(*slot ^ odd as u16, context)
                                    .map(|mut resolved| {
                                        resolved.position_raw = apply(
                                            transpose(child.orientation),
                                            sub(resolved.position_raw, attach),
                                        );
                                        resolved
                                    })
                            })
                        })
                        .collect();
                    let mut child_vars = vars.clone();
                    child_vars.registers.fill(0);
                    child_vars.registers[..4].copy_from_slice(&child.registers[..4]);
                    self.visit(
                        usize::from(child.model_id),
                        child_origin,
                        child_basis,
                        Some(&child_linked),
                        &child_vars,
                    )?;
                }
                ModelPainterOp::BeginGroup {
                    referenced_slots, ..
                } => {
                    // C6 in the real barrel model resolves44 even when its
                    // conditional face was skipped. Queue ordering is later;
                    // the reference callback itself happens here.
                    for &slot in referenced_slots {
                        let record = model
                            .records
                            .get(usize::from(slot >> 1))
                            .ok_or(Error::ModelBoundary("group source slot"))?;
                        match record[0] {
                            0 => {}
                            14 if record[2..] == [0, 0] && matches!(record[1], 0 | 1) => {
                                self.stamp_muzzle(
                                    model,
                                    record[1] as usize,
                                    origin,
                                    basis,
                                    context,
                                )?;
                            }
                            _ => return Err(Error::ModelBoundary("generated group source slot")),
                        }
                    }
                }
                ModelPainterOp::EndGroup => {}
            }
        }
        self.ancestors.pop();
        Ok(())
    }

    fn vertex(
        &mut self,
        model: &ModelEntry,
        geometry: &MaterializedModel,
        index: usize,
        visited: &mut [bool],
        origin: [f64; 3],
        basis: [[f64; 3]; 3],
        context: ModelMaterializationContext<'_>,
    ) -> Result<(), Intro2GunTurretShotDrainError> {
        use Intro2GunTurretShotDrainError as Error;
        let seen = visited
            .get_mut(index)
            .ok_or(Error::ModelBoundary("vertex index"))?;
        if *seen {
            return Ok(());
        }
        *seen = true;
        if geometry.vertex_type_flags.get(index) != Some(&14) {
            return Ok(());
        }
        //The intrinsic resolver retains14's authored callback operands. Only
        //the native E index0/1 domain is admitted; no SubH/M/N dispatch exists.
        let operands = geometry.vertices[index];
        let selector = if operands == [0., 0., 0.] {
            0
        } else if operands == [1., 0., 0.] {
            1
        } else {
            return Err(Error::ModelBoundary("external callback operands"));
        };
        self.stamp_muzzle(model, selector, origin, basis, context)
    }

    fn stamp_muzzle(
        &mut self,
        model: &ModelEntry,
        selector: usize,
        origin: [f64; 3],
        basis: [[f64; 3]; 3],
        context: ModelMaterializationContext<'_>,
    ) -> Result<(), Intro2GunTurretShotDrainError> {
        use Intro2GunTurretShotDrainError as Error;
        let slot = if selector == 0 {
            self.descriptor.raw_word_at_0x12
        } else {
            self.descriptor.alternate_emitter_raw
        };
        let muzzle = model
            .resolve_slot_with_context(slot, context)
            .ok_or(Error::ModelBoundary("muzzle slot"))?;
        //Use intrinsic model geometry and mount composition. That shared API
        //currently retains f64 mount trigonometry; this is its existing
        //precision boundary, not a claim of bit-exact671F0 angle-table math.
        let local = add(origin, apply(basis, muzzle.position_raw)).map(|v| v as i32);
        let columns = [self.body.lateral, self.body.up, self.body.forward];
        self.muzzles[selector] = Some(std::array::from_fn(|axis| {
            let delta = (0..3).fold(0_i32, |sum, column| {
                sum.wrapping_add(
                    ((i64::from(local[column]) * i64::from(columns[column][axis])) >> 31) as i32,
                )
            });
            self.position[axis].wrapping_add(delta as i16)
        }));
        Ok(())
    }
}

fn apply(matrix: [[f64; 3]; 3], point: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|r| (0..3).map(|c| matrix[r][c] * point[c]).sum())
}
fn multiply(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| (0..3).map(|k| left[r][k] * right[k][c]).sum()))
}
fn transpose(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    std::array::from_fn(|r| std::array::from_fn(|c| matrix[c][r]))
}
fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| left[i] + right[i])
}
fn sub(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| left[i] - right[i])
}

#[cfg(test)]
mod tests;
