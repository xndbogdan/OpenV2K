//! Retail gameplay text/resource notification slots.
//!
//! `FUN_00456900` writes one replaceable direct-text slot at controller
//! `+0x2c0`. `FUN_004568B0` writes the separate `+0x2c4` resource slot and
//! remembers every event id in a 32-bit mask so each resource hint is shown at
//! most once per session. Both slots are rendered by `FUN_00452CB0` through the
//! authored Section-2 timing/layout prefix parsed by `FUN_00452790`.

use crate::attract_attention::{
    AttractAttentionResourceTextRequest, ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
    ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
};
use crate::entity::EntityManager;
use crate::entity_pair_callbacks::MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID;

/// The caller's controller phase for `FUN_004568B0` resource notifications.
/// Retail admits these requests only in phase 5. Actor origin and world ID
/// do not identify that phase: the same actor can run in a cinematic or play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayNotificationPhase {
    Playing,
    NonGameplay,
}

/// Direct Section-2 string requested by class-14 `FUN_0040C3A0` at session
/// byte zero: `FUN_00456900(0xC6, 0)`. Capture `20260817-072421`.
pub const TYPE9_SESSION_ZERO_CLASS14_TEXT_ID: usize = 0xc6;
/// Direct global Section-2 string selected when the cargo hold is full.
pub const CARGO_FULL_TEXT_ID: usize = 0xda;
/// Deduplicated hint paired with the direct cargo-full text.
pub const CARGO_FULL_HINT_EVENT_ID: u8 = 0x11;
/// Deduplicated hint emitted after collecting entity type 68 (`weight`).
pub const WEIGHT_COLLECTED_HINT_EVENT_ID: u8 = 0x15;
///443B50's successful attachment of a child whose capability800 is set.
pub const CARGO_COLLECTED_HINT_EVENT_ID: u8 = 0;
/// Deduplicated resource hint selected after a successful player-authored
/// lethal hit on a target carrying retail state bit `0x01000000`.
pub const PLAYER_KILL_HINT_EVENT_ID: u8 = 4;
/// Parameterized direct text used by controller operation `0x33` after a
/// successful fuel pickup. Its authored body is `%s`; formatter selector 10
/// resolves the queued argument as another global Section-2 string.
pub const FUEL_COLLECTED_TEXT_ID: usize = 0xcd;
/// Global Section-2 string argument `Extra Fuel` supplied to text `0xCD`.
pub const FUEL_COLLECTED_TEXT_ARGUMENT_ID: usize = 0x114;
/// Deduplicated hint paired with a successful fuel pickup.
pub const FUEL_COLLECTED_HINT_EVENT_ID: u8 = 0x0c;
/// Direct global Section-2 string selected when the fuel tank is already full.
pub const FUEL_FULL_TEXT_ID: usize = 0xdc;
/// Direct text used when the repair pickup is rejected above 38,000 health.
pub const HULL_REPAIR_FULL_TEXT_ID: usize = 0xdd;
/// Deduplicated hint paired with an accepted static kind-2 repair pickup.
pub const HULL_REPAIRED_HINT_EVENT_ID: u8 = 0x0d;
/// Operation `0x35` reuses the parameterized 0xCD line with this argument.
pub const SHIELD_COLLECTED_TEXT_ARGUMENT_ID: usize = 0x110;
/// Parameterized argument emitted by selector `0x36` after adding lives.
pub const EXTRA_LIFE_COLLECTED_TEXT_ARGUMENT_ID: usize = 0x111;
/// Parameterized argument emitted by selector `0x3A` when the player type has
/// an authored cargo attachment list, including duplicate capacity pickups.
pub const CARGO_CAPACITY_COLLECTED_TEXT_ARGUMENT_ID: usize = 0x112;
/// Shared parameterized line used by weapon and capability acquisitions.
pub const POWER_UP_COLLECTED_TEXT_ID: usize = 0xcd;
/// Direct-text argument installed by selector `0x3C`.
pub const TARGETTER_COLLECTED_TEXT_ARGUMENT_ID: usize = 0x10f;
/// Deduplicated resource hint installed by selector `0x3C`.
pub const TARGETTER_COLLECTED_HINT_EVENT_ID: u8 = 0x13;
/// Direct-text argument installed by selector `0x3E`.
pub const TURBO_COLLECTED_TEXT_ARGUMENT_ID: usize = 0x113;
/// Direct notification emitted by the first zero-amount trophy claim in a
/// level through `FUN_00456820(0)`.
pub const TROPHY_LEVEL_CLAIM_TEXT_ID: usize = 0xd0;
/// Resource hint paired with the first per-level trophy claim.
pub const TROPHY_LEVEL_CLAIM_HINT_EVENT_ID: u8 = 0x14;
/// Parameterized line emitted whenever the trophy count reaches a multiple
/// of five and the controller reward byte advances.
pub const TROPHY_REWARD_TEXT_ARGUMENT_ID: usize = 0x111;
/// Direct warning submitted while the controlled player's callback-entry hull
/// health is strictly below `0x1389`. Authored `0xDF` is layout 14 / formatter
/// 15: centered, no type-on, and `FUN_00452270` case `0xF` blink-empty.
pub const HULL_LOW_TEXT_ID: usize = 0xdf;
/// Direct warning submitted by the powered VTOL branch below 10000 fuel.
pub const FUEL_LOW_TEXT_ID: usize = 0xe0;
/// Deduplicated resource event submitted when the VTOL callback begins with an
/// empty tank. `DAT_004CAD80[0x0B]` selects global Section-2 string `0xEC`,
/// `You cannot fly without any fuel`.
pub const FUEL_EMPTY_HINT_EVENT_ID: u8 = 0x0b;
/// Working Factory delivery helper `FUN_00418D30` queues this deduplicated
/// resource event after submitting operation `0x33` and before staffing.
pub const FACTORY_DELIVERY_HINT_EVENT_ID: u8 = 5;
/// C910 queues event8 after a visible spider successfully attaches a person.
/// The caller retains 4568B0's gameplay-controller phase guard.
pub const SPIDER_CAPTURE_HINT_EVENT_ID: u8 = 8;
/// `FUN_004147A0` requests event 9 after appending an infected firing record
/// when capability 8 and entity state `0x04000000` are both set.
pub const INFECTED_FIRING_HINT_EVENT_ID: u8 = 9;
/// Resource event queued when a visible staffing change reaches capacity.
pub const FACTORY_CAPACITY_HINT_EVENT_ID: u8 = 2;
/// Deduplicated resource hint emitted before each finite-stock worker
/// ejection attempt in Working Factory phase 2.
pub const FACTORY_OUTPUT_CONVERSION_HINT_EVENT_ID: u8 = 0x16;
/// Direct string emitted after a successful high-threshold factory product.
pub const FACTORY_PRODUCT_READY_TEXT_ID: usize = 0xd1;
/// Direct string `Factory under attack`, emitted by Working Factory's first
/// eligible under-attack callback.
pub const FACTORY_UNDER_ATTACK_TEXT_ID: usize = 0xd3;
/// `FUN_0041BEB0` lethal/dying continuation `FUN_004568B0(3)`.
/// `DAT_004CAD80[3]` is global Section-2 string `0xE4`.
pub const HIVE_DESTROYED_HINT_EVENT_ID: u8 = 3;
/// `FUN_0041BEB0` lock hint `FUN_004568B0(10)`. `DAT_004CAD80[10]` is
/// global Section-2 string `0xEB`.
pub const HIVE_LOCKED_HINT_EVENT_ID: u8 = 10;
/// `FUN_0041BEB0` unlock resource `FUN_004568B0(0xE)`. `DAT_004CAD80[0xE]`
/// is global Section-2 string `0xEF`.
pub const HIVE_UNLOCK_HINT_EVENT_ID: u8 = 0x0e;
/// Direct `FUN_00456900(0xD2)` submitted immediately before the unlock hint.
pub const HIVE_NOW_VULNERABLE_TEXT_ID: usize = 0xd2;
/// Direct text `0xCE` when a visible staffing clamp hits capacity, including
/// the damaged-repair re-entry path in `FUN_00419010`.
pub const FACTORY_CAPACITY_STAFFING_TEXT_ID: usize = 0xce;
/// Global Section-11 sound used by `FUN_00452790` while text types on.
///
/// The call dereferences `*DAT_004FE64C`, i.e. physical global slot zero.
/// `0x2D` in the same function is layout preset 5's baseline percentage, not
/// a sound id.
pub const GAMEPLAY_TEXT_TYPE_SOUND_ID: usize = 0;

