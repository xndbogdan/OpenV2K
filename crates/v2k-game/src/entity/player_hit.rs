//! Local player model-switch wrappers and direct `415040` hull continuation.
//!
//! `410090` installs 4C8A30; `438080` copies it to 4DC6E0 for types 46/51,
//! overriding only +20. Infection +18 and cure +1C remain DA00/DA60.
//! Every Player Control frame 4CD940+48*n (n=0..6) has null +20/+24 hooks.

use super::*;
use crate::damage::absorb_pre_health_damage_buffer;
use crate::world_fx::ParticleEntityImpact;

/// 15040 versus14E10: both filter255E0; only Checked owns bit8000 and
/// source46 filtered-zero feedback. Radial falloff owns its outer admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDamageEntry {
    Checked,
    Unchecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCheckedDamageRequest {
    pub entry: PlayerDamageEntry,
    pub target_id: u32,
    pub delivery: DamageDeliveryRecord,
    pub ratio_numerator: u32,
    pub ratio_denominator: u16,
}

/// Actual local hull and synchronous native death/effect custody.
pub struct PlayerCheckedDamageFrame<'a> {
    pub hull: &'a mut PlayerHull,
    pub resources: &'a crate::resource_cache::ResourceCache,
    pub world_fx: &'a mut WorldFx,
    pub retail_tick: u32,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub extra_lives: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDyingContactRuntime {
    player_id: u32,
    next_burst_retail_tick: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerDyingContactBurstRequest {
    pub position_raw: [i16; 3],
    pub source_extent_raw: u16,
    pub sea_level_raw: Option<i16>,
    pub retail_tick: u32,
}

impl PlayerDyingContactRuntime {
    pub const fn player_id(self) -> u32 {
        self.player_id
    }
    pub const fn next_burst_tick(self) -> i32 {
        self.next_burst_retail_tick
    }

    ///447AF0 uses a strict signed comparison, then calls475F0 before reseeding.
    pub fn emit_if_due(
        &mut self,
        fx: &mut WorldFx,
        request: PlayerDyingContactBurstRequest,
    ) -> bool {
        let tick = request.retail_tick as i32;
        if self.next_burst_retail_tick >= tick {
            return false;
        }
        fx.emit_player_wreck_burst_raw(
            request.position_raw,
            request.source_extent_raw,
            request.sea_level_raw,
            self.player_id,
        );
        self.next_burst_retail_tick =
            tick.wrapping_add(i32::from(fx.next_shared_retail_random_u16() >> 11));
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCheckedDamageBlock {
    Runtime(&'static str),
    Modifier(u32),
    LinkedAttachments,
    RemoteOwner,
    HitCallback(u32),
    PlayerFeedback,
    HullMismatch,
    DeathStyle,
    PendingReplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerCheckedDamageOutcome {
    NotPlayer,
    Ineligible,
    FilteredOut,
    Blocked(PlayerCheckedDamageBlock),
    Applied {
        /// Exact 15040 return before its buffer consumes any damage.
        filtered_damage_raw: i32,
        generic_hit_sound_id: Option<u16>,
        death_sound_id: Option<u16>,
        position_raw: [i16; 3],
        dying: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerModelSwitchHitBlock {
    Delivery,
    Style,
    Runtime(&'static str),
    Impact(Fun00411250ImpactReactionBlock),
    Checked(PlayerCheckedDamageBlock),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerModelSwitchHitOutcome {
    NotApplicable,
    Suppressed,
    Blocked(PlayerModelSwitchHitBlock),
    Applied {
        entry: EntityHitEntry,
        model_slot: usize,
        checked: PlayerCheckedDamageOutcome,
    },
}

pub(super) enum PlayerDamagePlan {
    NotPlayer,
    Ineligible,
    FilteredOut,
    AlreadyDying {
        index: usize,
        filtered_damage_raw: i32,
        buffer_after: i32,
        position_raw: [i16; 3],
    },
    Alive {
        index: usize,
        filtered_damage_raw: i32,
        transition: GenericEntityDamageTransition,
        generic_hit_sound_id: Option<u16>,
        death_sound_id: Option<u16>,
        position_raw: [i16; 3],
        death: Option<PlayerDyingPlan>,
        player_kill_hint: bool,
    },
}

impl PlayerDamagePlan {
    pub(super) fn terminal_at(&mut self, position: [i16; 3]) -> bool {
        if let Self::Alive {
            transition,
            position_raw,
            ..
        } = self
        {
            if transition.stage == GenericEntityDamageStage::DeathDispatchRequired {
                *position_raw = position;
                return true;
            }
        }
        false
    }
}

pub(super) struct PlayerDyingPlan {
    index: usize,
    model_slot: usize,
    context: BehaviorContextRuntime,
    source_extent_raw: u16,
    sea_level_raw: Option<i16>,
}

fn known<T>(
    value: RetailRuntimeValue<T>,
    name: &'static str,
) -> Result<T, PlayerCheckedDamageBlock> {
    match value {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(PlayerCheckedDamageBlock::Runtime(name)),
    }
}

fn player_control_style(entity: &Entity) -> bool {
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return false;
    };
    // All seven authenticated Player Control records and the exact alternate
    // Dying record have null infection/cure/death hook words. Use the same
    // active-style authority as physical contact rather than descriptor alone.
    crate::player_contact_style::is_player_style(context, 24)
        || crate::player_contact_style::is_player_style(context, 25)
}

impl EntityManager {
    pub fn player_dying_contact_runtime(&self) -> Option<PlayerDyingContactRuntime> {
        self.player_dying_contact_runtime
    }

    pub fn commit_player_dying_contact_runtime(
        &mut self,
        runtime: PlayerDyingContactRuntime,
    ) -> bool {
        if self.player_id != Some(runtime.player_id)
            || self
                .player_dying_contact_runtime
                .is_none_or(|current| current.player_id != runtime.player_id)
            || self.player().is_none_or(|player| {
                player.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
                    != RetailRuntimeValue::Known(DYING_STATE_BIT)
            })
        {
            return false;
        }
        self.player_dying_contact_runtime = Some(runtime);
        true
    }
    fn prepare_player_dying(
        &self,
        index: usize,
        infected: bool,
        resources: &crate::resource_cache::ResourceCache,
    ) -> Result<PlayerDyingPlan, PlayerCheckedDamageBlock> {
        let entity = &self.entities[index];
        if entity.capability_flags & 0x20 != 0 {
            return Err(PlayerCheckedDamageBlock::Runtime(
                "post-death objective continuation",
            ));
        }
        if known(
            entity.collision.constructor_sound_attachment_id_at_0x8c,
            "constructor sound attachment",
        )?
        .is_some()
        {
            return Err(PlayerCheckedDamageBlock::Runtime(
                "logical sound attachment release",
            ));
        }
        match self.player_pending_replacement_handle {
            RetailRuntimeValue::Known(None) => {}
            RetailRuntimeValue::Known(Some(_)) => {
                return Err(PlayerCheckedDamageBlock::PendingReplacement)
            }
            RetailRuntimeValue::Unresolved => {
                return Err(PlayerCheckedDamageBlock::Runtime(
                    "pending replacement handle",
                ))
            }
        }
        if !player_control_style(entity) {
            return Err(PlayerCheckedDamageBlock::DeathStyle);
        }
        let initializer = self
            .type_metadata
            .get(46)
            .and_then(|metadata| metadata.initializer.as_ref())
            .ok_or(PlayerCheckedDamageBlock::Runtime(
                "death alternate metadata",
            ))?;
        if initializer.behavior_rule_ref != 1 || initializer.alternate_behavior_class_ref != 25 {
            return Err(PlayerCheckedDamageBlock::DeathStyle);
        }
        match self.player_pair_damage_modifier_context_empty() {
            RetailRuntimeValue::Known(true) => {}
            RetailRuntimeValue::Known(false) => {
                return Err(PlayerCheckedDamageBlock::LinkedAttachments)
            }
            RetailRuntimeValue::Unresolved => {
                return Err(PlayerCheckedDamageBlock::Runtime(
                    "controller attachment list",
                ))
            }
        }
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(PlayerCheckedDamageBlock::DeathStyle);
        };
        let program = crate::entity_behavior::audited_behavior_program(25).unwrap();
        let context = context
            .reselect_audited_type_default(program, 0, program.initial_style)
            .ok_or(PlayerCheckedDamageBlock::DeathStyle)?;
        let model_slot = if infected { 3 } else { 1 };
        let model_id = entity
            .model_in_slot(model_slot)
            .ok_or(PlayerCheckedDamageBlock::Runtime("dying model slot"))?;
        let model = resources
            .global_model(model_id)
            .ok_or(PlayerCheckedDamageBlock::Runtime("dying model allocation"))?;
        Ok(PlayerDyingPlan {
            index,
            model_slot,
            context,
            source_extent_raw: model.radius >> 1,
            sea_level_raw: resources
                .terrain()
                .and_then(|terrain| terrain.water_enabled().then(|| terrain.sea_level_raw())),
        })
    }

    fn commit_player_dying(
        &mut self,
        plan: PlayerDyingPlan,
        fx: &mut WorldFx,
        retail_tick: u32,
        notifications: &mut crate::gameplay_notifications::GameplayNotifications,
        extra_lives: u8,
    ) {
        let entity = &mut self.entities[plan.index];
        // Physical hull death and checked damage converge on the same 10C10
        // publication before style/model initialization and water/static tails.
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        entity.current_behavior_context = RetailRuntimeValue::Known(Some(plan.context));
        entity
            .select_active_model_slot(plan.model_slot)
            .expect("preflight resolved dying model");
        let id = entity.id;
        let position = entity.position_raw();
        // 447280: empty cargo release, synchronous475F0, then controller208
        // seed. The seed consumes its RNG word even without a later contact.
        fx.emit_player_wreck_burst_raw(position, plan.source_extent_raw, plan.sea_level_raw, id);
        // 447468..447484: after the burst and the authenticated null +6C,
        // submit CA/D9 before the controller+208 RNG seed.
        notifications.queue_player_destroyed(extra_lives, retail_tick as i32);
        let delay = i32::from(fx.next_shared_retail_random_u16() >> 13);
        self.player_dying_contact_runtime = Some(PlayerDyingContactRuntime {
            player_id: id,
            next_burst_retail_tick: (retail_tick as i32).wrapping_add(delay),
        });
        // 4474B7..44755A clears 2 then1 and replaces0. The PlayerDeathLifecycle
        // host owns the new zeroed control7/BB8 wrapper, not an actor callback.
        entity.actor_tasks.clear_slot(ActorTaskSlot::Tertiary);
        entity.actor_tasks.clear_slot(ActorTaskSlot::Secondary);
        entity.actor_tasks.clear_slot(ActorTaskSlot::Primary);
    }

    /// Idempotent constructor boundary for hull deaths from physical contacts.
    /// The actual timed-control visit begins in the host after this publication.
    pub fn begin_player_dying(
        &mut self,
        frame: PlayerCheckedDamageFrame<'_>,
    ) -> Result<bool, PlayerCheckedDamageBlock> {
        let id = self
            .player_id
            .ok_or(PlayerCheckedDamageBlock::Runtime("player allocation"))?;
        if self
            .player_dying_contact_runtime
            .is_some_and(|runtime| runtime.player_id == id)
        {
            return Ok(false);
        }
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == id)
            .ok_or(PlayerCheckedDamageBlock::Runtime("player allocation"))?;
        let entity = &self.entities[index];
        if entity.entity_type != PLAYER_ENTITY_TYPE
            || !frame.hull.dying
            || frame.hull.health_raw != 0
        {
            return Err(PlayerCheckedDamageBlock::HullMismatch);
        }
        let infected = known(
            entity
                .collision
                .state_flags_at_0x08
                .masked(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            "infected model bit",
        )? != 0;
        if known(
            entity
                .collision
                .state_flags_at_0x08
                .masked(REMOTE_OWNED_STATE_BIT),
            "remote ownership",
        )? != 0
        {
            return Err(PlayerCheckedDamageBlock::RemoteOwner);
        }
        let death_sound = known(entity.collision.death_sound_id, "death sound")?;
        let position = entity.position_raw();
        let plan = self.prepare_player_dying(index, infected, frame.resources)?;
        let entity = &mut self.entities[index];
        entity.collision.health_raw = RetailRuntimeValue::Known(0);
        entity.collision.pre_health_damage_buffer_raw =
            RetailRuntimeValue::Known(frame.hull.pre_health_damage_buffer_raw);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        if let Some(sound) = death_sound {
            frame
                .world_fx
                .emit_fixed_positional_sound_raw(sound, position);
        }
        self.commit_player_dying(
            plan,
            frame.world_fx,
            frame.retail_tick,
            frame.notifications,
            frame.extra_lives,
        );
        Ok(true)
    }

    pub(super) fn prepare_player_checked_damage(
        &self,
        request: PlayerCheckedDamageRequest,
        hull: &PlayerHull,
        model_switch: Option<EntityHitEntry>,
        resources: &crate::resource_cache::ResourceCache,
    ) -> Result<PlayerDamagePlan, PlayerCheckedDamageBlock> {
        let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.id == request.target_id)
        else {
            return Err(PlayerCheckedDamageBlock::Runtime("target allocation"));
        };
        let entity = &self.entities[index];
        if entity.entity_type != PLAYER_ENTITY_TYPE || self.player_id != Some(request.target_id) {
            return Ok(PlayerDamagePlan::NotPlayer);
        }
        let collision = &entity.collision;
        if request.entry == PlayerDamageEntry::Checked
            && known(
                collision
                    .state_flags_at_0x08
                    .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT),
                "checked-damage bit",
            )? == 0
        {
            return Ok(PlayerDamagePlan::Ineligible);
        }
        let profile = known(collision.damage_profile, "damage profile")?;
        let filtered = request.delivery.packet.filtered_raw_with_ratio(
            Some(&profile),
            request.ratio_numerator,
            request.ratio_denominator,
        );
        if filtered == 0 {
            if request.entry == PlayerDamageEntry::Checked
                && request
                    .delivery
                    .filtered_zero_feedback_required(entity.capability_flags)
            {
                return Err(PlayerCheckedDamageBlock::PlayerFeedback);
            }
            return Ok(PlayerDamagePlan::FilteredOut);
        }
        match known(
            collision.pair_callbacks.damage_modifier_address,
            "damage modifier",
        )? {
            None => {}
            Some(0x0044_84A0) => match self.player_pair_damage_modifier_context_empty() {
                RetailRuntimeValue::Known(true) => {}
                RetailRuntimeValue::Known(false) => {
                    return Err(PlayerCheckedDamageBlock::LinkedAttachments)
                }
                RetailRuntimeValue::Unresolved => {
                    return Err(PlayerCheckedDamageBlock::Runtime(
                        "controller attachment list",
                    ))
                }
            },
            Some(address) => return Err(PlayerCheckedDamageBlock::Modifier(address)),
        }
        if known(
            collision.state_flags_at_0x08.masked(REMOTE_OWNED_STATE_BIT),
            "remote ownership",
        )? != 0
        {
            return Err(PlayerCheckedDamageBlock::RemoteOwner);
        }
        let buffer = known(collision.pre_health_damage_buffer_raw, "pre-health buffer")?;
        let dying = known(
            collision.state_flags_at_0x08.masked(DYING_STATE_BIT),
            "dying bit",
        )? != 0;
        if hull.pre_health_damage_buffer_raw != buffer || hull.dying != dying {
            return Err(PlayerCheckedDamageBlock::HullMismatch);
        }
        let position_raw = entity.position_raw();
        if dying {
            return Ok(PlayerDamagePlan::AlreadyDying {
                index,
                filtered_damage_raw: filtered,
                buffer_after: absorb_pre_health_damage_buffer(buffer, filtered).0,
                position_raw,
            });
        }
        let generic_hit_sound_id = known(collision.generic_hit_sound_id, "generic-hit sound")?;
        if let Some(address) = known(
            collision.pair_callbacks.type_hit_callback_address,
            "generic-hit callback",
        )? {
            return Err(PlayerCheckedDamageBlock::HitCallback(address));
        }
        let health = known(collision.health_raw, "health")?;
        if hull.health_raw != health {
            return Err(PlayerCheckedDamageBlock::HullMismatch);
        }
        let transition = generic_entity_damage_transition(
            GenericEntityDamageState {
                health_raw: health,
                pre_health_buffer_raw: buffer,
                already_dying: false,
            },
            filtered,
        );
        let mut death = None;
        let mut player_kill_hint = false;
        let death_sound_id = if transition.stage == GenericEntityDamageStage::DeathDispatchRequired
        {
            if !player_control_style(entity) {
                return Err(PlayerCheckedDamageBlock::DeathStyle);
            }
            let infected = match model_switch {
                Some(EntityHitEntry::Infected) => true,
                Some(EntityHitEntry::Cured) => false,
                _ => {
                    known(
                        collision
                            .state_flags_at_0x08
                            .masked(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
                        "infected model bit",
                    )? != 0
                }
            };
            if entity.model_in_slot(if infected { 3 } else { 1 }).is_none() {
                return Err(PlayerCheckedDamageBlock::Runtime("dying model slot"));
            }
            death = Some(self.prepare_player_dying(index, infected, resources)?);
            // 14E90 reads the post-10C10 target bit and queues resource4 only
            // for the delivered source46 word, independently of target type.
            if request.delivery.source_entity_type_raw == 46 {
                player_kill_hint = known(
                    collision.state_flags_at_0x08.masked(0x0100_0000),
                    "player-kill bit",
                )? != 0;
            }
            known(collision.death_sound_id, "death sound")?
        } else {
            None
        };
        Ok(PlayerDamagePlan::Alive {
            index,
            filtered_damage_raw: filtered,
            transition,
            generic_hit_sound_id,
            death_sound_id,
            position_raw,
            death,
            player_kill_hint,
        })
    }

    pub(super) fn commit_player_checked_damage(
        &mut self,
        plan: PlayerDamagePlan,
        hull: &mut PlayerHull,
        fx: &mut WorldFx,
        retail_tick: u32,
        notifications: &mut crate::gameplay_notifications::GameplayNotifications,
        extra_lives: u8,
    ) -> PlayerCheckedDamageOutcome {
        match plan {
            PlayerDamagePlan::NotPlayer => PlayerCheckedDamageOutcome::NotPlayer,
            PlayerDamagePlan::Ineligible => PlayerCheckedDamageOutcome::Ineligible,
            PlayerDamagePlan::FilteredOut => PlayerCheckedDamageOutcome::FilteredOut,
            PlayerDamagePlan::AlreadyDying {
                index,
                filtered_damage_raw,
                buffer_after,
                position_raw,
            } => {
                self.entities[index].collision.pre_health_damage_buffer_raw =
                    RetailRuntimeValue::Known(buffer_after);
                hull.pre_health_damage_buffer_raw = buffer_after;
                PlayerCheckedDamageOutcome::Applied {
                    filtered_damage_raw,
                    generic_hit_sound_id: None,
                    death_sound_id: None,
                    position_raw,
                    dying: true,
                }
            }
            PlayerDamagePlan::Alive {
                index,
                filtered_damage_raw,
                transition,
                generic_hit_sound_id,
                death_sound_id,
                position_raw,
                death,
                player_kill_hint,
            } => {
                let dying = transition.stage == GenericEntityDamageStage::DeathDispatchRequired;
                let entity = &mut self.entities[index];
                let health = if dying {
                    0
                } else {
                    transition.health_after_subtraction_raw
                };
                entity.collision.pre_health_damage_buffer_raw =
                    RetailRuntimeValue::Known(transition.pre_health_buffer_after_raw);
                hull.pre_health_damage_buffer_raw = transition.pre_health_buffer_after_raw;
                if let Some(sound_id) = generic_hit_sound_id {
                    fx.emit_fixed_positional_sound_raw(sound_id, position_raw);
                }
                entity.collision.health_raw = RetailRuntimeValue::Known(health);
                hull.health_raw = health;
                if dying {
                    entity
                        .collision
                        .state_flags_at_0x08
                        .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
                    hull.dying = true;
                }
                if let Some(sound_id) = death_sound_id {
                    fx.emit_fixed_positional_sound_raw(sound_id, position_raw);
                }
                if let Some(plan) = death {
                    self.commit_player_dying(plan, fx, retail_tick, notifications, extra_lives);
                }
                if player_kill_hint {
                    notifications.queue_player_kill(retail_tick as i32);
                }
                PlayerCheckedDamageOutcome::Applied {
                    filtered_damage_raw,
                    generic_hit_sound_id,
                    death_sound_id,
                    position_raw,
                    dying,
                }
            }
        }
    }

    /// Direct 15040: complete packet provenance and explicit filter ratio.
    /// No entry here writes +34, dispatches a hit style or adds primary effects.
    /// Nonempty 484A0 controller links require their own redirection owner.
    pub fn apply_player_checked_damage(
        &mut self,
        request: PlayerCheckedDamageRequest,
        frame: PlayerCheckedDamageFrame<'_>,
    ) -> PlayerCheckedDamageOutcome {
        match self.prepare_player_checked_damage(request, frame.hull, None, frame.resources) {
            Ok(plan) => self.commit_player_checked_damage(
                plan,
                frame.hull,
                frame.world_fx,
                frame.retail_tick,
                frame.notifications,
                frame.extra_lives,
            ),
            Err(block) => PlayerCheckedDamageOutcome::Blocked(block),
        }
    }

    /// Complete local F780/11250 or F7C0/11320 player visit. Resolve every
    /// reached unowned continuation before committing the model prefix.
    pub fn apply_player_model_switch_particle_hit(
        &mut self,
        impact: ParticleEntityImpact,
        frame: PlayerCheckedDamageFrame<'_>,
    ) -> PlayerModelSwitchHitOutcome {
        let PlayerCheckedDamageFrame {
            hull,
            resources,
            world_fx: fx,
            retail_tick,
            notifications,
            extra_lives,
        } = frame;
        let entry = impact.entity_hit_entry();
        if entry == EntityHitEntry::PrimaryProjectile
            || self.player_id != Some(impact.target_entity_id)
        {
            return PlayerModelSwitchHitOutcome::NotApplicable;
        }
        if impact.damage.is_none() {
            return PlayerModelSwitchHitOutcome::Suppressed;
        }
        let blocked = |block| PlayerModelSwitchHitOutcome::Blocked(block);
        let Some(delivery) = impact.damage_delivery_record() else {
            return blocked(PlayerModelSwitchHitBlock::Delivery);
        };
        let Some(index) = self
            .entities
            .iter()
            .position(|entity| entity.id == impact.target_entity_id)
        else {
            return blocked(PlayerModelSwitchHitBlock::Runtime("player allocation"));
        };
        let entity = &self.entities[index];
        if entity.entity_type != PLAYER_ENTITY_TYPE || !player_control_style(entity) {
            return blocked(PlayerModelSwitchHitBlock::Style);
        }
        let Ok(flags) = known(
            entity.collision.state_flags_at_0x08.masked(
                DYING_STATE_BIT
                    | ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
                    | IMPACT_REACTION_ENABLED_STATE_BIT
                    | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
            ),
            "model/impact bits",
        ) else {
            return blocked(PlayerModelSwitchHitBlock::Runtime("model/impact bits"));
        };
        let sound = match self.type_metadata.get(PLAYER_ENTITY_TYPE as usize) {
            Some(metadata) if entry == EntityHitEntry::Infected && flags & DYING_STATE_BIT == 0 => {
                metadata.infected_model_presentation_sound_id
            }
            Some(_) if entry == EntityHitEntry::Infected => RetailRuntimeValue::Known(None),
            Some(metadata) => metadata.cured_model_presentation_sound_id,
            None => RetailRuntimeValue::Unresolved,
        };
        if !matches!(sound, RetailRuntimeValue::Known(_)) {
            return blocked(PlayerModelSwitchHitBlock::Runtime("model-switch sound"));
        }
        let model_slot = usize::from(flags & DYING_STATE_BIT != 0)
            + if entry == EntityHitEntry::Infected {
                2
            } else {
                0
            };
        if entity.model_in_slot(model_slot).is_none() {
            return blocked(PlayerModelSwitchHitBlock::Runtime("selected model slot"));
        }
        let reaction_enabled = flags & IMPACT_REACTION_ENABLED_STATE_BIT != 0
            && flags & IMPACT_REACTION_SUPPRESSED_STATE_BIT == 0;
        if reaction_enabled {
            match known(
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(IMPACT_REACTION_NETWORKED_STATE_BIT),
                "network impact bit",
            ) {
                Ok(0) => {}
                Ok(_) => {
                    return blocked(PlayerModelSwitchHitBlock::Impact(
                        Fun00411250ImpactReactionBlock::NetworkRequest,
                    ))
                }
                Err(_) => return blocked(PlayerModelSwitchHitBlock::Runtime("network impact bit")),
            }
            if entity.mass_raw == 0 {
                return blocked(PlayerModelSwitchHitBlock::Impact(
                    Fun00411250ImpactReactionBlock::ZeroMass,
                ));
            }
        }
        let request = PlayerCheckedDamageRequest {
            entry: PlayerDamageEntry::Checked,
            target_id: impact.target_entity_id,
            delivery,
            ratio_numerator: 0,
            ratio_denominator: 0,
        };
        let plan = match self.prepare_player_checked_damage(request, hull, Some(entry), resources) {
            Ok(plan) => plan,
            Err(block) => return blocked(PlayerModelSwitchHitBlock::Checked(block)),
        };
        let prefix = self
            .apply_model_switch_particle_hit_prefix(impact.target_entity_id, entry)
            .expect("preflight resolved prefix");
        self.entities[index]
            .select_active_model_slot(model_slot)
            .expect("preflight resolved model");
        if let Some(sound_id) = prefix.sound_id {
            fx.emit_fixed_positional_sound_raw(sound_id, prefix.position_raw);
        }
        // All seven current Player Control style+20/+24 words are null.
        if reaction_enabled {
            let reaction = self.apply_fun_00411250_player_impact_reaction(
                impact.target_entity_id,
                delivery.packet,
                impact.velocity_raw,
                || u32::from(fx.next_shared_retail_random_u16()),
            );
            debug_assert!(matches!(
                reaction,
                Fun00411250ImpactReactionOutcome::Applied(_)
            ));
        }
        let checked = self.commit_player_checked_damage(
            plan,
            hull,
            fx,
            retail_tick,
            notifications,
            extra_lives,
        );
        PlayerModelSwitchHitOutcome::Applied {
            entry,
            model_slot: match self.entities[index].collision.active_model_slot() {
                RetailRuntimeValue::Known(slot) => slot,
                RetailRuntimeValue::Unresolved => unreachable!("preflight resolved model bits"),
            },
            checked,
        }
    }
}

#[cfg(test)]
mod tests;
