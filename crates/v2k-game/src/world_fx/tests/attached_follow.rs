use super::*;
use crate::entity_collision_state::RetailRuntimeValue;

fn follow_from_models(particle: &mut WorldParticle, models: Option<&[EntityCollisionModel]>) {
    let owner = models.map_or(RetailRuntimeValue::Unresolved, |models| {
        RetailRuntimeValue::Known(
            models
                .iter()
                .find(|model| Some(model.entity_id) == particle.attached_owner_handle)
                .map(ParticleAttachmentOwner::from_collision_model),
        )
    });
    super::apply_attached_owner_follow(particle, owner)
}

#[test]
fn attached_follow_gate_covers_exactly_classes_83_84_86() {
    for class in 0..PARTICLE_DESCRIPTORS.len() as u8 {
        assert_eq!(
            particle_uses_attached_follow_update(class),
            matches!(class, 83 | 84 | 86),
            "class{class} attached-follow gate"
        );
    }
}

#[test]
fn attached_emission_stores_owner_link_offset_and_zero_frame_class() {
    let mut fx = WorldFx::new();
    let birth = fx
        .emit_attached_static_particle_raw(AttachedStaticEmission {
            target_handle: 7,
            position_world: [4.0, 2.0, 5.0],
            offset_raw: [90, 180, 270],
            particle_class: 84,
        })
        .expect("empty pool accepts the attached allocation");
    assert_eq!(birth.particle_class, 84);
    let particles = fx.test_particles_in_virgin_birth_order();
    assert_eq!(particles.len(), 1);
    let attached = particles[0];
    assert_eq!(attached.source_class, 84);
    assert_eq!(attached.position, [4.0, 2.0, 5.0]);
    assert_eq!(attached.velocity, [0.0; 3]);
    assert_eq!(attached.owner_id, Some(7));
    assert_eq!(attached.attached_owner_handle, Some(7));
    assert_eq!(attached.attached_offset_raw, [90, 180, 270]);
    assert_eq!(attached.sprite_id, 0);
    assert_eq!(
        attached.lifetime_ticks,
        particle_descriptor(84).unwrap().lifetime_ticks()
    );
}

fn attached_test_particle(owner: Option<u32>) -> WorldParticle {
    let spawn = invisible_descriptor_particle_spawn(84, [0; 3], owner);
    let mut particle = world_particle([1.0, 2.0, 3.0], spawn);
    particle.attached_owner_handle = owner;
    particle.attached_offset_raw = [256, 512, 768];
    particle
}

#[test]
fn attached_follow_copies_owner_center_without_rotated_state() {
    let mut particle = attached_test_particle(Some(7));
    let models = [collision_entity(7, [10.0, 20.0, 30.0], 100)];
    follow_from_models(&mut particle, Some(&models));
    assert_eq!(particle.position, [10.0, 20.0, 30.0]);
}

#[test]
fn attached_follow_rotates_offset_by_owner_basis_with_rotated_state() {
    let mut particle = attached_test_particle(Some(7));
    let models = [EntityCollisionModel {
        entity_id: 7,
        center_world: [10.0, 20.0, 30.0],
        radius_raw: 100,
        model_id: 0,
        // 90-degree yaw: world X takes model Z, world Z takes model X.
        orientation_world_from_model: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        anim_vars: AnimVars::default(),
        state_flags_at_0x08: Some(ATTACHED_FOLLOW_ROTATED_STATE_BIT),
        capability_flags_at_0x64: 0,
    }];
    follow_from_models(&mut particle, Some(&models));
    // Offset [256, 512, 768] raw is [1.0, 2.0, 3.0] world, rotated to [3, 2, 1].
    assert_eq!(particle.position, [13.0, 22.0, 31.0]);
}

#[test]
fn attached_follow_marks_record_when_owner_is_gone() {
    let mut particle = attached_test_particle(Some(7));
    follow_from_models(
        &mut particle,
        Some(&[collision_entity(9, [10.0, 20.0, 30.0], 100)]),
    );
    assert!(particle.pending_destruction);
}

#[test]
fn attached_follow_keeps_position_without_owner_link_or_context_or_state() {
    let mut particle = attached_test_particle(None);
    follow_from_models(
        &mut particle,
        Some(&[collision_entity(7, [10.0, 20.0, 30.0], 100)]),
    );
    assert_eq!(particle.position, [1.0, 2.0, 3.0]);

    let mut particle = attached_test_particle(Some(7));
    follow_from_models(&mut particle, None);
    assert_eq!(particle.position, [1.0, 2.0, 3.0]);

    let mut particle = attached_test_particle(Some(7));
    let mut model = collision_entity(7, [10.0, 20.0, 30.0], 100);
    model.state_flags_at_0x08 = None;
    follow_from_models(&mut particle, Some(&[model]));
    assert_eq!(particle.position, [1.0, 2.0, 3.0]);
}

#[test]
fn native_attached_follow_preserves_q31_term_narrowing_and_signed_word_wrap() {
    use crate::common_mover::type9_attitude::Type9BodyBasis;
    let mut particle = attached_test_particle(Some(7));
    particle.attached_offset_raw = [256, -512, 768];
    let owner = ParticleAttachmentOwner {
        entity_id: 7,
        position_raw: [32760, -10, -32760],
        state_flags_at_0x08: Some(ATTACHED_FOLLOW_ROTATED_STATE_BIT),
        capability_flags_at_0x64: 0,
        basis: ParticleAttachmentBasis::Native(Type9BodyBasis {
            lateral: [2147352576, 0, 0],
            up: [0, 2147352576, 0],
            forward: [0, 0, 2147352576],
        }),
    };
    super::apply_attached_owner_follow(&mut particle, RetailRuntimeValue::Known(Some(owner)));
    assert_eq!(
        world_position_to_raw(particle.position),
        [-32521, -522, -31993]
    );
}