/// Direct message 0xD9 remains active until explicitly cleared with id zero.
const STICKY_DIRECT_STRING_ID: usize = 0xd9;

/// Executable table `DAT_004CAD80`: gameplay resource event id -> global
/// Section-2 string id. Event 0x0f deliberately has no resource.
const RESOURCE_EVENT_STRING_IDS: [usize; 25] = [
    0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef, 0,
    0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
];

/// Scratch timestamp for deferred native construction receipts. Ordinary
/// 451710 clears their visible slots before resetting the world clock, while
/// retaining the consumed resource-event bits. This is not a presentation epoch.
const FRESH_LEVEL1_TYPE9_NOTIFICATION_LOAD_TICK: i32 = 0;

/// The live BA40 callback uses this visit's clock, including when a retained
/// root transition resumes. It must publish before the cue constructor runs;
/// the initial-load receipt queue has a separate level-clock rebasing owner.
#[derive(Clone, Copy)]
pub(crate) struct OrdinaryType9LiveNotificationContext<'a> {
    pub dispatch_resource_text: &'a dyn Fn(AttractAttentionResourceTextRequest, u32),
    pub retail_tick: u32,
}

impl OrdinaryType9LiveNotificationContext<'_> {
    pub(crate) fn dispatch(self, request: AttractAttentionResourceTextRequest) {
        (self.dispatch_resource_text)(request, self.retail_tick);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NotificationSlot {
    string_id: usize,
    sub_param: usize,
    start_tick: i32,
    cursor: bool,
}

/// One authored notification line ready for the sprite-font renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct GameplayNotificationLine {
    pub string_id: usize,
    pub text: String,
    pub x_percent: i32,
    pub baseline_percent: i32,
    pub width_percent: i32,
    /// `FUN_00454390` centers on display width. `FUN_00452790` also centers
    /// when the layout preset is strictly greater than 9 (`FUN_00470f80`
    /// width, then `(display_width - string_width) / 2`).
    pub center_x: bool,
}

/// Complete output of one retail HUD notification pass.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GameplayNotificationPresentation {
    /// Direct text precedes the deduplicated resource slot, matching
    /// `FUN_00452CB0`'s two `FUN_00437EE0` calls.
    pub lines: Vec<GameplayNotificationLine>,
    /// At most one type-on cue is emitted per frame because both slots share
    /// `DAT_004F72E8`'s strict three-tick throttle.
    pub play_typewriter_sound: bool,
}

/// Process-global `FUN_00452790` type-on sound throttle (`DAT_004F72E8`).
///
/// Every active authored text record visits the same field in table order.
/// An incomplete prefix may cue physical global sound zero only when the
/// signed tick difference is strictly greater than two; a complete active
/// record clears the field. Inactive records return before touching it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TextTypewriterCadence {
    last_tick: i32,
}

impl TextTypewriterCadence {
    /// Observe one active authored record, preserving the executable's shared
    /// field update and strict signed `> 2` test.
    pub fn observe_active_line(&mut self, reveal_incomplete: bool, retail_tick: i32) -> bool {
        if !reveal_incomplete {
            self.last_tick = 0;
            return false;
        }
        if retail_tick.wrapping_sub(self.last_tick) > 2 {
            self.last_tick = retail_tick;
            true
        } else {
            false
        }
    }
}

/// `FUN_00452270` case `0xE`/`0xF`: empty the output while
/// `|DAT_004FED60 / 40|` is even. Non-negative world ticks hide on
/// `0..=39`, `80..=119`, ... and show on `40..=79`.
pub const fn fun_00452270_case_e_hides_line(dat_004fed60: i32) -> bool {
    let quotient = dat_004fed60 / 0x28;
    let sign = quotient >> 31;
    let abs = (quotient ^ sign).wrapping_sub(sign);
    abs & 1 == 0
}

/// The two retail gameplay-notification slots and their session event mask.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameplayNotifications {
    direct: Option<NotificationSlot>,
    resource: Option<NotificationSlot>,
    seen_resource_events: u32,
    /// Live `FUN_0042DD10` selector 3 (`+0x1A8 + +0x1B0`) consumed by
    /// `FUN_00452270` case 2 at draw time. Class-14 `0xC6` is `People left: %d`.
    people_left: i32,
}

/// Evidence mismatch in one retained Attract-Attention event receipt.
///
/// Both words are authenticated before narrowing the event to the resource
/// table's byte-sized index. In particular, `0x110` must not alias event
/// `0x10` through the retail helper's masked dedup bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractAttentionResourceTextReceiptError {
    UnexpectedEvent { actual: u32 },
    UnexpectedGlobalResourceId { actual: u32 },
}

/// Atomic drain failure for retained native Attract-Attention receipts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshLevel1Type9NotificationDrainError {
    pub receipt_index: usize,
    pub receipt: AttractAttentionResourceTextRequest,
    pub error: AttractAttentionResourceTextReceiptError,
}

impl GameplayNotifications {
    pub fn new() -> Self {
        Self::default()
    }

    /// `456C50` writes this independent hint mask after the checkpoint copies.
    pub const fn save_tail_seen_mask(&self) -> u32 {
        self.seen_resource_events
    }

    /// 44F7E1 ->437FF0 clears the slots and hint mask on frontend/session
    /// selection. Campaign warps preserve this mask. The independent
    /// process-global [`TextTypewriterCadence`] is owned by the runtime.
    pub fn reset_session(&mut self) {
        self.direct = None;
        self.resource = None;
        self.seen_resource_events = 0;
    }

    /// 45189C/4518AE: ordinary native Loading (+28E == 0) clears both visible
    /// slots after authored construction and451C00.437F30 leaves each slot's
    /// resource-event mask at+0C intact, so a collected-weight/BA40 hint that
    /// already fired during load must not return when gameplay begins.
    pub fn clear_native_load_slots(&mut self) {
        self.direct = None;
        self.resource = None;
    }

    /// Validate and queue Attract Attention's exact event-`0x10` resource
    /// request through retail's canonical once-per-session resource owner.
    pub fn queue_attract_attention_resource_text(
        &mut self,
        request: AttractAttentionResourceTextRequest,
        retail_tick: i32,
    ) -> Result<(), AttractAttentionResourceTextReceiptError> {
        validate_attract_attention_resource_text_receipt(request)?;
        self.queue_validated_attract_attention_resource_text(retail_tick);
        Ok(())
    }

    /// Move every construction-committed Attract Attention receipt out of the
    /// entity manager only after the complete batch is authenticated.
    ///
    /// Validation precedes both manager custody transfer and notification
    /// mutation, so a malformed later receipt cannot consume a valid prefix or
    /// set the canonical event mask. Repeated valid receipts are all consumed;
    /// [`Self::queue_resource`] preserves retail's first-event timestamp.
    /// The caller admits ordinary native Section-13 construction (phase5),
    /// then calls [`Self::clear_native_load_slots`] before presentation.
    pub fn drain_fresh_level1_type9_attract_attention_receipts(
        &mut self,
        manager: &mut EntityManager,
    ) -> Result<usize, FreshLevel1Type9NotificationDrainError> {
        self.drain_attract_attention_receipts(manager, FRESH_LEVEL1_TYPE9_NOTIFICATION_LOAD_TICK)
    }

    /// Commit constructor receipts at the caller's actual gameplay timestamp.
    /// A live factory/Base birth calls this before its next callback phase;
    /// native loading uses the separate load wrapper and clears its slots.
    /// Both paths validate the complete batch before transferring custody.
    pub fn drain_attract_attention_receipts(
        &mut self,
        manager: &mut EntityManager,
        retail_tick: i32,
    ) -> Result<usize, FreshLevel1Type9NotificationDrainError> {
        for (receipt_index, &receipt) in manager
            .pending_fresh_level1_type9_resource_text_receipts()
            .iter()
            .enumerate()
        {
            if let Err(error) = validate_attract_attention_resource_text_receipt(receipt) {
                return Err(FreshLevel1Type9NotificationDrainError {
                    receipt_index,
                    receipt,
                    error,
                });
            }
        }

        let receipts = manager.take_pending_fresh_level1_type9_resource_text_receipts();
        let drained = receipts.len();
        for _receipt in receipts {
            self.queue_validated_attract_attention_resource_text(retail_tick);
        }
        Ok(drained)
    }

