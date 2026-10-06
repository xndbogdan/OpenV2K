//! Particle draw callbacks:43D300 forward copies and442240 fading backward copies.

use super::*;
use v2k_formats::fixed_math::retail_integer_sqrt;

const STREAK_DRAW_CALLBACK_VA: u32 = 0x0043_D300;
const STREAK_SAMPLE_COUNT: usize = 10;
const BACKWARD_STREAK_DRAW_CALLBACK_VA: u32 = 0x0044_2240;
const BACKWARD_STREAK_SAMPLE_COUNT: usize = 7;

#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawRecord {
    Live,
    StackStreak,
    LiveThenBackwardStackStreak,
}

impl WorldFx {
    /// Run the retail draw callbacks once and retain their ordered D410
    /// survivors for both terrain lighting and subsequent sprite submission.
    pub fn prepare_presentation(
        &mut self,
        viewport: [u32; 2],
        far_depth_raw: i32,
        project: impl FnMut(&WorldParticle) -> v2k_render::ParticleCenterProjection,
    ) -> ParticlePresentationFrame {
        self.prepare_presentation_with_stack_addresses(
            viewport,
            far_depth_raw,
            ParticlePresentationStackAddresses::default(),
            project,
        )
    }

    /// As prepare_presentation, with an explicit copied-record address for
    /// callers holding a captured retail address or a controlled source oracle.
    /// No port stack or live pool address is substituted when it is absent.
    pub fn prepare_presentation_with_stack_addresses(
        &mut self,
        viewport: [u32; 2],
        far_depth_raw: i32,
        stack_addresses: ParticlePresentationStackAddresses,
        mut project: impl FnMut(&WorldParticle) -> v2k_render::ParticleCenterProjection,
    ) -> ParticlePresentationFrame {
        // Copy candidates in exact priority/list order before mutating the
        // intrusive pool. Each draw callback completes before the next record.
        let candidates: Vec<_> = self
            .particles
            .presentation()
            .map(|presented| (presented.slot, *presented.particle))
            .collect();
        let mut particles = Vec::with_capacity(candidates.len());
        for (slot, particle) in candidates {
            let Some(descriptor) = particle_descriptor(particle.source_class) else {
                self.particles.free(slot);
                continue;
            };
            let record = match descriptor.raw_u32(0x30) {
                STREAK_DRAW_CALLBACK_VA => DrawRecord::StackStreak,
                BACKWARD_STREAK_DRAW_CALLBACK_VA => DrawRecord::LiveThenBackwardStackStreak,
                _ => DrawRecord::Live,
            };
            // Class19 is an authored invisible collision probe. Ordinary
            // null-frame D410 sets80 before projection; D300's equivalent
            // write affects only its stack copy, never the live allocation.
            if descriptor.frame_list_va() == 0 {
                if record == DrawRecord::Live
                    && particle.source_class != PLAYER_SURFACE_PROBE_PARTICLE_CLASS
                {
                    self.particles.free(slot);
                } else if record == DrawRecord::LiveThenBackwardStackStreak {
                    //44230B first calls D410 on the real record. D410 sets80;
                    //40120 owns later removal, while copied calls set only80
                    //in their own stack record.
                    if let Some(live) = self.particles.slots[slot].as_mut() {
                        live.pending_destruction = true;
                    }
                }
                continue;
            }
            let (count, step_raw) = match record {
                DrawRecord::Live => (1, [0; 3]),
                DrawRecord::StackStreak | DrawRecord::LiveThenBackwardStackStreak => (
                    if record == DrawRecord::StackStreak {
                        STREAK_SAMPLE_COUNT
                    } else {
                        BACKWARD_STREAK_SAMPLE_COUNT
                    },
                    streak_step_raw(
                        particle
                            .velocity
                            .map(|value| world_velocity_component_to_raw(value) as i16),
                    ),
                ),
            };
            let mut sample = particle;
            let mut position_raw = world_position_to_raw(particle.position);
            if record != DrawRecord::Live {
                // D410 reads signed position words even for442240's first
                // live pointer. Its presentation copy must not invent legacy
                // float remainders. The copied +8/+A/+C words also own the first projection;
                // keep legacy fractional integration out of this draw copy.
                sample.position = raw_position_to_world(position_raw);
            }
            for index in 0..count {
                if record == DrawRecord::LiveThenBackwardStackStreak {
                    if index == 0 {
                        //44226D takes the signed descriptor+0A scale, not the
                        //separate frame tuple scale. The copy predates D410.
                        sample.draw_scale_raw = descriptor.draw_scale_raw();
                    } else {
                        let multiplier: i16 = if index == 6 { 3 } else { 1 };
                        for (position, step) in position_raw.iter_mut().zip(step_raw) {
                            *position = position.wrapping_sub(step.wrapping_mul(multiplier));
                        }
                        sample.position = raw_position_to_world(position_raw);
                        if index < 6 && index % 2 == 1 {
                            sample.draw_scale_raw = ((sample.draw_scale_raw as i16) >> 1) as u16;
                        }
                    }
                } else if index != 0 {
                    for (position, step) in position_raw.iter_mut().zip(step_raw) {
                        *position = position.wrapping_add(step);
                    }
                    sample.position = raw_position_to_world(position_raw);
                }
                let live_record = record == DrawRecord::Live
                    || (record == DrawRecord::LiveThenBackwardStackStreak && index == 0);
                let draw_record_address = if live_record {
                    ParticleDrawRecordAddress::LivePoolSlot
                } else {
                    ParticleDrawRecordAddress::Stack {
                        callback_va: descriptor.raw_u32(0x30),
                        record_address: match record {
                            DrawRecord::StackStreak => stack_addresses.forward_streak_record,
                            _ => stack_addresses.backward_streak_record,
                        },
                    }
                };
                let projection = project(&sample);
                match d410_pre_cull(projection, viewport, far_depth_raw, descriptor.flags()) {
                    ParticlePreCull::Active => particles.push(PreparedParticle {
                        slot,
                        particle: sample,
                        projection,
                        draw_record_address,
                    }),
                    ParticlePreCull::RetainHidden => {}
                    ParticlePreCull::Delete if record == DrawRecord::Live => {
                        self.particles.free(slot);
                    }
                    ParticlePreCull::Delete if live_record => {
                        if let Some(live) = self.particles.slots[slot].as_mut() {
                            live.pending_destruction = true;
                        }
                    }
                    //43D300 and442240 continue their copied D410 calls after a
                    //sample sets80. D410 never reads that bit to skip the next
                    //call. The copy's10 visible bit likewise is not written
                    //back. Each accepted sample independently reaches light,
                    //then the ordinary later far/rectangle sprite gate.
                    ParticlePreCull::Delete => {}
                }
            }
        }
        ParticlePresentationFrame {
            particles,
            viewport,
            far_depth_raw,
        }
    }
}

///43D30D..43D3B9 normalizes the signed velocity words into signed Q12 words,
///then arithmetic-shifts each product by12. The sqrt return is narrowed to
///i16 BEFORE division; this is not457960's Q31 normalization or +1 length.
fn streak_step_raw(velocity: [i16; 3]) -> [i16; 3] {
    let squared = velocity.into_iter().fold(0_i32, |sum, component| {
        sum.wrapping_add(i32::from(component).wrapping_mul(i32::from(component)))
    });
    let speed = i32::from(retail_integer_sqrt(squared) as i16);
    let direction_q12 = if speed == 0 {
        //43D373 stores00001000, then loads BX=1000,DI=0,AX=0.
        [0x1000, 0, 0]
    } else {
        velocity.map(|component| ((i32::from(component) << 12) / speed) as i16)
    };
    direction_q12.map(|component| (i32::from(component) * 25 >> 12) as i16)
}

#[cfg(test)]
mod tests;
