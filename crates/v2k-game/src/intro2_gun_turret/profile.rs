//! E/L Gun Turret data. Live authored92/96/97/99/102/103/104/115 and Intro2
//! authored92/102/115 retain distinct receipts; campaign reconstruction also
//! uses96/100 metadata.
//! Castle Type104's Section12 row uses common vtable4C8A30 and class29/4C8230:
//! D4A0 birth, D190 initializer, null style death cleanup, alternate49/BD20.

use super::{AXIS, EMITTER, MODEL};
use crate::{damage::DamageProfile, entity::Entity};
use v2k_formats::collision::{CommonAxisDescriptor, ProjectileEmitterDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2GunTurretProfile {
    Type92,
    Type96,
    Type97,
    Type99,
    Type100,
    Type102,
    Type103,
    Type104,
    Type115,
}

impl Intro2GunTurretProfile {
    /// Authored104B0 allocations with the common D4A0/D190 E/L constructor.
    /// Type115 retains alternate1; sharing this birth does not grant class49.
    /// Type100 shares the constructor, but its method19 has no live aim
    /// owner yet.
    pub const fn for_native_authored(entity_type: u32) -> Option<Self> {
        match entity_type {
            92 => Some(Self::Type92),
            96 => Some(Self::Type96),
            97 => Some(Self::Type97),
            99 => Some(Self::Type99),
            102 => Some(Self::Type102),
            103 => Some(Self::Type103),
            104 => Some(Self::Type104),
            115 => Some(Self::Type115),
            _ => None,
        }
    }

    /// Intro2-authored Type92/102/115. Ordinary tulaz/tucannon use
    /// [`Self::for_ordinary_cargo`].
    pub const fn for_type(entity_type: u32) -> Option<Self> {
        match entity_type {
            92 => Some(Self::Type92),
            102 => Some(Self::Type102),
            115 => Some(Self::Type115),
            _ => None,
        }
    }

    /// Class-29 E/L cargo families whose Section-12 rows authenticate the
    /// shared D190 constructor. Distinct models/emitters stay per-type.
    pub const fn for_ordinary_cargo(entity_type: u32) -> Option<Self> {
        match entity_type {
            92 => Some(Self::Type92),
            96 => Some(Self::Type96),
            97 => Some(Self::Type97),
            100 => Some(Self::Type100),
            _ => None,
        }
    }

    pub const fn entity_type(self) -> u32 {
        match self {
            Self::Type92 => 92,
            Self::Type96 => 96,
            Self::Type97 => 97,
            Self::Type99 => 99,
            Self::Type100 => 100,
            Self::Type102 => 102,
            Self::Type103 => 103,
            Self::Type104 => 104,
            Self::Type115 => 115,
        }
    }

    pub const fn model(self) -> usize {
        match self {
            Self::Type92 => 171,
            Self::Type96 => 175,
            Self::Type97 => 173,
            Self::Type99 => 165,
            Self::Type100 => 152,
            Self::Type102 => MODEL,
            Self::Type103 => 168,
            Self::Type104 => 156,
            Self::Type115 => 331,
        }
    }

    pub const fn model_slots(self) -> [u16; 4] {
        match self {
            Self::Type115 => [331, 144, 332, 144],
            _ => [self.model() as u16; 4],
        }
    }

    /// Section-12 model-variable words; a third is bound by emitter slot 1.
    pub const fn model_variable_count(self) -> u32 {
        match self {
            Self::Type99 | Self::Type100 | Self::Type102 | Self::Type103 => 3,
            _ => 2,
        }
    }

    pub const fn alternate_behavior_class(self) -> u32 {
        match self {
            Self::Type115 => 1,
            _ => 49,
        }
    }

    pub const fn sub_l(self) -> [u8; 6] {
        match self {
            Self::Type115 => [1, 2, 0x40, 0x1f, 0xa0, 0x0f],
            _ => super::SUB_L,
        }
    }

    pub const fn choices(self) -> &'static [v2k_formats::collision::BehaviorChoice] {
        match self {
            Self::Type115 => &super::FLOWER_CHOICES,
            _ => &super::INITIAL_CHOICES,
        }
    }

    pub const fn gravity_lead(self) -> bool {
        //44EA60 method16 uses jump index4 ->44EA7B (one).
        matches!(self, Self::Type115)
    }

    pub const fn health(self) -> i32 {
        match self {
            Self::Type92 | Self::Type96 | Self::Type97 | Self::Type99 | Self::Type103 => 5000,
            Self::Type100 => 32000,
            Self::Type102 => 4000,
            Self::Type104 => 6000,
            Self::Type115 => 1000,
        }
    }

    pub const fn capability(self) -> u32 {
        match self {
            Self::Type99 | Self::Type102 | Self::Type103 | Self::Type104 => 0x44,
            Self::Type115 => 0,
            _ => 0x1044,
        }
    }

    pub const fn axis(self) -> CommonAxisDescriptor {
        match self {
            Self::Type92 | Self::Type96 | Self::Type97 | Self::Type104 | Self::Type115 => {
                CommonAxisDescriptor {
                    raw_word_at_0x04: 0x100b,
                    ..AXIS
                }
            }
            Self::Type100 => CommonAxisDescriptor {
                strict_axis_limit_raw: 3840,
                raw_word_at_0x04: 0x000b,
            },
            Self::Type99 | Self::Type102 | Self::Type103 => AXIS,
        }
    }

    pub const fn damage(self) -> DamageProfile {
        DamageProfile {
            thresholds_raw: match self {
                Self::Type92 | Self::Type100 => [0, 2000, 1800, 200, 200, 200, 0],
                Self::Type96 | Self::Type103 => [0, 2000, 2100, 200, 200, 200, 0],
                Self::Type97 | Self::Type99 | Self::Type102 => [0, 8000, 2100, 200, 200, 200, 0],
                Self::Type104 => [0, 6000, 2100, 200, 2000, 200, 0],
                Self::Type115 => [0, 500, 500, 200, 2000, 200, 0],
            },
            multipliers_q8: match self {
                Self::Type115 => [0, 256, 256, 128, 256, 0, 0],
                Self::Type104 => [0, 256, 256, 128, 256, 256, 256],
                _ => [0, 256, 256, 256, 256, 256, 0],
            },
        }
    }

    pub const fn emitter(self) -> ProjectileEmitterDescriptor {
        match self {
            Self::Type92 => ProjectileEmitterDescriptor {
                random_interval_us: 400_000,
                speed_override_raw: 4000,
                target_axis_tolerance_raw: 3840,
                raw_word_at_0x12: 18,
                alternate_emitter_raw: 0,
                variable_bindings: [0; 4],
                ..EMITTER
            },
            Self::Type96 => ProjectileEmitterDescriptor {
                projectile_method: 13,
                random_interval_us: 400_000,
                speed_override_raw: 4000,
                target_axis_tolerance_raw: 3840,
                sound_id: 77,
                raw_word_at_0x12: 18,
                alternate_emitter_raw: 0,
                variable_bindings: [0; 4],
                ..EMITTER
            },
            Self::Type97 => ProjectileEmitterDescriptor {
                projectile_method: 12,
                random_interval_us: 400_000,
                spread_raw: 1024,
                speed_override_raw: 4000,
                target_axis_tolerance_raw: 3840,
                sound_id: 76,
                raw_word_at_0x12: 18,
                alternate_emitter_raw: 0,
                variable_bindings: [0; 4],
                ..EMITTER
            },
            Self::Type99 => ProjectileEmitterDescriptor {
                projectile_method: 12,
                sound_id: 76,
                ..EMITTER
            },
            Self::Type100 => ProjectileEmitterDescriptor {
                projectile_method: 19,
                random_interval_us: 2_500_000,
                speed_override_raw: 2000,
                target_axis_tolerance_raw: 3072,
                sound_id: 7,
                raw_word_at_0x12: 20,
                alternate_emitter_raw: 0,
                stochastic_gate_mode: 0,
                auxiliary_command: 1,
                variable_bindings: [0, 3, 3, 0],
                ..EMITTER
            },
            Self::Type102 => EMITTER,
            Self::Type103 => ProjectileEmitterDescriptor {
                projectile_method: 13,
                sound_id: 77,
                ..EMITTER
            },
            Self::Type104 => ProjectileEmitterDescriptor {
                projectile_method: 18,
                random_interval_us: 400_000,
                speed_override_raw: 2000,
                target_axis_tolerance_raw: 2560,
                sound_id: 88,
                raw_word_at_0x12: 16,
                alternate_emitter_raw: 0,
                auxiliary_command: 1,
                variable_bindings: [0; 4],
                ..EMITTER
            },
            Self::Type115 => ProjectileEmitterDescriptor {
                projectile_method: 16,
                random_interval_us: 400_000,
                speed_override_raw: 2000,
                target_axis_tolerance_raw: 2560,
                sound_id: 0,
                raw_word_at_0x12: 26,
                alternate_emitter_raw: 0,
                variable_bindings: [0; 4],
                ..EMITTER
            },
        }
    }

    pub const fn authored_xz(self, spawn: usize) -> Option<[i16; 2]> {
        match (self, spawn) {
            (Self::Type92, 52) => Some([0x5800, 0xe400u16 as i16]),
            (Self::Type102, 53) => Some([0x6400, 0xf500u16 as i16]),
            (Self::Type102, 54) => Some([0x6800, 0xeb00u16 as i16]),
            (Self::Type115, 61) => Some([-16384, 2816]),
            _ => None,
        }
    }
}

pub(super) fn profile_for_entity(entity: &Entity) -> Option<Intro2GunTurretProfile> {
    Intro2GunTurretProfile::for_type(entity.entity_type).or_else(|| {
        let runtime = entity.intro2_gun_turret_runtime?;
        (Intro2GunTurretProfile::for_native_authored(entity.entity_type) == Some(runtime.profile)
            && matches!(
                runtime.origin,
                super::GunTurretConstructionOrigin::NativeOrdinary(_)
            ))
        .then_some(runtime.profile)
    })
}