    /// Queue the exact three feedback contracts from the full-capacity branch:
    /// direct text 0xDA and deduplicated resource event 0x11. Positional sound
    /// 2 remains a world-audio concern at the call site.
    pub fn queue_cargo_full(&mut self, retail_tick: i32) {
        self.queue_direct(CARGO_FULL_TEXT_ID, 0, retail_tick);
        self.queue_resource(CARGO_FULL_HINT_EVENT_ID, retail_tick);
    }

    /// Queue type 68's successful-collection resource event 0x15. The cargo
    /// transfer particle and positional sound 8 are independent world effects.
    pub fn queue_weight_collected(&mut self, retail_tick: i32) {
        self.queue_resource(WEIGHT_COLLECTED_HINT_EVENT_ID, retail_tick);
    }

    ///443BEE requests this before the Type68-specific hint443C11.
    pub fn queue_cargo_collected(&mut self, retail_tick: i32) {
        self.queue_resource(CARGO_COLLECTED_HINT_EVENT_ID, retail_tick);
    }

    /// Queue generic-damage's successful player-kill event 4.
    ///
    /// `FUN_00414E90` owns the source-type and target-state predicates before
    /// calling `FUN_004568B0(4)`. This method owns only that call's resource
    /// slot and once-per-session deduplication; callers must retain those
    /// checked-damage predicates and executable order.
    pub fn queue_player_kill(&mut self, retail_tick: i32) {
        self.queue_resource(PLAYER_KILL_HINT_EVENT_ID, retail_tick);
    }

    /// Queue Main Base callback `FUN_004258A0`'s accepted-conversion resource
    /// event. Retail submits event 1 before deferred destruction and the
    /// fallible replacement spawn, so callers must not make this conditional
    /// on spawn success.
    pub fn queue_main_base_conversion(&mut self, retail_tick: i32) {
        self.queue_resource(MAIN_BASE_CONVERSION_RESOURCE_EVENT_ID, retail_tick);
    }

    /// Queue the accepted Working Factory delivery's event 5. This is the
    /// `FUN_004568B0` resource slot, not positional audio; operation `0x33`
    /// owns the independent world effect at the caller.
    pub fn queue_factory_delivery(&mut self, retail_tick: i32) {
        self.queue_resource(FACTORY_DELIVERY_HINT_EVENT_ID, retail_tick);
    }

    pub fn queue_spider_capture(&mut self, retail_tick: i32) {
        self.queue_resource(SPIDER_CAPTURE_HINT_EVENT_ID, retail_tick);
    }

    /// Queue `FUN_004147A0`'s successful infected firing hint. The firing
    /// caller owns the append, capability/state predicates, and phase-5 guard;
    /// this owner preserves event 9's first timestamp and session deduplication.
    pub fn queue_infected_firing_hint(&mut self, retail_tick: i32) {
        self.queue_resource(INFECTED_FIRING_HINT_EVENT_ID, retail_tick);
    }

    /// Queue the event-2 refresh emitted when staffing reaches capacity.
    pub fn queue_factory_capacity_reached(&mut self, retail_tick: i32) {
        self.queue_resource(FACTORY_CAPACITY_HINT_EVENT_ID, retail_tick);
    }

    /// Queue Working Factory phase 2's resource event `0x16`. Retail submits
    /// this before the fallible converted-output allocation, so callers must
    /// preserve it even when that allocation fails.
    pub fn queue_factory_output_conversion(&mut self, retail_tick: i32) {
        self.queue_resource(FACTORY_OUTPUT_CONVERSION_HINT_EVENT_ID, retail_tick);
    }

    /// Queue retail string `0xD1` after a successful product allocation whose
    /// authored production threshold is greater than four seconds.
    pub fn queue_factory_product_ready(&mut self, retail_tick: i32) {
        self.queue_direct(FACTORY_PRODUCT_READY_TEXT_ID, 0, retail_tick);
    }

    /// Queue Working Factory callback `FUN_00425C60`'s direct text `0xD3`.
    pub fn queue_factory_under_attack(&mut self, retail_tick: i32) {
        self.queue_direct(FACTORY_UNDER_ATTACK_TEXT_ID, 0, retail_tick);
    }

    /// Primary25D60's first newly observed Main Base attack (direct text C8).
    pub fn queue_main_base_under_attack(&mut self, retail_tick: i32) {
        self.queue_direct(0xc8, 0, retail_tick);
    }

    /// Queue retail string `0xCE` after repair or a visible staffing clamp
    /// reaches capacity.
    pub fn queue_factory_capacity_staffing(&mut self, retail_tick: i32) {
        self.queue_direct(FACTORY_CAPACITY_STAFFING_TEXT_ID, 0, retail_tick);
    }

    /// Queue controller operation `0x33`'s successful fuel-pickup feedback:
    /// parameterized direct text `Extra Fuel` and the once-per-session flight
    /// mode hint. Positional sound 5 remains a world-audio concern.
    pub fn queue_fuel_collected(&mut self, retail_tick: i32) {
        self.queue_direct(
            FUEL_COLLECTED_TEXT_ID,
            FUEL_COLLECTED_TEXT_ARGUMENT_ID,
            retail_tick,
        );
        self.queue_resource(FUEL_COLLECTED_HINT_EVENT_ID, retail_tick);
    }

    /// Queue controller operation `0x33`'s rejection text. Retail does not
    /// enqueue a resource hint or play the pickup sound on this branch.
    pub fn queue_fuel_full(&mut self, retail_tick: i32) {
        self.queue_direct(FUEL_FULL_TEXT_ID, 0, retail_tick);
    }

    /// Queue case `0x37`'s successful repair hint. Positional sound 5 is
    /// independent and remains a world-audio concern at the call site.
    pub fn queue_hull_repaired(&mut self, retail_tick: i32) {
        self.queue_resource(HULL_REPAIRED_HINT_EVENT_ID, retail_tick);
    }

    /// Queue case `0x37`'s full-health rejection. Retail keeps the pickup and
    /// submits neither its positional sound nor its resource hint.
    pub fn queue_hull_repair_full(&mut self, retail_tick: i32) {
        self.queue_direct(HULL_REPAIR_FULL_TEXT_ID, 0, retail_tick);
    }

    /// Queue case `0x35`'s direct parameterized `Shields` line. This pickup
    /// has no `FUN_004568B0` resource event and no positional sound.
    pub fn queue_shield_collected(&mut self, retail_tick: i32) {
        self.queue_direct(
            FUEL_COLLECTED_TEXT_ID,
            SHIELD_COLLECTED_TEXT_ARGUMENT_ID,
            retail_tick,
        );
    }

    /// Queue selector `0x36`'s parameterized pickup-counter line.
    pub fn queue_extra_life_collected(&mut self, retail_tick: i32) {
        self.queue_direct(
            POWER_UP_COLLECTED_TEXT_ID,
            EXTRA_LIFE_COLLECTED_TEXT_ARGUMENT_ID,
            retail_tick,
        );
    }

    /// Queue selector `0x3A`'s parameterized cargo/beam-capacity line.
    /// Retail emits it for both an increase and an already-satisfied target,
    /// but only when the player's authored attachment list exists.
    pub fn queue_cargo_capacity_collected(&mut self, retail_tick: i32) {
        self.queue_direct(
            POWER_UP_COLLECTED_TEXT_ID,
            CARGO_CAPACITY_COLLECTED_TEXT_ARGUMENT_ID,
            retail_tick,
        );
    }

    /// Queue the event 6/7 weapon hint. Keeping this separate from the direct
    /// text lets the caller retain retail's event -> sound -> text order.
    pub fn queue_weapon_pickup_hint(&mut self, resource_event_id: u8, retail_tick: i32) {
        self.queue_resource(resource_event_id, retail_tick);
    }

    /// Queue the descriptor-supplied `%s` line after accepted weapon sound.
    pub fn queue_weapon_collected_text(&mut self, text_argument_id: u16, retail_tick: i32) {
        self.queue_direct(
            POWER_UP_COLLECTED_TEXT_ID,
            usize::from(text_argument_id),
            retail_tick,
        );
    }

    /// Queue selector `0x3C`'s first-acquisition feedback in executable order.
    pub fn queue_targetter_collected(&mut self, retail_tick: i32) {
        self.queue_direct(
            POWER_UP_COLLECTED_TEXT_ID,
            TARGETTER_COLLECTED_TEXT_ARGUMENT_ID,
            retail_tick,
        );
        self.queue_resource(TARGETTER_COLLECTED_HINT_EVENT_ID, retail_tick);
    }

