//! Type53 receipt binding for the shared01430 mover.
use super::*;
pub(super) use crate::native_ground_actor::mover::MoverFrame;
use crate::{
    common_mover::target_prelude::CommonMoverTrackedTargetSnapshot,
    wander_near_location::WanderNearPrivateState,
};
pub(super) fn run(
    entity: &mut Entity,
    frame: MoverFrame<'_>,
    target: &mut WanderNearPrivateState,
    tracked_target: RetailRuntimeValue<Option<CommonMoverTrackedTargetSnapshot>>,
    next_random: &mut impl FnMut() -> u32,
) -> Result<bool, Intro2Type53Block> {
    crate::native_ground_actor::mover::run::<super::profile::Type53Profile>(
        entity,
        frame,
        target,
        tracked_target,
        next_random,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_mover::component_dispatch::CommonMoverDispatchMode;
    use crate::common_mover::type9_attitude::Type9BodyBasis;

    #[v2k_test_support::retail_test]
    fn intro2_common_mover_type53_restores_sub_d_prefix_after_late_h_block() {
        let Some((session, metadata)) = super::super::tests::fixture() else {
            return;
        };
        let mut manager = super::super::tests::generic(&session, &metadata);
        let entity = manager.entity_mut(21).unwrap();
        publish_intro2_type53(
            entity,
            &metadata[53],
            &[],
            session.cache.terrain().unwrap(),
            &mut || 0,
        )
        .unwrap();
        let basis = Type9BodyBasis::from_angle_words(0x1800, 0x1000, -0x1400);
        entity.set_rotation_heading_pitch_roll_raw([0x1800, 0x1000, -0x1400]);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(basis);
        entity.sub_h_external_frame_runtime = RetailRuntimeValue::Known(None);
        let before = entity.intro2_type53_runtime.unwrap();
        let position = entity.position_raw();
        let velocity = entity.velocity_raw();
        let mut target = WanderNearPrivateState::tracked_entity(
            [
                position[0].wrapping_add(2048),
                position[1],
                position[2].wrapping_add(1024),
            ],
            0,
        );
        let result = run(
            entity,
            MoverFrame {
                metadata: &metadata[53],
                terrain: session.cache.terrain().unwrap(),
                dispatch_mode: CommonMoverDispatchMode::Normal,
                elapsed_micros: 20_000,
                global_elapsed_micros: 20_000,
            },
            &mut target,
            RetailRuntimeValue::Known(None),
            &mut || panic!("unexpected RNG"),
        );
        assert_eq!(result, Err(Intro2Type53Block::Runtime("Sub-H allocation")));
        let after = entity.intro2_type53_runtime.unwrap();
        assert_ne!(
            after.sub_d_owner, before.sub_d_owner,
            "classifier/cadence prefix survives the later block"
        );
        assert_eq!(
            after.sub_d_owner.classifier_cache().stagger_counter(),
            before
                .sub_d_owner
                .classifier_cache()
                .stagger_counter()
                .wrapping_add(1)
        );
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw()[0],
            0x1800_i16.wrapping_sub(after.sub_d_runtime.last_yaw_step_raw)
        );
        assert_eq!(
            (entity.position_raw(), entity.velocity_raw()),
            (position, velocity),
            "C/A/B have not run"
        );
        assert_eq!(
            entity.physical_body_basis_q31(),
            RetailRuntimeValue::Known(basis)
        );
    }
}