    /// Queue selector `0x3E`'s first-acquisition feedback. Unlike Targetter,
    /// this branch submits no deduplicated resource hint.
    pub fn queue_turbo_collected(&mut self, retail_tick: i32) {
        self.queue_direct(
            POWER_UP_COLLECTED_TEXT_ID,
            TURBO_COLLECTED_TEXT_ARGUMENT_ID,
            retail_tick,
        );
    }

    /// Queue `FUN_00456820(0)`'s first per-level CLAIMED transition.
    pub fn queue_trophy_level_claimed(&mut self, retail_tick: i32) {
        self.queue_direct(TROPHY_LEVEL_CLAIM_TEXT_ID, 0, retail_tick);
        self.queue_resource(TROPHY_LEVEL_CLAIM_HINT_EVENT_ID, retail_tick);
    }

    /// Queue the every-fifth-trophy reward line. When this follows a new
    /// level claim in the same transaction, it deliberately replaces 0xD0.
    pub fn queue_trophy_reward(&mut self, retail_tick: i32) {
        self.queue_direct(
            POWER_UP_COLLECTED_TEXT_ID,
            TROPHY_REWARD_TEXT_ARGUMENT_ID,
            retail_tick,
        );
    }

    /// 447280 local death: CA when controller+194 has extra lives, else D9.
    pub fn queue_player_destroyed(&mut self, extra_lives: u8, retail_tick: i32) {
        self.queue_direct(if extra_lives > 0 { 0xca } else { 0xd9 }, 0, retail_tick);
    }

    /// Queue `FUN_00446640`'s replaceable low-hull warning line.
    pub fn queue_hull_low(&mut self, retail_tick: i32) {
        self.queue_direct(HULL_LOW_TEXT_ID, 0, retail_tick);
    }

    /// Queue class-14's session-zero `FUN_00456900(0xC6, 0)` request.
    ///
    /// `FUN_00456900` stamps the direct slot with `(g_default_param * 1000) /
    /// 0x32` milliseconds, which the port stores as this 50-Hz tick. Passing
    /// tick 0 would make the authored 3000 ms duration expire after 150 ticks
    /// of already-elapsed world time.
    pub fn queue_type9_session_zero_class14(&mut self, retail_tick: i32) {
        self.queue_direct(TYPE9_SESSION_ZERO_CLASS14_TEXT_ID, 0, retail_tick);
    }

    /// Refresh the draw-time `FUN_0042DD10` selector-3 count.
    ///
    /// `FUN_00452270` case 2 calls `DAT_004fecdc(..., 3, controller)` while
    /// `FUN_00452790` formats `0xC6`, so this is the remaining live
    /// capability-`0x400` + `0x800` census, including factory Sub-M `+0x68`.
    pub fn set_people_left(&mut self, people_left: i32) {
        self.people_left = people_left;
    }

    /// Queue `FUN_00445310`'s replaceable low-fuel warning line.
    pub fn queue_fuel_low(&mut self, retail_tick: i32) {
        self.queue_direct(FUEL_LOW_TEXT_ID, 0, retail_tick);
    }

    /// Queue `FUN_0041BEB0`'s premature-lock resource event 10.
    ///
    /// Retail submits this when the hive is stamped back to one billion
    /// health after a hit while `FUN_00415120` is still true. Deduplication
    /// persists across campaign worlds.
    pub fn queue_hive_locked_hint(&mut self, retail_tick: i32) {
        self.queue_resource(HIVE_LOCKED_HINT_EVENT_ID, retail_tick);
    }

    /// Queue the unlock pair `FUN_00456900(0xD2)` then `FUN_004568B0(0xE)`.
    ///
    /// `FUN_00452CB0` draws the direct slot then the resource slot, so these
    /// appear as two stacked in-world rows. They are not overlay-51 results
    /// lines; those use `0xBD..0xC4`.
    pub fn queue_hive_now_vulnerable(&mut self, retail_tick: i32) {
        self.queue_direct(HIVE_NOW_VULNERABLE_TEXT_ID, 0, retail_tick);
        self.queue_resource(HIVE_UNLOCK_HINT_EVENT_ID, retail_tick);
    }

    /// Queue `FUN_0041BEB0`'s hive-death resource event 3 / string `0xE4`.
    ///
    /// Hive death leaves the HUD live, so its authored duration starts now.
    /// The later wreck-entry map does not restart this in-world hint.
    pub fn queue_hive_destroyed_hint(&mut self, retail_tick: i32) {
        self.queue_resource(HIVE_DESTROYED_HINT_EVENT_ID, retail_tick);
    }

    /// `0042DD10` casualty warning C7 or loss D4, before any systemic abort.
    /// The selector owns C7's per-world latch; the shared slot retains its
    /// normal sticky-D9 rule and does not create a resource notification.
    pub fn queue_campaign_casualty_text(
        &mut self,
        text: crate::campaign_failure::CampaignCasualtyText,
        retail_tick: i32,
    ) {
        self.queue_direct(text.string_id(), 0, retail_tick);
    }

    /// Queue `FUN_00445310`'s empty-at-entry resource event.
    ///
    /// Retail pairs this once-per-session hint with positional global sound
    /// `0x31`, sets the controller's mode-change request, and returns from the
    /// VTOL callback. The sound remains a world-audio concern at the call site.
    pub fn queue_fuel_empty(&mut self, retail_tick: i32) {
        self.queue_resource(FUEL_EMPTY_HINT_EVENT_ID, retail_tick);
    }

    fn queue_direct(&mut self, string_id: usize, sub_param: usize, retail_tick: i32) {
        // FUN_00437F30 replaces and restarts the direct slot except while the
        // sticky 0xD9 message is active. A zero id is its explicit clear.
        if self
            .direct
            .is_some_and(|slot| slot.string_id == STICKY_DIRECT_STRING_ID)
            && string_id != 0
        {
            return;
        }
        self.direct = (string_id != 0).then_some(NotificationSlot {
            string_id,
            sub_param,
            start_tick: retail_tick,
            cursor: false,
        });
    }

    fn queue_resource(&mut self, event_id: u8, retail_tick: i32) {
        let bit = 1u32.wrapping_shl(u32::from(event_id & 0x1f));
        if self.seen_resource_events & bit != 0 {
            return;
        }
        self.seen_resource_events |= bit;

        let Some(&string_id) = RESOURCE_EVENT_STRING_IDS.get(usize::from(event_id)) else {
            return;
        };
        // Event 0x0f has a literal zero table entry. Retail writes the slot but
        // the render-side resource-null guard suppresses it.
        if string_id == 0 {
            self.resource = None;
            return;
        }
        self.resource = Some(NotificationSlot {
            string_id,
            sub_param: 0,
            start_tick: retail_tick,
            cursor: true,
        });
    }

    fn queue_validated_attract_attention_resource_text(&mut self, retail_tick: i32) {
        self.queue_resource(ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT as u8, retail_tick);
    }

    /// Resolve and evaluate both slots at one process-global 50-Hz tick.
    ///
    /// `resolve_string` addresses the cumulative global Section-2 pool
    /// (`DAT_004FE628`), not the current level's local string indices.
    pub fn presentation<'a>(
        &mut self,
        retail_tick: i32,
        typewriter_cadence: &mut TextTypewriterCadence,
        mut resolve_string: impl FnMut(usize) -> Option<&'a str>,
    ) -> GameplayNotificationPresentation {
        let mut presentation = GameplayNotificationPresentation::default();
        for slot in [self.direct, self.resource].into_iter().flatten() {
            let Some(raw) = resolve_string(slot.string_id) else {
                continue;
            };
            let substitution = (slot.sub_param != 0)
                .then(|| resolve_string(slot.sub_param))
                .flatten();
            let elapsed_ms = retail_tick
                .wrapping_sub(slot.start_tick)
                .max(0)
                .saturating_mul(20);
            let cursor_visible = slot.cursor && ((retail_tick / 10) & 1) != 0;
            let Some(parsed) = parse_authored_notification(
                raw,
                substitution,
                self.people_left,
                elapsed_ms,
                retail_tick,
                cursor_visible,
            ) else {
                continue;
            };

            if parsed.reveal_incomplete {
                if typewriter_cadence.observe_active_line(true, retail_tick) {
                    presentation.play_typewriter_sound = true;
                }
            } else {
                // FUN_00452790 clears the shared cadence once an active line is
                // fully revealed. The following resource slot may therefore
                // start its own cue in the same HUD pass. Case 0xF blink-empty
                // still visits this complete-record clear.
                typewriter_cadence.observe_active_line(false, retail_tick);
            }
            if parsed.text.is_empty() {
                continue;
            }
            presentation.lines.push(GameplayNotificationLine {
                string_id: slot.string_id,
                text: parsed.text,
                x_percent: parsed.x_percent,
                baseline_percent: parsed.baseline_percent,
                width_percent: parsed.width_percent,
                center_x: parsed.center_x,
            });
        }
        presentation
    }
}

fn validate_attract_attention_resource_text_receipt(
    request: AttractAttentionResourceTextRequest,
) -> Result<(), AttractAttentionResourceTextReceiptError> {
    if request.event != ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT {
        return Err(AttractAttentionResourceTextReceiptError::UnexpectedEvent {
            actual: request.event,
        });
    }
    if request.global_resource_id != ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID {
        return Err(
            AttractAttentionResourceTextReceiptError::UnexpectedGlobalResourceId {
                actual: request.global_resource_id,
            },
        );
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedNotification {
    text: String,
    x_percent: i32,
    baseline_percent: i32,
    width_percent: i32,
    center_x: bool,
    reveal_incomplete: bool,
}

fn parse_authored_notification(
    raw: &str,
    substitution: Option<&str>,
    people_left: i32,
    elapsed_ms: i32,
    retail_tick: i32,
    cursor_visible: bool,
) -> Option<ParsedNotification> {
    let trimmed = raw.trim_start_matches([' ', '\t']);
    let (parameters, body) = if let Some(rest) = trimmed.strip_prefix('<') {
        let end = rest.find('>')?;
        (&rest[..end], &rest[end + 1..])
    } else {
        ("", trimmed)
    };
    let fields = parameters
        .split(',')
        .map(parse_prefix_integer)
        .collect::<Vec<_>>();
    let delay_ms = fields.first().copied().unwrap_or(0);
    let end_ms = fields.get(1).copied().unwrap_or(0);
    if elapsed_ms < delay_ms || (end_ms != 0 && end_ms < elapsed_ms) {
        return None;
    }

    let layout = fields.get(2).copied().unwrap_or(0);
    let (x_percent, baseline_percent, width_percent) = match layout {
        1 | 0x0b => (25, 87, 50),
        2 | 3 | 0x0c | 0x0d => (5, 16, 80),
        4 | 0x0e => (25, 78, 60),
        5 | 0x0f => (60, 7, 45),
        6 | 0x10 => (50, 11, 80),
        // The cargo contracts always author a preset. Retaining an explicit
        // zero layout makes malformed/missing resources non-fatal without
        // inventing uninitialized retail stack values.
        _ => (0, 0, 0),
    };

    // A nonzero layout preset jumps FUN_00452790's field cursor from slot 2
    // to slot 6, so the next token is the reveal interval and the fifth
    // comma-separated token is field 7 / `local_f8`. Selector 10 fetches the
    // queued sub-parameter as `%s`. Selector 2 is `FUN_00452270` case 2:
    // `DAT_004fecdc` selector 3 into `%d`. Selector 15 is case `0xF`: empty
    // the body while `|DAT_004FED60 / 40|` is even, and skip the blinking
    // cursor (`local_f8 == 0xf` forces param_6 off). Layouts strictly
    // greater than 9 then center through `FUN_00470f80`.
    let mut reveal_interval_ms = fields.get(3).copied().unwrap_or(0);
    if reveal_interval_ms == 0 {
        reveal_interval_ms = 1;
    }
    let formatter = fields.get(4).copied().unwrap_or(0);
    let formatted_body;
    let body = match formatter {
        10 => {
            formatted_body = body.replacen("%s", substitution?, 1);
            formatted_body.as_str()
        }
        2 => {
            formatted_body = body.replacen("%d", &people_left.to_string(), 1);
            formatted_body.as_str()
        }
        0xf if fun_00452270_case_e_hides_line(retail_tick) => {
            formatted_body = String::new();
            formatted_body.as_str()
        }
        _ => body,
    };
    let cursor_visible = cursor_visible && formatter != 0xf;
    let visible_bytes = elapsed_ms
        .saturating_sub(delay_ms)
        .checked_div(reveal_interval_ms)
        .unwrap_or(0)
        .max(0) as usize;
    let reveal_incomplete = reveal_interval_ms > 1 && visible_bytes < body.len();
    let mut text = if reveal_incomplete {
        // The authored gameplay strings are single-byte ASCII. FUN_00452790
        // truncates at this byte index (except for a '%' formatting escape,
        // which none of the cargo strings use).
        body[..visible_bytes.min(body.len())].to_owned()
    } else {
        body.to_owned()
    };
    if cursor_visible {
        // DAT_004D145C is the literal NUL-terminated string " _".
        text.push_str(" _");
    }

    Some(ParsedNotification {
        text,
        x_percent,
        baseline_percent,
        width_percent,
        center_x: layout > 9,
        reveal_incomplete,
    })
}

fn parse_prefix_integer(field: &str) -> i32 {
    let field = field.trim();
    if field.is_empty() || field == "*" {
        0
    } else {
        field.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: &str = "\t\t<*,3000, 4,30, *>Your cargo bay is full";
    const LIMITED: &str = "\t\t<*,3000, 1,30, *>There is only limited space in your cargo hold";
    const HEAVY: &str = "\t<*,3000, 1,30, *>This should make your ship heavier";
    const PLAYER_KILL_HINT: &str =
        "\t<*,3000, 1,30, *>Good shooting! Now kill all the other creatures to save the world";
    const HIVE_LOCKED_HINT: &str =
        "\t\t<*,3000, 1,30, *>The hive can only be destroyed once the alien creatures are dead";
    const HIVE_VULNERABLE: &str = "\t\t<*,3000, 4,30, *>The Hive is now vulnerable";
    const HIVE_UNLOCK_HINT: &str =
        "\t<*,4000, 1,30, *>It is now possible to destroy the alien hive";
    const HIVE_DESTROYED_HINT: &str =
        "\t\t<*,3000, 1,30, *>Fly down the hive to go to the next world";

    fn resolve(id: usize) -> Option<&'static str> {
        match id {
            0xda => Some(FULL),
            0xd2 => Some(HIVE_VULNERABLE),
            0xe2 => Some("\t<*,3000, 1,30, *>Your native has been converted"),
            0xe4 => Some(HIVE_DESTROYED_HINT),
            0xe5 => Some(PLAYER_KILL_HINT),
            0xeb => Some(HIVE_LOCKED_HINT),
            0xef => Some(HIVE_UNLOCK_HINT),
            0xf1 => Some(LIMITED),
            0xf5 => Some(HEAVY),
            _ => None,
        }
    }

    fn attract_attention_request() -> AttractAttentionResourceTextRequest {
        AttractAttentionResourceTextRequest {
            event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
            global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
        }
    }

    #[test]
    fn native_load_clears_old_clock_messages_but_preserves_consumed_hints() {
        let mut notifications = GameplayNotifications::new();
        notifications
            .queue_attract_attention_resource_text(attract_attention_request(), 4793)
            .unwrap();
        notifications.queue_cargo_collected(4793);
        notifications.queue_weight_collected(4793);
        notifications.queue_direct(STICKY_DIRECT_STRING_ID, 0, 4793);
        let consumed = notifications.seen_resource_events;
        notifications.clear_native_load_slots();
        assert!(notifications.direct.is_none());
        assert!(notifications.resource.is_none());
        assert_eq!(notifications.seen_resource_events, consumed);

        // Clock reset and recollection cannot refresh a consumed load hint.
        notifications.queue_weight_collected(1);
        notifications.queue_cargo_collected(1);
        notifications
            .queue_attract_attention_resource_text(attract_attention_request(), 1)
            .unwrap();
        assert!(notifications.resource.is_none());
        notifications.queue_cargo_full(2);
        assert_eq!(notifications.direct.unwrap().start_tick, 2);
        assert_eq!(notifications.resource.unwrap().string_id, 0xf1);

        // Frontend/session selection, unlike a campaign warp, resets the mask.
        notifications.reset_session();
        notifications.queue_weight_collected(3);
        assert_eq!(notifications.resource.unwrap().start_tick, 3);
    }

    #[test]
    fn attract_attention_resource_text_uses_f0_and_never_refreshes_event_10() {
        let mut notifications = GameplayNotifications::new();
        let request = attract_attention_request();

        notifications
            .queue_attract_attention_resource_text(request, 100)
            .unwrap();
        notifications
            .queue_attract_attention_resource_text(request, 120)
            .unwrap();
        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xf0,
                sub_param: 0,
                start_tick: 100,
                cursor: true,
            })
        );
        assert_eq!(
            notifications.seen_resource_events & (1 << ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT),
            1 << ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT
        );

        notifications.queue_weight_collected(130);
        let replacement = notifications.resource;
        notifications
            .queue_attract_attention_resource_text(request, 140)
            .unwrap();
        assert_eq!(
            notifications.resource, replacement,
            "a later event may replace the slot, but seen event 0x10 must not return"
        );

        notifications.reset_session();
        notifications
            .queue_attract_attention_resource_text(request, 200)
            .unwrap();
        assert_eq!(notifications.resource.unwrap().start_tick, 200);
    }

    #[test]
    fn attract_attention_resource_text_rejects_aliases_without_mutation() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_cargo_full(75);
        let before = notifications.clone();

        assert_eq!(
            notifications.queue_attract_attention_resource_text(
                AttractAttentionResourceTextRequest {
                    event: 0x110,
                    global_resource_id: ATTRACT_ATTENTION_RESOURCE_TEXT_GLOBAL_ID,
                },
                100,
            ),
            Err(AttractAttentionResourceTextReceiptError::UnexpectedEvent { actual: 0x110 })
        );
        assert_eq!(notifications, before);

        assert_eq!(
            notifications.queue_attract_attention_resource_text(
                AttractAttentionResourceTextRequest {
                    event: ATTRACT_ATTENTION_RESOURCE_TEXT_EVENT,
                    global_resource_id: 0xf1,
                },
                100,
            ),
            Err(
                AttractAttentionResourceTextReceiptError::UnexpectedGlobalResourceId {
                    actual: 0xf1,
                }
            )
        );
        assert_eq!(notifications, before);
    }

    #[test]
    fn weight_hint_uses_f5_and_deduplicates_for_the_session() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_weight_collected(100);
        notifications.queue_weight_collected(120);

        let frame = notifications.presentation(160, &mut cadence, resolve);
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xf5);
        // The ignored second queue did not restart the original 30-ms reveal.
        assert_eq!(frame.lines[0].text, HEAVY.rsplit_once('>').unwrap().1);

        notifications.reset_session();
        notifications.queue_weight_collected(200);
        // At the enqueue tick the typewriter has revealed no characters yet.
        assert!(notifications
            .presentation(200, &mut cadence, resolve)
            .lines
            .is_empty());
        assert_eq!(
            notifications
                .presentation(260, &mut cadence, resolve)
                .lines
                .len(),
            1
        );
    }

    #[test]
    fn infected_firing_hint_uses_ea_and_keeps_the_first_session_timestamp() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_infected_firing_hint(100);
        notifications.queue_infected_firing_hint(120);
        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xea,
                sub_param: 0,
                start_tick: 100,
                cursor: true,
            })
        );
        assert_eq!(notifications.save_tail_seen_mask(), 1 << 9);
        notifications.clear_native_load_slots();
        notifications.queue_infected_firing_hint(140);
        assert_eq!(notifications.resource, None);
        assert_eq!(notifications.save_tail_seen_mask(), 1 << 9);
        notifications.reset_session();
        notifications.queue_infected_firing_hint(160);
        assert_eq!(notifications.resource.unwrap().start_tick, 160);
    }

    #[test]
    fn hive_locked_hint_uses_eb_and_deduplicates_for_the_session() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_hive_locked_hint(100);
        notifications.queue_hive_locked_hint(120);

        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xeb,
                sub_param: 0,
                start_tick: 100,
                cursor: true,
            })
        );
        let mut cadence = TextTypewriterCadence::default();
        let frame = notifications.presentation(200, &mut cadence, resolve);
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xeb);
        assert_eq!(
            frame.lines[0].text,
            "The hive can only be destroyed once the alien creatures are dead"
        );
    }

    #[test]
    fn hive_destroyed_hint_uses_e4_without_restarting_its_live_hud_clock() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_hive_destroyed_hint(100);
        notifications.queue_hive_destroyed_hint(120);
        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xe4,
                sub_param: 0,
                start_tick: 100,
                cursor: true,
            })
        );
        let mut cadence = TextTypewriterCadence::default();
        assert!(notifications
            .presentation(100, &mut cadence, resolve)
            .lines
            .is_empty());
        let frame = notifications.presentation(165, &mut cadence, resolve);
        assert_eq!(frame.lines[0].string_id, 0xe4);
        assert_eq!(
            frame.lines[0].text,
            "Fly down the hive to go to the next world"
        );
    }

    #[test]
    fn hive_unlock_queues_direct_d2_then_resource_ef() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_hive_now_vulnerable(50);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: 0xd2,
                sub_param: 0,
                start_tick: 50,
                cursor: false,
            })
        );
        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xef,
                sub_param: 0,
                start_tick: 50,
                cursor: true,
            })
        );
    }

    #[test]
    fn player_kill_hint_uses_e5_without_refreshing_its_first_timestamp() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_player_kill(100);
        notifications.queue_player_kill(120);

        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xe5,
                sub_param: 0,
                start_tick: 100,
                cursor: true,
            })
        );
        assert_eq!(
            notifications.seen_resource_events & (1 << PLAYER_KILL_HINT_EVENT_ID),
            1 << PLAYER_KILL_HINT_EVENT_ID
        );

        let mut cadence = TextTypewriterCadence::default();
        let frame = notifications.presentation(200, &mut cadence, resolve);
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xe5);
        assert_eq!(
            frame.lines[0].text,
            "Good shooting! Now kill all the other creatures to save the world"
        );
    }

    #[test]
    fn main_base_conversion_queues_event_one_before_spawn_outcome_is_known() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_main_base_conversion(100);
        notifications.queue_main_base_conversion(120);

        let frame = notifications.presentation(160, &mut cadence, resolve);
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xe2);
        assert_eq!(frame.lines[0].text, "Your native has been converted");
    }

    #[test]
    fn cargo_full_presents_direct_and_deduplicated_slots_in_retail_order() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_cargo_full(100);

        let frame = notifications.presentation(102, &mut cadence, resolve);
        assert!(frame.play_typewriter_sound);
        assert_eq!(frame.lines.len(), 2);
        assert_eq!(frame.lines[0].string_id, 0xda);
        assert_eq!(frame.lines[0].text, "Y");
        assert_eq!(frame.lines[0].x_percent, 25);
        assert_eq!(frame.lines[0].baseline_percent, 78);
        assert_eq!(frame.lines[0].width_percent, 60);
        assert_eq!(frame.lines[1].string_id, 0xf1);
        assert_eq!(frame.lines[1].baseline_percent, 87);
        assert_eq!(frame.lines[1].width_percent, 50);
    }

    #[test]
    fn direct_slot_restarts_but_the_full_hint_does_not() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_cargo_full(100);
        notifications.queue_cargo_full(110);

        let frame = notifications.presentation(112, &mut cadence, resolve);
        assert_eq!(frame.lines[0].text, "Y");
        // Resource event 0x11 retained its first timestamp: 240 ms / 30 = 8.
        assert!(frame.lines[1].text.starts_with("There is"));
    }

    #[test]
    fn type9_session_zero_class14_queues_direct_c6() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_type9_session_zero_class14(100);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: TYPE9_SESSION_ZERO_CLASS14_TEXT_ID,
                sub_param: 0,
                start_tick: 100,
                cursor: false,
            })
        );
    }

    #[test]
    fn casualty_warning_and_loss_replace_direct_text_without_resource_hints() {
        use crate::campaign_failure::CampaignCasualtyText;

        let mut notifications = GameplayNotifications::new();
        for (text, string_id, tick) in [
            (CampaignCasualtyText::OneMoreLoss, 0xc7, 0x1741),
            (CampaignCasualtyText::WorldLost, 0xd4, 0x1dcb),
        ] {
            notifications.queue_campaign_casualty_text(text, tick);
            assert_eq!(
                notifications.direct,
                Some(NotificationSlot {
                    string_id,
                    sub_param: 0,
                    start_tick: tick,
                    cursor: false,
                })
            );
            assert!(notifications.resource.is_none());
        }
        notifications.queue_direct(STICKY_DIRECT_STRING_ID, 0, 8000);
        notifications.queue_campaign_casualty_text(CampaignCasualtyText::WorldLost, 8001);
        assert_eq!(
            notifications.direct.unwrap().string_id,
            STICKY_DIRECT_STRING_ID
        );
    }

    #[test]
    fn type9_people_left_formats_selector3_before_type_on() {
        const PEOPLE_LEFT: &str = "\t<*,3000, 4,30, 2>People left: %d";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_type9_session_zero_class14(100);
        notifications.set_people_left(19);

        let typing = notifications.presentation(102, &mut cadence, |id| {
            (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
        });
        assert_eq!(typing.lines.len(), 1);
        assert_eq!(
            typing.lines[0].string_id,
            TYPE9_SESSION_ZERO_CLASS14_TEXT_ID
        );
        assert_eq!(typing.lines[0].text, "P");
        assert_eq!(typing.lines[0].x_percent, 25);
        assert_eq!(typing.lines[0].baseline_percent, 78);

        notifications.set_people_left(6);
        let complete = notifications.presentation(250, &mut cadence, |id| {
            (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
        });
        assert_eq!(complete.lines[0].text, "People left: 6");
    }

    #[test]
    fn type9_people_left_duration_uses_queue_tick_not_level_epoch() {
        const PEOPLE_LEFT: &str = "\t<*,3000, 4,30, 2>People left: %d";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_type9_session_zero_class14(0);
        notifications.set_people_left(4);
        let expired = notifications.presentation(151, &mut cadence, |id| {
            (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
        });
        assert!(expired.lines.is_empty());

        notifications.queue_type9_session_zero_class14(150);
        let visible = notifications.presentation(152, &mut cadence, |id| {
            (id == TYPE9_SESSION_ZERO_CLASS14_TEXT_ID).then_some(PEOPLE_LEFT)
        });
        assert_eq!(visible.lines[0].text, "P");
    }

    #[test]
    fn sticky_direct_slot_requires_an_explicit_zero_clear() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_direct(STICKY_DIRECT_STRING_ID, 0, 100);
        notifications.queue_direct(CARGO_FULL_TEXT_ID, 0, 110);
        assert_eq!(
            notifications.direct.unwrap().string_id,
            STICKY_DIRECT_STRING_ID
        );

        notifications.queue_direct(0, 0, 120);
        assert!(notifications.direct.is_none());
        notifications.queue_direct(CARGO_FULL_TEXT_ID, 0, 130);
        assert_eq!(notifications.direct.unwrap().string_id, CARGO_FULL_TEXT_ID);
    }

    #[test]
    fn factory_product_ready_and_under_attack_use_distinct_direct_texts() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_factory_product_ready(100);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: FACTORY_PRODUCT_READY_TEXT_ID,
                sub_param: 0,
                start_tick: 100,
                cursor: false,
            })
        );

        notifications.queue_factory_under_attack(120);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: FACTORY_UNDER_ATTACK_TEXT_ID,
                sub_param: 0,
                start_tick: 120,
                cursor: false,
            })
        );
    }

    #[test]
    fn factory_output_conversion_uses_resource_event_16_and_deduplicates() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_factory_output_conversion(100);
        notifications.queue_factory_output_conversion(120);

        assert_eq!(
            notifications.resource,
            Some(NotificationSlot {
                string_id: 0xf6,
                sub_param: 0,
                start_tick: 100,
                cursor: true,
            })
        );
        assert_eq!(
            notifications.seen_resource_events & (1 << FACTORY_OUTPUT_CONVERSION_HINT_EVENT_ID),
            1 << FACTORY_OUTPUT_CONVERSION_HINT_EVENT_ID
        );
    }

    #[test]
    fn authored_three_second_end_is_inclusive() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_weight_collected(100);
        assert_eq!(
            notifications
                .presentation(250, &mut cadence, resolve)
                .lines
                .len(),
            1
        );
        assert!(notifications
            .presentation(251, &mut cadence, resolve)
            .lines
            .is_empty());
    }

    #[test]
    fn resource_slot_appends_the_retail_blink_cursor() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_weight_collected(100);
        let frame = notifications.presentation(110, &mut cadence, resolve);
        assert_eq!(frame.lines[0].text, "This s _");
    }

    #[test]
    fn typewriter_cue_uses_the_first_physical_global_sound_slot() {
        assert_eq!(GAMEPLAY_TEXT_TYPE_SOUND_ID, 0);
    }

    #[test]
    fn shared_typewriter_cadence_uses_strict_signed_tick_differences() {
        let mut cadence = TextTypewriterCadence::default();
        assert!(cadence.observe_active_line(true, 100));
        assert!(!cadence.observe_active_line(true, 101));
        assert!(!cadence.observe_active_line(true, 102));
        assert!(cadence.observe_active_line(true, 103));

        // DAT_004FED60 resets after a load, but DAT_004F72E8 does not. A line
        // left incomplete by an Intro2 skip therefore retains the signed
        // negative difference until the new clock passes the stored tick.
        assert!(!cadence.observe_active_line(true, 0));
        assert!(!cadence.observe_active_line(true, 105));
        assert!(cadence.observe_active_line(true, 106));

        // A complete active record is the operation that clears the field;
        // an inactive gap never calls this method.
        assert!(!cadence.observe_active_line(false, 106));
        assert!(cadence.observe_active_line(true, 109));
    }

    #[test]
    fn session_reset_does_not_own_the_process_global_typewriter_cadence() {
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        assert!(cadence.observe_active_line(true, 100));
        notifications.reset_session();
        assert!(!cadence.observe_active_line(true, 102));
        assert!(cadence.observe_active_line(true, 103));
    }

    #[test]
    fn fuel_collection_formats_direct_text_and_deduplicates_flight_hint() {
        const PARAMETERIZED: &str = "\t\t<*,3000, 4,30,10>%s";
        const EXTRA_FUEL: &str = "Extra Fuel";
        const FLIGHT_HINT: &str = "\t\t<*,3000, 1,30, *>Try changing to flight mode now";
        let resolve_fuel = |id| match id {
            FUEL_COLLECTED_TEXT_ID => Some(PARAMETERIZED),
            FUEL_COLLECTED_TEXT_ARGUMENT_ID => Some(EXTRA_FUEL),
            0xed => Some(FLIGHT_HINT),
            _ => None,
        };

        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_fuel_collected(100);
        notifications.queue_fuel_collected(110);
        let frame = notifications.presentation(112, &mut cadence, resolve_fuel);

        assert_eq!(frame.lines.len(), 2);
        assert_eq!(frame.lines[0].string_id, FUEL_COLLECTED_TEXT_ID);
        assert_eq!(frame.lines[0].text, "E");
        assert_eq!(frame.lines[1].string_id, 0xed);
        assert!(frame.lines[1].text.starts_with("Try chan"));
    }

    #[test]
    fn level_one_powerups_keep_their_distinct_direct_and_resource_contracts() {
        let mut notifications = GameplayNotifications::new();
        notifications.queue_weapon_pickup_hint(7, 100);
        notifications.queue_weapon_collected_text(0x0104, 100);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: POWER_UP_COLLECTED_TEXT_ID,
                sub_param: 0x0104,
                start_tick: 100,
                cursor: false,
            })
        );
        assert_eq!(notifications.resource.unwrap().string_id, 0xe8);

        notifications.queue_targetter_collected(110);
        assert_eq!(
            notifications.direct.unwrap().sub_param,
            TARGETTER_COLLECTED_TEXT_ARGUMENT_ID
        );
        assert_eq!(notifications.resource.unwrap().string_id, 0xf3);

        notifications.queue_turbo_collected(115);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: POWER_UP_COLLECTED_TEXT_ID,
                sub_param: TURBO_COLLECTED_TEXT_ARGUMENT_ID,
                start_tick: 115,
                cursor: false,
            })
        );
        assert_eq!(
            notifications.resource.unwrap().string_id,
            0xf3,
            "Turbo does not enqueue a new resource hint"
        );

        notifications.queue_trophy_level_claimed(120);
        notifications.queue_trophy_reward(120);
        assert_eq!(
            notifications.direct,
            Some(NotificationSlot {
                string_id: POWER_UP_COLLECTED_TEXT_ID,
                sub_param: TROPHY_REWARD_TEXT_ARGUMENT_ID,
                start_tick: 120,
                cursor: false,
            })
        );
        assert_eq!(notifications.resource.unwrap().string_id, 0xf4);
    }

    #[test]
    fn full_fuel_tank_queues_only_replaceable_direct_text() {
        const FULL_FUEL: &str = "\t\t<*,3000, 4,30, *>Your fuel tanks are full";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_fuel_full(100);

        let frame = notifications.presentation(102, &mut cadence, |id| {
            (id == FUEL_FULL_TEXT_ID).then_some(FULL_FUEL)
        });
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, FUEL_FULL_TEXT_ID);
        assert_eq!(frame.lines[0].text, "Y");
        assert_eq!(notifications.seen_resource_events, 0);
    }

    #[test]
    fn low_hull_and_fuel_warnings_share_the_replaceable_direct_slot() {
        const LOW_HULL: &str = "\t\t<*,3000, 4,30, *>Warning! Hull integrity low";
        const LOW_FUEL: &str = "\t\t<*,3000, 4,30, *>Warning! Fuel low";
        let resolve_warning = |id| match id {
            HULL_LOW_TEXT_ID => Some(LOW_HULL),
            FUEL_LOW_TEXT_ID => Some(LOW_FUEL),
            _ => None,
        };
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();

        notifications.queue_hull_low(100);
        let hull_frame = notifications.presentation(102, &mut cadence, resolve_warning);
        assert_eq!(hull_frame.lines.len(), 1);
        assert_eq!(hull_frame.lines[0].string_id, HULL_LOW_TEXT_ID);
        assert_eq!(hull_frame.lines[0].text, "W");

        notifications.queue_fuel_low(110);
        let fuel_frame = notifications.presentation(112, &mut cadence, resolve_warning);
        assert_eq!(fuel_frame.lines.len(), 1);
        assert_eq!(fuel_frame.lines[0].string_id, FUEL_LOW_TEXT_ID);
        assert_eq!(fuel_frame.lines[0].text, "W");
        assert_eq!(notifications.seen_resource_events, 0);
        assert!(!hull_frame.lines[0].center_x);
    }

    #[test]
    fn authored_hull_low_centers_and_blinks_on_formatter_f() {
        const LOW_HULL: &str = "\t\t<*,3000,14, *,15>Your ship is critically damaged";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_hull_low(100);

        let hidden = notifications.presentation(100, &mut cadence, |id| {
            (id == HULL_LOW_TEXT_ID).then_some(LOW_HULL)
        });
        assert!(hidden.lines.is_empty());
        assert!(!hidden.play_typewriter_sound);

        let shown = notifications.presentation(140, &mut cadence, |id| {
            (id == HULL_LOW_TEXT_ID).then_some(LOW_HULL)
        });
        assert_eq!(shown.lines.len(), 1);
        assert_eq!(shown.lines[0].text, "Your ship is critically damaged");
        assert!(shown.lines[0].center_x);
        assert_eq!(shown.lines[0].x_percent, 25);
        assert_eq!(shown.lines[0].baseline_percent, 78);
        assert_eq!(shown.lines[0].width_percent, 60);
        assert!(!shown.play_typewriter_sound);
    }

    #[test]
    fn empty_vtol_fuel_uses_event_0b_and_authored_string_0ec_once_per_session() {
        const EMPTY_FUEL: &str = "\t<*,3000, 1,30, *>You cannot fly without any fuel";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();

        notifications.queue_fuel_empty(100);
        notifications.queue_fuel_empty(110);
        let frame =
            notifications.presentation(250, &mut cadence, |id| (id == 0xec).then_some(EMPTY_FUEL));

        assert_eq!(FUEL_EMPTY_HINT_EVENT_ID, 0x0b);
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xec);
        assert_eq!(frame.lines[0].text, "You cannot fly without any fuel _");
        assert_eq!(
            notifications.seen_resource_events,
            1 << FUEL_EMPTY_HINT_EVENT_ID
        );

        notifications.reset_session();
        notifications.queue_fuel_empty(200);
        assert_eq!(
            notifications.resource.unwrap().string_id,
            RESOURCE_EVENT_STRING_IDS[usize::from(FUEL_EMPTY_HINT_EVENT_ID)]
        );
    }

    #[test]
    fn accepted_hull_repair_queues_only_the_deduplicated_repair_hint() {
        const REPAIR_HINT: &str =
            "\t\t<*,3000, 1,30, *>This repairs some of the damage to your ship";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_hull_repaired(100);
        notifications.queue_hull_repaired(110);

        let frame = notifications.presentation(112, &mut cadence, |id| {
            (id == RESOURCE_EVENT_STRING_IDS[usize::from(HULL_REPAIRED_HINT_EVENT_ID)])
                .then_some(REPAIR_HINT)
        });
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xee);
        assert!(frame.lines[0].text.starts_with("This rep"));
    }

    #[test]
    fn rejected_hull_repair_queues_only_the_direct_full_health_text() {
        const NO_DAMAGE: &str = "\t\t<*,3000, 4,30, *>Your ship has no damage";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_hull_repair_full(100);

        let frame = notifications.presentation(102, &mut cadence, |id| {
            (id == HULL_REPAIR_FULL_TEXT_ID).then_some(NO_DAMAGE)
        });
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, HULL_REPAIR_FULL_TEXT_ID);
        assert_eq!(frame.lines[0].text, "Y");
        assert_eq!(notifications.seen_resource_events, 0);
    }

    #[test]
    fn shield_collection_formats_only_the_parameterized_direct_text() {
        const PARAMETERIZED: &str = "\t\t<*,3000, 4,30,10>%s";
        const SHIELDS: &str = "Shields";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_shield_collected(100);

        let frame = notifications.presentation(102, &mut cadence, |id| match id {
            FUEL_COLLECTED_TEXT_ID => Some(PARAMETERIZED),
            SHIELD_COLLECTED_TEXT_ARGUMENT_ID => Some(SHIELDS),
            _ => None,
        });
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, FUEL_COLLECTED_TEXT_ID);
        assert_eq!(frame.lines[0].text, "S");
        assert_eq!(notifications.seen_resource_events, 0);
    }

    #[test]
    fn life_and_cargo_capacity_use_their_exact_parameterized_arguments() {
        const PARAMETERIZED: &str = "\t\t<*,3000, 4,30,10>%s";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();

        notifications.queue_extra_life_collected(100);
        let life = notifications.presentation(102, &mut cadence, |id| match id {
            POWER_UP_COLLECTED_TEXT_ID => Some(PARAMETERIZED),
            EXTRA_LIFE_COLLECTED_TEXT_ARGUMENT_ID => Some("Pickup counter incremented"),
            _ => None,
        });
        assert_eq!(life.lines.len(), 1);
        assert_eq!(life.lines[0].string_id, POWER_UP_COLLECTED_TEXT_ID);
        assert_eq!(life.lines[0].text, "P");

        notifications.queue_cargo_capacity_collected(200);
        let cargo = notifications.presentation(202, &mut cadence, |id| match id {
            POWER_UP_COLLECTED_TEXT_ID => Some(PARAMETERIZED),
            CARGO_CAPACITY_COLLECTED_TEXT_ARGUMENT_ID => Some("Beam power set"),
            _ => None,
        });
        assert_eq!(cargo.lines.len(), 1);
        assert_eq!(cargo.lines[0].string_id, POWER_UP_COLLECTED_TEXT_ID);
        assert_eq!(cargo.lines[0].text, "B");
        assert_eq!(notifications.seen_resource_events, 0);
    }

    #[test]
    fn missing_parameter_string_suppresses_only_the_direct_fuel_line() {
        const PARAMETERIZED: &str = "\t\t<*,3000, 4,30,10>%s";
        const FLIGHT_HINT: &str = "\t\t<*,3000, 1,30, *>Try changing to flight mode now";
        let mut notifications = GameplayNotifications::new();
        let mut cadence = TextTypewriterCadence::default();
        notifications.queue_fuel_collected(100);

        let frame = notifications.presentation(102, &mut cadence, |id| match id {
            FUEL_COLLECTED_TEXT_ID => Some(PARAMETERIZED),
            0xed => Some(FLIGHT_HINT),
            _ => None,
        });
        assert_eq!(frame.lines.len(), 1);
        assert_eq!(frame.lines[0].string_id, 0xed);
    }
}
