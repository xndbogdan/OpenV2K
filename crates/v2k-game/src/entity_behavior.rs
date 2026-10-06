//! Data-driven retail entity-behavior selection and initializer state policy.
//!
//! Section 12 `+0x118` is a weighted list of named behavior classes. Retail
//! resolves those classes through the executable table at `0x004C8AA0`, then
//! installs the descriptor's initial 0x48-byte style through
//! `FUN_0040EA10`. Keeping that indirection here prevents entity types from
//! becoming a second, hand-maintained behavior dispatch table.

use v2k_formats::collision::BehaviorChoice;

use crate::entity_collision_state::RetailRuntimeValue;

/// Maximum candidate count in `FUN_00425680`'s fixed local weight array.
pub const MAX_BEHAVIOR_CHOICES: usize = 8;

/// One exact 0x48-byte behavior-style record relevant to impact, release,
/// live-pair, and death dispatch. Behavior identity and current style are
/// separate because retail can switch variants after initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviorStyle {
    pub class_id: u8,
    pub variant: u8,
    pub frame_address: u32,
    /// Style `+0x0C`, reached through type-vtable `+0x48` / `FUN_0040DC50`
    /// after `FUN_00416750` releases an entity from its current relation.
    pub release_callback_address: Option<u32>,
    /// Style `+0x18`, reached through type-vtable `+0x38` / `FUN_0040D8D0`
    /// during `FUN_00411AD0` active-entity pair contact. This is separate from
    /// the component-state `+0x18` callback invoked later in the same pass.
    pub pair_contact_callback_address: Option<u32>,
    /// Style `+0x28`, reached through type-vtable `+0x14` /
    /// `FUN_0040DAC0` before `FUN_00411030` applies the primary-hit reaction
    /// and before checked damage mutates health.
    pub impact_callback_address: Option<u32>,
    /// Style `+0x2C`, reached by the default type death callback
    /// `FUN_0040DB80`. This callback runs before the standard dying/weighted
    /// behavior continuation and may clear the dying state instead of allowing
    /// generic removal.
    pub death_callback_address: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseCallbackPolicy {
    None,
    TerrainAlignAndReselect,
    EnablePairAndReselect,
    UnknownAddress(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCallbackPolicy {
    None,
    BaseFactoryProgression,
    CapturePeopleCleanup,
    UnknownAddress(u32),
}

/// Address-level classification of the behavior-style `+0x28` callback.
///
/// `FUN_0040C690` enters the weighted behavior selector and consumes the
/// process-global RNG stream on the live, non-dying hit path. Capture People
/// variants 2--5 instead use `FUN_0040D040`, whose relation cleanup can
/// conditionally reach that same selector. Keeping both programs distinct is
/// required to preserve their different state and RNG prerequisites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactCallbackPolicy {
    None,
    ReselectBehavior,
    CapturePeopleCleanup,
    UnknownAddress(u32),
}

/// Address-level classification of the behavior-style `+0x18` callback.
///
/// This is deliberately a policy rather than a dispatcher. The callbacks
/// below require different live state, and an unknown retained address must
/// never be treated as a null callback merely because its program has not yet
/// been ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairContactCallbackPolicy {
    None,
    CapturePeople,
    LifterDelivery,
    MainBaseConversion,
    Hive,
    PowerUp,
    PlayerContact,
    UnknownAddress(u32),
}

impl BehaviorStyle {
    /// Classify without dispatching the two release callbacks whose behavior
    /// is recovered. Unknown addresses remain lossless and cannot silently run.
    pub const fn release_callback_policy(self) -> ReleaseCallbackPolicy {
        match self.release_callback_address {
            None => ReleaseCallbackPolicy::None,
            Some(0x0040_D1C0) => ReleaseCallbackPolicy::TerrainAlignAndReselect,
            Some(0x0040_CE90) => ReleaseCallbackPolicy::EnablePairAndReselect,
            Some(address) => ReleaseCallbackPolicy::UnknownAddress(address),
        }
    }

    /// Classify without dispatching the two recovered style-death programs.
    /// A later unaudited variant remains unresolved before this method is
    /// reached; an unknown retained address cannot silently run.
    pub const fn death_callback_policy(self) -> DeathCallbackPolicy {
        match self.death_callback_address {
            None => DeathCallbackPolicy::None,
            Some(0x0041_9750) => DeathCallbackPolicy::BaseFactoryProgression,
            Some(0x0040_D040) => DeathCallbackPolicy::CapturePeopleCleanup,
            Some(address) => DeathCallbackPolicy::UnknownAddress(address),
        }
    }

    /// Classify the pre-health impact callback without dispatching it or
    /// claiming ownership of the shared evaluator/RNG stream.
    pub const fn impact_callback_policy(self) -> ImpactCallbackPolicy {
        match self.impact_callback_address {
            None => ImpactCallbackPolicy::None,
            Some(0x0040_C690) => ImpactCallbackPolicy::ReselectBehavior,
            Some(0x0040_D040) => ImpactCallbackPolicy::CapturePeopleCleanup,
            Some(address) => ImpactCallbackPolicy::UnknownAddress(address),
        }
    }

    /// Classify the pair callback without running it or assuming that two
    /// callbacks with similar return objects share the same state program.
    pub const fn pair_contact_callback_policy(self) -> PairContactCallbackPolicy {
        match self.pair_contact_callback_address {
            None => PairContactCallbackPolicy::None,
            Some(0x0040_C910) => PairContactCallbackPolicy::CapturePeople,
            Some(0x0042_5850) => PairContactCallbackPolicy::LifterDelivery,
            Some(0x0042_58A0) => PairContactCallbackPolicy::MainBaseConversion,
            Some(0x0042_59F0) => PairContactCallbackPolicy::Hive,
            Some(0x0042_5AF0) => PairContactCallbackPolicy::PowerUp,
            Some(0x0044_7D70) => PairContactCallbackPolicy::PlayerContact,
            Some(address) => PairContactCallbackPolicy::UnknownAddress(address),
        }
    }
}

/// One named behavior's executable descriptor and exact initial style. Later
/// variants are catalogued separately where their
/// release/pair/impact/death semantics have been audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviorProgram {
    pub class_id: u8,
    pub name: &'static str,
    pub descriptor_address: u32,
    /// First byte of the descriptor's 0x48-byte style table. This is kept
    /// separate from the semantic style catalog because the live context owns
    /// a raw table index.
    pub style_table_base_address: u32,
    /// Descriptor dword `+0x04`, copied into live context allocation `+0x18`
    /// before the initial style is installed.
    pub initial_style_table_index_raw: u32,
    pub initial_style: BehaviorStyle,
    /// Raw style `+0x34`; translated by `FUN_0040D440` as the normal policy.
    pub authored_enable_policy: u32,
    /// Raw style `+0x38`; translated with set/clear orientation reversed.
    pub authored_disable_policy: u32,
    /// Style `+0x40`, invoked after the style and masks are installed.
    pub initializer_callback_address: u32,
    /// Initial style `+0x44`, passed as the initializer's second argument.
    /// This is independently audited because zero is a meaningful value.
    pub initializer_argument_raw: RetailRuntimeValue<u32>,
    /// Unconditional host-state edits made by the initializer beyond style
    /// masks. Environment-dependent setup effects are retained separately.
    pub unconditional_initializer_state_set_bits: u32,
    pub unconditional_initializer_state_clear_bits: u32,
    /// Bits set only after the initializer successfully creates its authored
    /// component/model dependency. This is currently nonzero only for the gun
    /// turret; applying it before setup succeeds would invent live state.
    pub component_setup_success_state_set_bits: u32,
}

pub const INITIALIZER_FAILURE_FALLBACK_DESCRIPTOR_ADDRESS: u32 = 0x004C_8888;
pub const INITIALIZER_FAILURE_FALLBACK_STYLE_ADDRESS: u32 = 0x004C_74F8;
pub const INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY: u32 = 0x0000_1280;
pub const INITIALIZER_FAILURE_FALLBACK_DISABLE_POLICY: u32 = 0;
pub const INITIALIZER_FAILURE_FALLBACK_INITIALIZER_ADDRESS: u32 = 0x0040_C4D0;

/// Descriptor identity retained by the allocated behavior context.
///
/// The initializer-failure descriptor at `0x004C8888` is not one of the
/// named programs in the executable table and must not be represented by a
/// fabricated class id (including class zero's authored `"None"` program).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorDescriptorIdentity {
    Named(&'static BehaviorProgram),
    InitializerFailureFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorChoiceListSource {
    TypeDefault,
}

/// Exact active style identity without assigning the unnamed fallback a fake
/// [`BehaviorStyle::class_id`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveBehaviorStyle {
    Audited(BehaviorStyle),
    InitializerFailureFallback,
}

impl ActiveBehaviorStyle {
    pub const fn style_address(self) -> u32 {
        match self {
            Self::Audited(style) => style.frame_address,
            Self::InitializerFailureFallback => INITIALIZER_FAILURE_FALLBACK_STYLE_ADDRESS,
        }
    }

    pub const fn audited(self) -> Option<BehaviorStyle> {
        match self {
            Self::Audited(style) => Some(style),
            Self::InitializerFailureFallback => None,
        }
    }

    pub const fn release_callback_policy(self) -> ReleaseCallbackPolicy {
        match self {
            Self::Audited(style) => style.release_callback_policy(),
            Self::InitializerFailureFallback => ReleaseCallbackPolicy::None,
        }
    }

    pub const fn death_callback_policy(self) -> DeathCallbackPolicy {
        match self {
            Self::Audited(style) => style.death_callback_policy(),
            Self::InitializerFailureFallback => DeathCallbackPolicy::None,
        }
    }

    pub const fn impact_callback_policy(self) -> ImpactCallbackPolicy {
        match self {
            Self::Audited(style) => style.impact_callback_policy(),
            Self::InitializerFailureFallback => ImpactCallbackPolicy::None,
        }
    }

    pub const fn pair_contact_callback_policy(self) -> PairContactCallbackPolicy {
        match self {
            Self::Audited(style) => style.pair_contact_callback_policy(),
            Self::InitializerFailureFallback => PairContactCallbackPolicy::None,
        }
    }
}

/// Live behavior-context authority corresponding to the allocation installed
/// through `FUN_0040ABB0` and the style published through `FUN_0040EA10`.
///
/// The raw descriptor-local style-table index is stored at behavior-context
/// `+0x10`. Target handle `+0x08` and auxiliary word `+0x0C` are independent
/// context state which descriptor reselection preserves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviorContextRuntime {
    descriptor: BehaviorDescriptorIdentity,
    choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
    target_handle_at_0x08: RetailRuntimeValue<Option<u32>>,
    auxiliary_word_at_0x0c: RetailRuntimeValue<u32>,
    style_table_index_raw_at_0x10: u32,
    active_style: ActiveBehaviorStyle,
}

impl BehaviorContextRuntime {
    /// Bind a weighted selection only after the caller has independently
    /// established that its fallible initializer published the selected
    /// descriptor rather than the unnamed fallback.
    pub(crate) fn from_published_weighted_selection(selection: BehaviorSelection) -> Option<Self> {
        if behavior_program(u32::from(selection.program.class_id)) != Some(selection.program) {
            return None;
        }
        Self::named_audited(
            selection.program,
            selection.program.initial_style_table_index_raw,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            selection.program.initial_style,
        )
    }

    /// Fresh constructor context after the owning weighted selector has run
    /// and the selected initializer is about to publish its first style.
    ///
    /// `FUN_0040ABE0` seeds the type-default choice-list pointer, the null
    /// entity handle, and the zero auxiliary word before entering
    /// `FUN_0040C6B0`.  This is deliberately distinct from an observed
    /// post-construction selection, whose private context words cannot be
    /// recovered from selection identity alone.
    pub(crate) fn from_fresh_weighted_selection(selection: BehaviorSelection) -> Option<Self> {
        if behavior_program(u32::from(selection.program.class_id)) != Some(selection.program) {
            return None;
        }
        Self::named_audited(
            selection.program,
            selection.program.initial_style_table_index_raw,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            selection.program.initial_style,
        )
    }

    /// Fresh constructor context for the complete statically infallible
    /// initializer set (currently only class-23 Power Up). `FUN_0040ABE0`
    /// seeds the type-default source, null target, and zero auxiliary word
    /// before invoking that initializer.
    pub(crate) fn from_infallible_initial_selection(selection: BehaviorSelection) -> Option<Self> {
        if !initializer_success_is_identity_determined(selection.program)
            || behavior_program(u32::from(selection.program.class_id)) != Some(selection.program)
        {
            return None;
        }
        Self::named_audited(
            selection.program,
            selection.program.initial_style_table_index_raw,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(None),
            RetailRuntimeValue::Known(0),
            selection.program.initial_style,
        )
    }

    /// Construct a named audited style while keeping the descriptor-local raw
    /// table index independent from [`BehaviorStyle::variant`].
    pub fn named_audited(
        program: &'static BehaviorProgram,
        style_table_index_raw: u32,
        choice_list_source: RetailRuntimeValue<BehaviorChoiceListSource>,
        target_handle_at_0x08: RetailRuntimeValue<Option<u32>>,
        auxiliary_word_at_0x0c: RetailRuntimeValue<u32>,
        style: BehaviorStyle,
    ) -> Option<Self> {
        let canonical = audited_behavior_style(u32::from(style.class_id), style.variant)?;
        if audited_behavior_program(u32::from(program.class_id)) != Some(program) {
            return None;
        }
        let style_address = style_table_index_raw
            .checked_mul(0x48)
            .and_then(|offset| program.style_table_base_address.checked_add(offset));
        if style.class_id != program.class_id
            || style != *canonical
            || style_address != Some(style.frame_address)
        {
            return None;
        }
        Some(Self {
            descriptor: BehaviorDescriptorIdentity::Named(program),
            choice_list_source,
            target_handle_at_0x08,
            auxiliary_word_at_0x0c,
            style_table_index_raw_at_0x10: style_table_index_raw,
            active_style: ActiveBehaviorStyle::Audited(style),
        })
    }

    pub const fn initializer_failure_fallback() -> Self {
        Self {
            descriptor: BehaviorDescriptorIdentity::InitializerFailureFallback,
            choice_list_source: RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            target_handle_at_0x08: RetailRuntimeValue::Unresolved,
            auxiliary_word_at_0x0c: RetailRuntimeValue::Unresolved,
            style_table_index_raw_at_0x10: 0,
            active_style: ActiveBehaviorStyle::InitializerFailureFallback,
        }
    }

    /// Apply outer `FUN_0040C6B0` failure replacement on the same allocated
    /// context. `FUN_0040ABB0` replaces descriptor/source/index identity but
    /// preserves the target and auxiliary context words.
    pub const fn with_initializer_failure_fallback(self) -> Self {
        Self {
            descriptor: BehaviorDescriptorIdentity::InitializerFailureFallback,
            choice_list_source: RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            target_handle_at_0x08: self.target_handle_at_0x08,
            auxiliary_word_at_0x0c: self.auxiliary_word_at_0x0c,
            style_table_index_raw_at_0x10: 0,
            active_style: ActiveBehaviorStyle::InitializerFailureFallback,
        }
    }

    /// Reuse one allocated context for a named program selected from the
    /// entity type's default behavior-choice list. Target and auxiliary words
    /// survive the descriptor/style replacement exactly as in
    /// `FUN_0040ABB0`.
    pub fn reselect_named_type_default(
        self,
        program: &'static BehaviorProgram,
        style_table_index_raw: u32,
        style: BehaviorStyle,
    ) -> Option<Self> {
        if behavior_program(u32::from(program.class_id)) != Some(program) {
            return None;
        }
        Self::named_audited(
            program,
            style_table_index_raw,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            self.target_handle_at_0x08,
            self.auxiliary_word_at_0x0c,
            style,
        )
    }

    /// Replace the descriptor/style with any statically audited named
    /// program selected through the entity type's default behavior source.
    /// Target and auxiliary words remain attached to the allocated context.
    ///
    /// This is the alternate-behavior counterpart to
    /// [`Self::reselect_named_type_default`]. It deliberately does not force
    /// the weighted catalog: audited classes 2 and 12 are reached through
    /// Section-12 `+0x124` of that same type-default source.
    pub(crate) fn reselect_audited_type_default(
        self,
        program: &'static BehaviorProgram,
        style_table_index_raw: u32,
        style: BehaviorStyle,
    ) -> Option<Self> {
        if audited_behavior_program(u32::from(program.class_id)) != Some(program) {
            return None;
        }
        Self::named_audited(
            program,
            style_table_index_raw,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            self.target_handle_at_0x08,
            self.auxiliary_word_at_0x0c,
            style,
        )
    }

    pub const fn descriptor(self) -> BehaviorDescriptorIdentity {
        self.descriptor
    }

    pub const fn descriptor_address(self) -> u32 {
        match self.descriptor {
            BehaviorDescriptorIdentity::Named(program) => program.descriptor_address,
            BehaviorDescriptorIdentity::InitializerFailureFallback => {
                INITIALIZER_FAILURE_FALLBACK_DESCRIPTOR_ADDRESS
            }
        }
    }

    pub const fn choice_list_source(self) -> RetailRuntimeValue<BehaviorChoiceListSource> {
        self.choice_list_source
    }

    pub const fn target_handle_at_0x08(self) -> RetailRuntimeValue<Option<u32>> {
        self.target_handle_at_0x08
    }

    pub const fn auxiliary_word_at_0x0c(self) -> RetailRuntimeValue<u32> {
        self.auxiliary_word_at_0x0c
    }

    pub const fn style_table_index_raw_at_0x10(self) -> u32 {
        self.style_table_index_raw_at_0x10
    }

    pub const fn active_style(self) -> ActiveBehaviorStyle {
        self.active_style
    }
}

/// Complete census of non-null release callbacks at `+0x0C` in every initial
/// style reachable from the cumulative retail Section-12 pool. All six share
/// the reset-and-reselect callback `FUN_0040D1C0`.
const fn initial_release_callback(class_id: u8) -> Option<u32> {
    match class_id {
        0 | 29 | 42 | 59 | 60 | 68 => Some(0x0040_D1C0),
        _ => None,
    }
}

/// Complete census of non-null `+0x2C` callbacks in every reachable initial
/// style. Main Base and Working Factory share the progression callback; it can
/// restore health and clear dying, so neither may use generic removal.
const fn initial_death_callback(class_id: u8) -> Option<u32> {
    match class_id {
        39 | 41 => Some(0x0041_9750),
        _ => None,
    }
}

/// Complete executable census of initial-style `+0x28` callbacks.
const fn initial_impact_callback(class_id: u8) -> Option<u32> {
    match class_id {
        4 | 5 | 6 | 7 | 9 | 13 | 15 | 21 | 26 | 32 | 33 | 54 | 58 | 69 => Some(0x0040_C690),
        67 => Some(0x0042_1990),
        _ => None,
    }
}

macro_rules! behavior {
    ($id:literal, $name:literal, $descriptor:literal, $frame:literal,
     $enable:literal, $disable:literal, $pair_contact:literal, $init:literal) => {
        BehaviorProgram {
            class_id: $id,
            name: $name,
            descriptor_address: $descriptor,
            style_table_base_address: $frame,
            initial_style_table_index_raw: 0,
            initial_style: BehaviorStyle {
                class_id: $id,
                variant: 0,
                frame_address: $frame,
                release_callback_address: initial_release_callback($id),
                pair_contact_callback_address: if $pair_contact == 0 {
                    None
                } else {
                    Some($pair_contact)
                },
                impact_callback_address: initial_impact_callback($id),
                death_callback_address: initial_death_callback($id),
            },
            authored_enable_policy: $enable,
            authored_disable_policy: $disable,
            initializer_callback_address: $init,
            initializer_argument_raw: RetailRuntimeValue::Unresolved,
            unconditional_initializer_state_set_bits: 0,
            unconditional_initializer_state_clear_bits: 0,
            component_setup_success_state_set_bits: 0,
        }
    };
}

/// Class11's primary has no timeout. Its terrain/static/water style hooks
/// select variant1, whose BAC0 explosion retires the allocation.
pub const TUMBLE_OUT_OF_SKY_BEHAVIOR_PROGRAM: BehaviorProgram = BehaviorProgram {
    initializer_argument_raw: RetailRuntimeValue::Known(0),
    ..behavior!(
        11,
        "Tumble Out Of Sky",
        0x004C88D8,
        0x004C7F60,
        0,
        0x95,
        0,
        0x0040C660
    )
};

pub const TUMBLE_OUT_OF_SKY_COMPLETION_STYLE: BehaviorStyle = BehaviorStyle {
    class_id: 11,
    variant: 1,
    frame_address: 0x004C_7FA8,
    release_callback_address: None,
    pair_contact_callback_address: None,
    impact_callback_address: None,
    death_callback_address: None,
};

/// Class12 is reached through Section12's alternate slot rather than its
/// weighted-choice list, so it lives in the separate audited catalog.
pub const COMMON_ACTOR_DYING_BEHAVIOR_PROGRAM: BehaviorProgram = behavior!(
    12,
    "Flip Over And Die",
    0x004C88D0,
    0x004C7ED0,
    0,
    0x00002015,
    0,
    0x0040C620
);

/// Audited class-14 alternate selected by ordinary Type-9 generic death.
///
/// `FUN_0040C3A0` resets Sub-I, clears the entity's `0x8000` state bit, and
/// publishes a 1,000-ms shared-retarget Primary task. The task owner's
/// terminal callback selects variant one, whose `FUN_0040C470` initializer
/// clears every task slot and stages the shared deferred destroy.
pub const EXPLODING_PERSON_BEHAVIOR_PROGRAM: BehaviorProgram = BehaviorProgram {
    initializer_argument_raw: RetailRuntimeValue::Known(0),
    unconditional_initializer_state_clear_bits: 0x0000_8000,
    ..behavior!(
        14,
        "Exploding Person",
        0x004C8868,
        0x004C70C0,
        0,
        0,
        0,
        0x0040C3A0
    )
};

/// Audited class-49 alternate selected by Type 61's generic-death
/// continuation. Its initial style has no release, pair, impact, or death
/// callback and applies no state policy; `FUN_0040BD20` synchronously emits
/// the explosion/radial bundle, clears the actor tasks, attempts the Type-60
/// ring allocation, and stages deferred removal.
pub const EXPLODE_WITH_RING_BEHAVIOR_PROGRAM: BehaviorProgram = BehaviorProgram {
    initializer_argument_raw: RetailRuntimeValue::Known(0),
    ..behavior!(
        49,
        "Explode With Ring",
        0x004C8830,
        0x004C71E0,
        0,
        0,
        0,
        0x0040BD20
    )
};

/// Type46's alternate rule1/class25. Descriptor4CDB48 starts at the
/// Player Control table's Dying bounce frame4CDAA8.447280 publishes the
/// BB8/4476F0 timed object after its synchronous wreck burst.
pub const PLAYER_DYING_BEHAVIOR_PROGRAM: BehaviorProgram = BehaviorProgram {
    initializer_argument_raw: RetailRuntimeValue::Known(0),
    ..behavior!(
        25,
        "Dying bounce",
        0x004CDB48,
        0x004CDAA8,
        0,
        0,
        0x00447D70,
        0x00447280
    )
};

/// Class-1 generic explosion selected by Type34's alternate rule 3. BAC0
/// emits BAF0's particle/radial bundle before clearing tasks and deferring removal.
pub const EXPLODE_BEHAVIOR_PROGRAM: BehaviorProgram =
    behavior!(1, "Explode", 0x004C8820, 0x004C7150, 0, 0, 0, 0x0040BAC0);

/// Audited class-2 alternate selected by the ordinary generic-death
/// trampoline. Its 0x48-byte initial style is entirely zero except for the
/// `FUN_0040C470` initializer at `+0x40`.
pub const QUIET_DEATH_BEHAVIOR_PROGRAM: BehaviorProgram = behavior!(
    2,
    "Die Quietly",
    0x004C8870,
    0x004C7420,
    0,
    0,
    0,
    0x0040C470
);

/// PE class18 descriptor4C8860 selects style4C73D8. Its only nonzero
/// style word is initializer40C080 at+40; the initializer owns synchronous
/// Type56 split births and deferred parent destruction.
pub const SPLIT_AND_EXPLODE_BEHAVIOR_PROGRAM: BehaviorProgram = behavior!(
    18,
    "Split And Explode",
    0x004C8860,
    0x004C73D8,
    0,
    0,
    0,
    0x0040C080
);

pub const QUIET_DEATH_STYLE: BehaviorStyle = QUIET_DEATH_BEHAVIOR_PROGRAM.initial_style;

/// Exact class-12 variant installed by `FUN_0040C620` after the lethal setup
/// transaction succeeds.
pub const COMMON_ACTOR_DYING_ACTIVE_STYLE: BehaviorStyle =
    COMMON_ACTOR_DYING_BEHAVIOR_PROGRAM.initial_style;

/// Exact terminal class-12 variant selected by `FUN_0040C750(..., 1)`.
pub const COMMON_ACTOR_DYING_COMPLETION_STYLE: BehaviorStyle = BehaviorStyle {
    class_id: 12,
    variant: 1,
    frame_address: 0x004C_7F18,
    release_callback_address: None,
    pair_contact_callback_address: None,
    impact_callback_address: None,
    death_callback_address: None,
};

/// Exact terminal class-14 variant selected by `FUN_0040C750(..., 1)`.
pub const EXPLODING_PERSON_COMPLETION_STYLE: BehaviorStyle = BehaviorStyle {
    class_id: 14,
    variant: 1,
    frame_address: 0x004C_7108,
    release_callback_address: None,
    pair_contact_callback_address: None,
    impact_callback_address: None,
    death_callback_address: None,
};

/// Named executable programs reached only through dedicated transitions rather
/// than the ordinary weighted behavior-choice selector.
pub static AUDITED_ALTERNATE_BEHAVIOR_PROGRAMS: &[BehaviorProgram] = &[
    EXPLODE_BEHAVIOR_PROGRAM,
    QUIET_DEATH_BEHAVIOR_PROGRAM,
    TUMBLE_OUT_OF_SKY_BEHAVIOR_PROGRAM,
    COMMON_ACTOR_DYING_BEHAVIOR_PROGRAM,
    EXPLODING_PERSON_BEHAVIOR_PROGRAM,
    SPLIT_AND_EXPLODE_BEHAVIOR_PROGRAM,
    PLAYER_DYING_BEHAVIOR_PROGRAM,
    EXPLODE_WITH_RING_BEHAVIOR_PROGRAM,
];

/// Every behavior class referenced by the cumulative retail Section-12
/// `+0x118` weighted-choice lists. All referenced descriptors start at variant
/// zero.
pub static REACHABLE_BEHAVIOR_PROGRAMS: &[BehaviorProgram] = &[
    behavior!(0, "None", 0x004C8878, 0x004C7468, 0x00002001, 0, 0, 0x0040C490),
    behavior!(
        4,
        "Defecate Virus",
        0x004C88C8,
        0x004C7E88,
        0,
        0,
        0,
        0x0040B9E0
    ),
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0),
        ..behavior!(
            5,
            "Move About Aimlessly",
            0x004C8898,
            0x004C7930,
            0,
            0,
            0,
            0x0040ACD0
        )
    },
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0),
        ..behavior!(
            6,
            "Wander Near Location",
            0x004C88A0,
            0x004C79C0,
            0,
            0x00021000,
            0,
            0x0040AD10
        )
    },
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0),
        ..behavior!(
            7,
            "Search And Attack Target",
            0x004C88A8,
            0x004C7A50,
            0,
            0x00021080,
            0,
            0x0040B6C0
        )
    },
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0x0000_0C00),
        ..behavior!(
            9,
            "Capture People",
            0x004C88C0,
            0x004C7FF0,
            0,
            0,
            0,
            0x0040B6C0
        )
    },
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0),
        ..behavior!(10, "Run Away", 0x004C88E0, 0x004C7618, 0, 0, 0, 0x0040B6C0)
    },
    behavior!(13, "Flocking", 0x004C88B8, 0x004C7DF8, 0, 0, 0, 0x0040B640),
    behavior!(
        15,
        "Trash Buildings",
        0x004C88E8,
        0x004C76F0,
        0,
        0,
        0,
        0x0040B7C0
    ),
    behavior!(
        16,
        "Generate Flares",
        0x004C88F0,
        0x004C77C8,
        0,
        0,
        0,
        0x0040B8B0
    ),
    behavior!(
        19,
        "Boulder Trailing Fire",
        0x004C8900,
        0x004C7858,
        0,
        0,
        0,
        0x0040B910
    ),
    behavior!(
        20,
        "Rolling Boulder",
        0x004C8908,
        0x004C78A0,
        0,
        0x00001005,
        0,
        0x0040B950
    ),
    behavior!(
        21,
        "Mobbing Target",
        0x004C8910,
        0x004C7C48,
        0,
        0,
        0,
        0x0040B460
    ),
    behavior!(
        22,
        "Powered Missile",
        0x004C8918,
        0x004C81A0,
        0,
        0,
        0x0040CEF0,
        0x0040B010
    ),
    behavior!(23, "Power Up", 0x004C9738, 0x004C96C0, 0, 0, 0x00425AF0, 0x004257F0),
    behavior!(
        24,
        "Player Control",
        0x004CDB38,
        0x004CD940,
        0x00010010,
        0,
        0x00447D70,
        0x004464F0
    ),
    behavior!(
        26,
        "Trash Furniture",
        0x004C8948,
        0x004C7738,
        0,
        0,
        0,
        0x0040B7C0
    ),
    behavior!(
        28,
        "Guided Missile",
        0x004C8920,
        0x004C81E8,
        0x00004000,
        0,
        0x0040CEF0,
        0x0040D160
    ),
    BehaviorProgram {
        component_setup_success_state_set_bits: 0x0000_0028,
        ..behavior!(
            29,
            "Gun Turret",
            0x004C8928,
            0x004C8230,
            0x00004027,
            0,
            0,
            0x0040D190
        )
    },
    behavior!(
        30,
        "Materialiser",
        0x004C8930,
        0x004C82C0,
        0x00004000,
        0,
        0,
        0x0040D1E0
    ),
    behavior!(31, "MindBall", 0x004C8940, 0x004C8278, 0x00004080, 0x00040000, 0, 0x0040D2A0),
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0),
        ..behavior!(
            32,
            "Guard Location",
            0x004C8958,
            0x004C7BB8,
            0,
            0,
            0,
            0x0040B5A0
        )
    },
    behavior!(
        33,
        "Follow Beacons",
        0x004C8960,
        0x004C7B28,
        0,
        0,
        0,
        0x0040B740
    ),
    behavior!(
        34,
        "Proximity Mine",
        0x004C8968,
        0x004C8548,
        0,
        0,
        0x0040CEF0,
        0x0040B080
    ),
    behavior!(
        35,
        "Standard Mine",
        0x004C8970,
        0x004C8350,
        0,
        0,
        0x0040CEF0,
        0x0040B250
    ),
    behavior!(
        36,
        "Die When Moved",
        0x004C8858,
        0x004C7390,
        0,
        0,
        0,
        0x0040B980
    ),
    BehaviorProgram {
        unconditional_initializer_state_clear_bits: 0x0000_0800,
        ..behavior!(
            38,
            "Change Sea Level",
            0x004C8980,
            0x004C7348,
            0,
            0,
            0,
            0x0040BDC0
        )
    },
    behavior!(
        39,
        "Working Factory",
        0x004C9708,
        0x004C9558,
        0,
        0,
        0x00425850,
        0x004257C0
    ),
    behavior!(
        41,
        "Main Base",
        0x004C9718,
        0x004C9480,
        0,
        0,
        0x004258A0,
        0x00425730
    ),
    behavior!(
        42,
        "Cleansing Landscape",
        0x004C8988,
        0x004C85D8,
        0,
        0,
        0,
        0x0040AD50
    ),
    BehaviorProgram {
        initializer_argument_raw: RetailRuntimeValue::Known(0x0000_0201),
        ..behavior!(
            45,
            "Attract Attention",
            0x004C8998,
            0x004C86B0,
            0,
            0,
            0,
            0x0040BA40
        )
    },
    behavior!(
        46,
        "Alien Hive",
        0x004C9728,
        0x004C94C8,
        0,
        0,
        0x004259F0,
        0x00425760
    ),
    behavior!(
        48,
        "Exploding Ring",
        0x004C89A0,
        0x004C8398,
        0,
        0,
        0,
        0x0040B290
    ),
    behavior!(
        50,
        "Defender Ball",
        0x004C89A8,
        0x004C83E0,
        0,
        0,
        0x0040C800,
        0x0040B0F0
    ),
    behavior!(
        53,
        "Defender Drone",
        0x004C89C0,
        0x004C8500,
        0,
        0,
        0,
        0x0040B200
    ),
    behavior!(54, "Go to job", 0x004C89C8, 0x004C8788, 0, 0, 0, 0x0040AF90),
    behavior!(55, "Megablast", 0x004C89D0, 0x004C84B8, 0, 0, 0, 0x0040B140),
    behavior!(
        56,
        "Intro Sequence",
        0x004CA930,
        0x004CA8E8,
        0,
        0,
        0,
        0x0042BEE0
    ),
    behavior!(
        58,
        "Tractor Beam",
        0x004C8840,
        0x004C7228,
        0,
        0,
        0,
        0x0040B500
    ),
    behavior!(
        59,
        "Remote Beamer",
        0x004C8848,
        0x004C72B8,
        0,
        0,
        0x0040CAF0,
        0x0040C530
    ),
    behavior!(
        60,
        "Portable Radar",
        0x004C8850,
        0x004C7300,
        0,
        0,
        0,
        0x0040C560
    ),
    behavior!(65, "Geyser", 0x004C8890, 0x004C7540, 0, 0, 0, 0x0040C500),
    behavior!(
        66,
        "Passive Player",
        0x004CDB40,
        0x004CD8F8,
        0x00010010,
        0,
        0x00447D70,
        0x00446EF0
    ),
    behavior!(67, "Queen", 0x004C8938, 0x004C8308, 0x00004000, 0, 0, 0x0040D210),
    behavior!(
        68,
        "Stationary",
        0x004C8880,
        0x004C74B0,
        0x0000A023,
        0,
        0,
        0x0040C490
    ),
    behavior!(
        69,
        "Shark Attack",
        0x004C89D8,
        0x004C7D20,
        0,
        0,
        0,
        0x0040D2D0
    ),
];

/// Non-initial variants whose release/pair/impact/death slots have been audited
/// from the executable. This is deliberately not presented as a complete
/// variant catalog; an absent later variant remains unresolved.
pub static AUDITED_NON_INITIAL_BEHAVIOR_STYLES: &[BehaviorStyle] = &[
    BehaviorStyle {
        class_id: 6,
        variant: 1,
        frame_address: 0x004C_7A08,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 7,
        variant: 1,
        frame_address: 0x004C_7A98,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_C690),
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 7,
        variant: 2,
        frame_address: 0x004C_7AE0,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 9,
        variant: 1,
        frame_address: 0x004C_8038,
        release_callback_address: None,
        pair_contact_callback_address: Some(0x0040_C910),
        impact_callback_address: Some(0x0040_C690),
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 9,
        variant: 2,
        frame_address: 0x004C_8080,
        release_callback_address: None,
        pair_contact_callback_address: Some(0x0040_D0B0),
        impact_callback_address: Some(0x0040_D040),
        death_callback_address: Some(0x0040_D040),
    },
    BehaviorStyle {
        class_id: 9,
        variant: 3,
        frame_address: 0x004C_80C8,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_D040),
        death_callback_address: Some(0x0040_D040),
    },
    BehaviorStyle {
        class_id: 9,
        variant: 4,
        frame_address: 0x004C_8110,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_D040),
        death_callback_address: Some(0x0040_D040),
    },
    BehaviorStyle {
        class_id: 9,
        variant: 5,
        frame_address: 0x004C_8158,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_D040),
        death_callback_address: Some(0x0040_D040),
    },
    BehaviorStyle {
        class_id: 10,
        variant: 1,
        frame_address: 0x004C_7660,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 10,
        variant: 2,
        frame_address: 0x004C_76A8,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    TUMBLE_OUT_OF_SKY_COMPLETION_STYLE,
    COMMON_ACTOR_DYING_COMPLETION_STYLE,
    BehaviorStyle {
        class_id: 13,
        variant: 1,
        frame_address: 0x004C_7E40,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_C690),
        death_callback_address: None,
    },
    EXPLODING_PERSON_COMPLETION_STYLE,
    BehaviorStyle {
        class_id: 32,
        variant: 1,
        frame_address: 0x004C_7C00,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_C690),
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 33,
        variant: 1,
        frame_address: 0x004C_7B70,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_C690),
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 39,
        variant: 4,
        frame_address: 0x004C_9678,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 42,
        variant: 1,
        frame_address: 0x004C_8620,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 45,
        variant: 1,
        frame_address: 0x004C_86F8,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: Some(0x0040_C690),
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 45,
        variant: 2,
        frame_address: 0x004C_8740,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 46,
        variant: 1,
        frame_address: 0x004C_9510,
        release_callback_address: None,
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
    BehaviorStyle {
        class_id: 54,
        variant: 1,
        frame_address: 0x004C_87D0,
        release_callback_address: Some(0x0040_CE90),
        pair_contact_callback_address: None,
        impact_callback_address: None,
        death_callback_address: None,
    },
];

pub fn behavior_program(class_id: u32) -> Option<&'static BehaviorProgram> {
    let class_id = u8::try_from(class_id).ok()?;
    REACHABLE_BEHAVIOR_PROGRAMS
        .binary_search_by_key(&class_id, |program| program.class_id)
        .ok()
        .map(|index| &REACHABLE_BEHAVIOR_PROGRAMS[index])
}

/// Resolve a named executable behavior descriptor from either the ordinary
/// Section-12 pool or a statically audited dedicated transition.
pub fn audited_behavior_program(class_id: u32) -> Option<&'static BehaviorProgram> {
    let class_id = u8::try_from(class_id).ok()?;
    behavior_program(class_id.into()).or_else(|| {
        AUDITED_ALTERNATE_BEHAVIOR_PROGRAMS
            .binary_search_by_key(&class_id, |program| program.class_id)
            .ok()
            .map(|index| &AUDITED_ALTERNATE_BEHAVIOR_PROGRAMS[index])
    })
}

/// Whether selection identity alone proves that the selected descriptor was
/// retained after initialization. Retail `FUN_004257F0` and demo
/// `FUN_004256C0` (Power Up) contain only fixed writes and an explicit zero
/// return. Every other currently catalogued initializer can propagate a
/// helper/task-allocation failure into the outer unnamed fallback.
pub const fn initializer_success_is_identity_determined(program: &BehaviorProgram) -> bool {
    program.class_id == 23
}

/// Resolve only style records whose complete release/pair/impact/death slots
/// have been audited. Variant zero comes from the named behavior's initial
/// record; unknown later variants stay absent rather than inheriting variant
/// zero.
pub fn audited_behavior_style(class_id: u32, variant: u8) -> Option<&'static BehaviorStyle> {
    let class_id = u8::try_from(class_id).ok()?;
    if variant == 0 {
        return audited_behavior_program(class_id.into()).map(|program| &program.initial_style);
    }
    AUDITED_NON_INITIAL_BEHAVIOR_STYLES
        .iter()
        .find(|style| style.class_id == class_id && style.variant == variant)
}

/// Evaluator identities resolved from the executable table at `0x004C8CDC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorWeightRule {
    Always,
    UnderAttack,
    Mutated,
    PlayerNearby,
    BaddieNearby,
    FurnitureNearby,
    BuildingsNearby,
    PeopleNearby,
    BeaconNearby,
    BaseNearby,
    JobNearby,
}

impl BehaviorWeightRule {
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match raw {
            1 => Some(Self::Always),
            2 => Some(Self::UnderAttack),
            5 => Some(Self::Mutated),
            6 => Some(Self::PlayerNearby),
            7 => Some(Self::BaddieNearby),
            8 => Some(Self::FurnitureNearby),
            9 => Some(Self::BuildingsNearby),
            10 => Some(Self::PeopleNearby),
            11 => Some(Self::BeaconNearby),
            12 => Some(Self::BaseNearby),
            13 => Some(Self::JobNearby),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorSelectionError {
    TooManyChoices { count: usize },
    UnknownWeightRule { raw: u32 },
    UnknownBehaviorClass { raw: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BehaviorSelection {
    pub choice_index: usize,
    pub program: &'static BehaviorProgram,
}

/// What can be known about initial behavior identity without evaluating the
/// world or advancing retail's process-global RNG stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RngIndependentInitialBehavior {
    Known(Option<BehaviorSelection>),
    RuntimeDependent,
}

/// Resolve only behavior identities invariant across evaluator and RNG state.
///
/// Retail still consumes one random word for an empty list and for a single
/// positive `Always` choice. This helper deliberately does not emulate that
/// side effect; it exists only so entity construction can retain identities
/// already forced by authored data without perturbing the shared RNG stream.
pub fn resolve_rng_independent_initial_behavior(
    choices: &[BehaviorChoice],
) -> Result<RngIndependentInitialBehavior, BehaviorSelectionError> {
    if choices.len() > MAX_BEHAVIOR_CHOICES {
        return Err(BehaviorSelectionError::TooManyChoices {
            count: choices.len(),
        });
    }
    if choices.is_empty() {
        return Ok(RngIndependentInitialBehavior::Known(None));
    }

    for choice in choices {
        BehaviorWeightRule::from_raw(choice.weight_rule_id).ok_or(
            BehaviorSelectionError::UnknownWeightRule {
                raw: choice.weight_rule_id,
            },
        )?;
    }

    let [choice] = choices else {
        return Ok(RngIndependentInitialBehavior::RuntimeDependent);
    };
    if BehaviorWeightRule::from_raw(choice.weight_rule_id) != Some(BehaviorWeightRule::Always) {
        return Ok(RngIndependentInitialBehavior::RuntimeDependent);
    }

    let signed_weight = choice.weight_multiplier as i32;
    if signed_weight == 0 {
        return Ok(RngIndependentInitialBehavior::Known(None));
    }
    if signed_weight < 0 {
        return Ok(RngIndependentInitialBehavior::RuntimeDependent);
    }

    let program = behavior_program(choice.behavior_class_id).ok_or(
        BehaviorSelectionError::UnknownBehaviorClass {
            raw: choice.behavior_class_id,
        },
    )?;
    Ok(RngIndependentInitialBehavior::Known(Some(
        BehaviorSelection {
            choice_index: 0,
            program,
        },
    )))
}

/// Exact weighted selection performed by `FUN_00425680`.
///
/// The RNG callback is unconditional, including a single deterministic
/// `Always` candidate. An empty/zero-weight list returns `Ok(None)` after that
/// draw, matching the selector's fall-through to its terminator record.
pub fn select_initial_behavior(
    choices: &[BehaviorChoice],
    mut evaluate: impl FnMut(BehaviorWeightRule) -> i32,
    mut next_random: impl FnMut() -> u32,
) -> Result<Option<BehaviorSelection>, BehaviorSelectionError> {
    if choices.len() > MAX_BEHAVIOR_CHOICES {
        return Err(BehaviorSelectionError::TooManyChoices {
            count: choices.len(),
        });
    }

    let mut weights = [0_i32; MAX_BEHAVIOR_CHOICES];
    let mut total = 0_i32;
    for (index, choice) in choices.iter().enumerate() {
        let rule = BehaviorWeightRule::from_raw(choice.weight_rule_id).ok_or(
            BehaviorSelectionError::UnknownWeightRule {
                raw: choice.weight_rule_id,
            },
        )?;
        let weight = evaluate(rule).wrapping_mul(choice.weight_multiplier as i32);
        weights[index] = weight;
        total = total.wrapping_add(weight);
    }
    if total > 0x7fff {
        total = 0x7fff;
    }

    let random = next_random();
    let threshold = (random & 0xffff).wrapping_mul(total as u32) as i32 >> 16;
    let mut cumulative = 0_i32;
    let Some(choice_index) = weights[..choices.len()].iter().position(|weight| {
        cumulative = cumulative.wrapping_add(*weight);
        threshold < cumulative
    }) else {
        return Ok(None);
    };
    let behavior_class_id = choices[choice_index].behavior_class_id;
    let program = behavior_program(behavior_class_id).ok_or(
        BehaviorSelectionError::UnknownBehaviorClass {
            raw: behavior_class_id,
        },
    )?;
    Ok(Some(BehaviorSelection {
        choice_index,
        program,
    }))
}

/// The two masks emitted by `FUN_0040D440` for one authored policy word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TranslatedStatePolicy {
    pub set_bits: u32,
    pub clear_bits: u32,
}

pub const fn translate_state_policy(authored: u32) -> TranslatedStatePolicy {
    let mut policy = TranslatedStatePolicy {
        set_bits: 0,
        clear_bits: 0,
    };
    if authored & 0x000001 != 0 {
        policy.clear_bits |= 0x0001_0000;
    }
    if authored & 0x000080 != 0 {
        policy.clear_bits |= 0x0000_8000;
    }
    if authored & 0x000100 != 0 {
        policy.clear_bits |= 0x0000_0800;
    }
    if authored & 0x000200 != 0 {
        policy.clear_bits |= 0x0002_0000;
    }
    if authored & 0x001000 != 0 {
        policy.clear_bits |= 0x0004_0000;
    }
    if authored & 0x008000 != 0 {
        policy.set_bits |= 0x0008_0000;
    }
    if authored & 0x020000 != 0 {
        policy.set_bits |= 0x0800_0000;
    }
    policy
}

pub const fn apply_type_state_policy(state: u32, authored: u32) -> u32 {
    let policy = translate_state_policy(authored);
    (state | policy.set_bits) & !policy.clear_bits
}

/// Apply `FUN_0040EA10`'s normal `+0x34` translation, reversed `+0x38`
/// translation, and proven unconditional initializer edits.
pub const fn apply_initial_behavior_state(state: u32, program: &BehaviorProgram) -> u32 {
    let enabled = translate_state_policy(program.authored_enable_policy);
    let disabled = translate_state_policy(program.authored_disable_policy);
    let state = (state | enabled.set_bits) & !enabled.clear_bits;
    let state = (state | disabled.clear_bits) & !disabled.set_bits;
    (state | program.unconditional_initializer_state_set_bits)
        & !program.unconditional_initializer_state_clear_bits
}

/// Collapse the ordered style/initializer writes into one masked state edit.
/// Set and clear masks are disjoint and preserve all unrelated input bits.
pub const fn initial_behavior_state_policy(program: &BehaviorProgram) -> TranslatedStatePolicy {
    TranslatedStatePolicy {
        set_bits: apply_initial_behavior_state(0, program),
        clear_bits: !apply_initial_behavior_state(u32::MAX, program),
    }
}

/// Apply the state edit owned by successful component/model setup inside an
/// initializer. Retail does not perform this edit when setup fails.
pub const fn apply_component_setup_success_state(state: u32, program: &BehaviorProgram) -> u32 {
    state | program.component_setup_success_state_set_bits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(rule: u32, weight: u32, class: u32) -> BehaviorChoice {
        BehaviorChoice {
            weight_rule_id: rule,
            weight_multiplier: weight,
            behavior_class_id: class,
        }
    }

    #[test]
    fn executable_program_catalog_is_sorted_unique_and_complete_for_retail_choices() {
        assert!(REACHABLE_BEHAVIOR_PROGRAMS
            .windows(2)
            .all(|pair| pair[0].class_id < pair[1].class_id));
        assert!(AUDITED_NON_INITIAL_BEHAVIOR_STYLES.windows(2).all(|pair| {
            (pair[0].class_id, pair[0].variant) < (pair[1].class_id, pair[1].variant)
        }));
        for id in [0, 6, 23, 24, 39, 41, 46, 69] {
            assert_eq!(behavior_program(id).unwrap().class_id, id as u8);
        }
    }

    #[test]
    fn audited_b6c0_initial_arguments_distinguish_capture_from_run_away() {
        let aimless = behavior_program(5).expect("Move About Aimlessly program");
        let search_attack = behavior_program(7).expect("Search And Attack Target program");
        let capture = behavior_program(9).expect("Capture People program");
        let run_away = behavior_program(10).expect("Run Away program");

        assert_eq!(aimless.initializer_callback_address, 0x0040_ACD0);
        assert_eq!(
            aimless.initializer_argument_raw,
            RetailRuntimeValue::Known(0),
            "class-5 style +0x44 is absent from the EXE non-zero field list"
        );
        assert_eq!(search_attack.initializer_callback_address, 0x0040_B6C0);
        assert_eq!(
            search_attack.initializer_argument_raw,
            RetailRuntimeValue::Known(0),
            "class-7 style +0x44 is absent from the EXE non-zero field list"
        );
        assert_eq!(capture.initializer_callback_address, 0x0040_B6C0);
        assert_eq!(
            capture.initializer_argument_raw,
            RetailRuntimeValue::Known(0x0C00)
        );
        assert_eq!(run_away.initializer_callback_address, 0x0040_B6C0);
        assert_eq!(
            run_away.initializer_argument_raw,
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn attract_attention_retains_the_proven_initial_style_argument() {
        let attract = behavior_program(45).expect("Attract Attention program");

        assert_eq!(attract.initializer_callback_address, 0x0040_BA40);
        assert_eq!(
            attract.initializer_argument_raw,
            RetailRuntimeValue::Known(0x0201)
        );
    }

    #[test]
    fn type47_birth_styles_retain_zero_initializer_arguments() {
        let wander = behavior_program(6).expect("Wander Near Location program");
        let guard = behavior_program(32).expect("Guard Location program");

        assert_eq!(wander.initializer_callback_address, 0x0040_AD10);
        assert_eq!(
            wander.initializer_argument_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(guard.initializer_callback_address, 0x0040_B5A0);
        assert_eq!(guard.initializer_argument_raw, RetailRuntimeValue::Known(0));
    }

    #[test]
    fn common_dying_descriptor_is_audited_without_joining_the_weighted_choice_pool() {
        assert!(behavior_program(12).is_none());
        assert!(AUDITED_ALTERNATE_BEHAVIOR_PROGRAMS
            .windows(2)
            .all(|pair| pair[0].class_id < pair[1].class_id));
        assert!(AUDITED_ALTERNATE_BEHAVIOR_PROGRAMS.iter().all(|alternate| {
            !REACHABLE_BEHAVIOR_PROGRAMS
                .iter()
                .any(|weighted| weighted.class_id == alternate.class_id)
        }));

        let program = audited_behavior_program(12).expect("class-12 alternate descriptor");
        assert_eq!(program.name, "Flip Over And Die");
        assert_eq!(program.descriptor_address, 0x004C_88D0);
        assert_eq!(program.style_table_base_address, 0x004C_7ED0);
        assert_eq!(program.initializer_callback_address, 0x0040_C620);
        assert_eq!(
            audited_behavior_style(12, 0),
            Some(&COMMON_ACTOR_DYING_ACTIVE_STYLE)
        );
        assert_eq!(
            audited_behavior_style(12, 1),
            Some(&COMMON_ACTOR_DYING_COMPLETION_STYLE)
        );
        let completion = BehaviorContextRuntime::named_audited(
            program,
            1,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            COMMON_ACTOR_DYING_COMPLETION_STYLE,
        )
        .expect("class-12 variant 1 is raw descriptor index 1");
        assert_eq!(completion.style_table_index_raw_at_0x10(), 1);
        assert!(BehaviorContextRuntime::initializer_failure_fallback()
            .reselect_named_type_default(program, 0, COMMON_ACTOR_DYING_ACTIVE_STYLE,)
            .is_none());
    }

    #[test]
    fn explode_with_ring_descriptor_is_an_audited_alternate_only() {
        assert!(behavior_program(49).is_none());

        let program = audited_behavior_program(49).expect("class-49 alternate descriptor");
        assert_eq!(program, &EXPLODE_WITH_RING_BEHAVIOR_PROGRAM);
        assert_eq!(program.name, "Explode With Ring");
        assert_eq!(program.descriptor_address, 0x004C_8830);
        assert_eq!(program.style_table_base_address, 0x004C_71E0);
        assert_eq!(program.authored_enable_policy, 0);
        assert_eq!(program.authored_disable_policy, 0);
        assert_eq!(program.initializer_callback_address, 0x0040_BD20);
        assert_eq!(
            program.initializer_argument_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(audited_behavior_style(49, 0), Some(&program.initial_style));
        assert_eq!(program.initial_style.release_callback_address, None);
        assert_eq!(program.initial_style.pair_contact_callback_address, None);
        assert_eq!(program.initial_style.impact_callback_address, None);
        assert_eq!(program.initial_style.death_callback_address, None);
    }

    #[test]
    fn only_power_up_can_publish_initial_context_from_identity_alone() {
        for program in REACHABLE_BEHAVIOR_PROGRAMS {
            assert_eq!(
                initializer_success_is_identity_determined(program),
                program.class_id == 23,
                "{}",
                program.name
            );
        }

        let power_up =
            BehaviorContextRuntime::from_infallible_initial_selection(BehaviorSelection {
                choice_index: 0,
                program: behavior_program(23).expect("Power Up program"),
            })
            .expect("Power Up initializer is statically infallible");
        assert_eq!(
            power_up.choice_list_source(),
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
        );
        assert_eq!(
            power_up.target_handle_at_0x08(),
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            power_up.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
    }

    #[test]
    fn fresh_weighted_context_seeds_the_constructor_owned_words() {
        for class_id in [9, 10, 33] {
            let program = behavior_program(class_id).expect("type-17 weighted program");
            let context =
                BehaviorContextRuntime::from_fresh_weighted_selection(BehaviorSelection {
                    choice_index: 0,
                    program,
                })
                .expect("canonical weighted selection");

            assert_eq!(
                context.descriptor(),
                BehaviorDescriptorIdentity::Named(program)
            );
            assert_eq!(
                context.choice_list_source(),
                RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault)
            );
            assert_eq!(
                context.target_handle_at_0x08(),
                RetailRuntimeValue::Known(None)
            );
            assert_eq!(
                context.auxiliary_word_at_0x0c(),
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(context.style_table_index_raw_at_0x10(), 0);
            assert_eq!(
                context.active_style(),
                ActiveBehaviorStyle::Audited(program.initial_style)
            );
        }
    }

    #[test]
    fn live_context_keeps_raw_style_index_target_and_unnamed_fallback_distinct() {
        let capture_people = behavior_program(9).expect("Capture People program");
        let capture_style = *audited_behavior_style(9, 1).expect("captured variant 1");
        let context = BehaviorContextRuntime::named_audited(
            capture_people,
            1,
            RetailRuntimeValue::Known(BehaviorChoiceListSource::TypeDefault),
            RetailRuntimeValue::Known(Some(0x04AB_0001)),
            RetailRuntimeValue::Known(0),
            capture_style,
        )
        .expect("raw index 1 addresses the audited style");

        assert_eq!(context.descriptor_address(), 0x004C_88C0);
        assert_eq!(context.style_table_index_raw_at_0x10(), 1);
        assert_eq!(context.active_style().style_address(), 0x004C_8038);

        let fallback = context.with_initializer_failure_fallback();
        assert_eq!(
            fallback.descriptor(),
            BehaviorDescriptorIdentity::InitializerFailureFallback
        );
        assert_eq!(
            fallback.descriptor_address(),
            INITIALIZER_FAILURE_FALLBACK_DESCRIPTOR_ADDRESS
        );
        assert_eq!(fallback.style_table_index_raw_at_0x10(), 0);
        assert_eq!(
            fallback.active_style(),
            ActiveBehaviorStyle::InitializerFailureFallback
        );
        assert_eq!(
            fallback.target_handle_at_0x08(),
            RetailRuntimeValue::Known(Some(0x04AB_0001))
        );
        assert_eq!(
            fallback.auxiliary_word_at_0x0c(),
            RetailRuntimeValue::Known(0)
        );
        assert_ne!(
            fallback.descriptor(),
            BehaviorDescriptorIdentity::Named(behavior_program(0).expect("authored None program"))
        );
    }

    #[test]
    fn fallback_policy_is_the_normal_clear_mask_and_wrapping_style_indices_are_rejected() {
        assert_eq!(
            translate_state_policy(INITIALIZER_FAILURE_FALLBACK_ENABLE_POLICY),
            TranslatedStatePolicy {
                set_bits: 0,
                clear_bits: 0x0006_8000,
            }
        );
        assert_eq!(INITIALIZER_FAILURE_FALLBACK_DISABLE_POLICY, 0);

        let program = behavior_program(9).expect("Capture People program");
        let style = *audited_behavior_style(9, 0).expect("initial style");
        assert!(BehaviorContextRuntime::named_audited(
            program,
            u32::MAX,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            style,
        )
        .is_none());

        let forged = Box::leak(Box::new(BehaviorProgram {
            descriptor_address: 0xDEAD_BEEF,
            ..*program
        }));
        assert!(BehaviorContextRuntime::named_audited(
            forged,
            0,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            RetailRuntimeValue::Unresolved,
            style,
        )
        .is_none());
        assert!(
            BehaviorContextRuntime::from_published_weighted_selection(BehaviorSelection {
                choice_index: 0,
                program: forged,
            })
            .is_none()
        );
    }

    #[test]
    fn state_policy_translation_matches_fun_0040d440() {
        assert_eq!(
            translate_state_policy(0x02_9981),
            TranslatedStatePolicy {
                set_bits: 0x0808_0000,
                clear_bits: 0x0005_8800,
            }
        );
        assert_eq!(
            translate_state_policy(0x001200),
            TranslatedStatePolicy {
                set_bits: 0,
                clear_bits: 0x0006_0000,
            }
        );
    }

    #[test]
    fn disable_policy_reverses_the_translated_set_and_clear_orientation() {
        let wander = behavior_program(6).unwrap();
        assert_eq!(
            apply_initial_behavior_state(0x0808_0000, wander),
            0x000C_0000
        );
    }

    #[test]
    fn player_styles_and_priority_programs_match_executable_addresses() {
        let player = behavior_program(24).unwrap();
        assert_eq!(player.initial_style.frame_address, 0x004CD940);
        assert_eq!(player.authored_enable_policy, 0x00010010);
        assert_eq!(player.initial_style.release_callback_address, None);
        assert_eq!(player.initial_style.impact_callback_address, None);
        assert_eq!(player.initial_style.death_callback_address, None);
        assert_eq!(
            player.initial_style.pair_contact_callback_address,
            Some(0x00447D70)
        );
        assert_eq!(
            behavior_program(41).unwrap().initial_style.frame_address,
            0x004C9480
        );
        assert_eq!(
            behavior_program(39).unwrap().initial_style.frame_address,
            0x004C9558
        );
        assert_eq!(
            behavior_program(46).unwrap().initial_style.frame_address,
            0x004C94C8
        );
    }

    #[test]
    fn initial_release_pair_impact_and_death_slots_match_the_complete_executable_census() {
        let release_callbacks = REACHABLE_BEHAVIOR_PROGRAMS
            .iter()
            .filter_map(|program| {
                program
                    .initial_style
                    .release_callback_address
                    .map(|address| (program.class_id, address))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            release_callbacks,
            [0, 29, 42, 59, 60, 68].map(|class_id| (class_id, 0x0040_D1C0))
        );

        let pair_callbacks = REACHABLE_BEHAVIOR_PROGRAMS
            .iter()
            .filter_map(|program| {
                program
                    .initial_style
                    .pair_contact_callback_address
                    .map(|address| (program.class_id, address))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            pair_callbacks,
            [
                (22, 0x0040_CEF0),
                (23, 0x0042_5AF0),
                (24, 0x0044_7D70),
                (28, 0x0040_CEF0),
                (34, 0x0040_CEF0),
                (35, 0x0040_CEF0),
                (39, 0x0042_5850),
                (41, 0x0042_58A0),
                (46, 0x0042_59F0),
                (50, 0x0040_C800),
                (59, 0x0040_CAF0),
                (66, 0x0044_7D70),
            ]
        );

        let impact_callbacks = REACHABLE_BEHAVIOR_PROGRAMS
            .iter()
            .filter_map(|program| {
                program
                    .initial_style
                    .impact_callback_address
                    .map(|address| (program.class_id, address))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            impact_callbacks,
            [
                (4, 0x0040_C690),
                (5, 0x0040_C690),
                (6, 0x0040_C690),
                (7, 0x0040_C690),
                (9, 0x0040_C690),
                (13, 0x0040_C690),
                (15, 0x0040_C690),
                (21, 0x0040_C690),
                (26, 0x0040_C690),
                (32, 0x0040_C690),
                (33, 0x0040_C690),
                (54, 0x0040_C690),
                (58, 0x0040_C690),
                (67, 0x0042_1990),
                (69, 0x0040_C690),
            ]
        );

        let death_callbacks = REACHABLE_BEHAVIOR_PROGRAMS
            .iter()
            .filter_map(|program| {
                program
                    .initial_style
                    .death_callback_address
                    .map(|address| (program.class_id, address))
            })
            .collect::<Vec<_>>();
        assert_eq!(death_callbacks, [(39, 0x0041_9750), (41, 0x0041_9750)]);
    }

    #[test]
    fn current_style_policy_distinguishes_both_recovered_release_callbacks() {
        assert_eq!(
            audited_behavior_style(0, 0)
                .unwrap()
                .release_callback_policy(),
            ReleaseCallbackPolicy::TerrainAlignAndReselect
        );
        assert_eq!(
            audited_behavior_style(6, 1)
                .unwrap()
                .release_callback_policy(),
            ReleaseCallbackPolicy::EnablePairAndReselect
        );
        assert_eq!(
            audited_behavior_style(9, 1)
                .unwrap()
                .release_callback_policy(),
            ReleaseCallbackPolicy::None
        );
        assert!(audited_behavior_style(6, 2).is_none());
    }

    #[test]
    fn current_style_death_policy_distinguishes_progression_and_cleanup() {
        assert_eq!(
            audited_behavior_style(41, 0)
                .unwrap()
                .death_callback_policy(),
            DeathCallbackPolicy::BaseFactoryProgression
        );
        assert_eq!(
            audited_behavior_style(9, 2)
                .unwrap()
                .death_callback_policy(),
            DeathCallbackPolicy::CapturePeopleCleanup
        );
        assert_eq!(
            audited_behavior_style(9, 1)
                .unwrap()
                .death_callback_policy(),
            DeathCallbackPolicy::None
        );
        assert!(audited_behavior_style(9, 6).is_none());
    }

    #[test]
    fn ordinary_gameplay_capture_closes_follow_and_attention_variant_one_styles() {
        for (class_id, frame_address) in [(33_u8, 0x004C_7B70), (45_u8, 0x004C_86F8)] {
            assert_eq!(
                audited_behavior_style(u32::from(class_id), 1),
                Some(&BehaviorStyle {
                    class_id,
                    variant: 1,
                    frame_address,
                    release_callback_address: None,
                    pair_contact_callback_address: None,
                    impact_callback_address: Some(0x0040_C690),
                    death_callback_address: None,
                })
            );
        }
    }

    #[test]
    fn impact_policy_preserves_null_reselection_cleanup_and_unknown_addresses() {
        assert_eq!(
            audited_behavior_style(6, 0)
                .unwrap()
                .impact_callback_policy(),
            ImpactCallbackPolicy::ReselectBehavior
        );
        assert_eq!(
            audited_behavior_style(10, 1)
                .unwrap()
                .impact_callback_policy(),
            ImpactCallbackPolicy::None
        );
        assert_eq!(
            audited_behavior_style(9, 2)
                .unwrap()
                .impact_callback_policy(),
            ImpactCallbackPolicy::CapturePeopleCleanup
        );
        assert_eq!(
            audited_behavior_style(67, 0)
                .unwrap()
                .impact_callback_policy(),
            ImpactCallbackPolicy::UnknownAddress(0x0042_1990)
        );
    }

    #[test]
    fn pair_contact_policy_preserves_known_and_unknown_addresses() {
        let style = |address| BehaviorStyle {
            class_id: 0,
            variant: 0,
            frame_address: 0,
            release_callback_address: None,
            pair_contact_callback_address: address,
            impact_callback_address: None,
            death_callback_address: None,
        };

        assert_eq!(
            style(None).pair_contact_callback_policy(),
            PairContactCallbackPolicy::None
        );
        assert_eq!(
            style(Some(0x0040_C910)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::CapturePeople
        );
        assert_eq!(
            style(Some(0x0042_5850)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::LifterDelivery
        );
        assert_eq!(
            style(Some(0x0042_58A0)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::MainBaseConversion
        );
        assert_eq!(
            style(Some(0x0042_59F0)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::Hive
        );
        assert_eq!(
            style(Some(0x0042_5AF0)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::PowerUp
        );
        assert_eq!(
            style(Some(0x0044_7D70)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::PlayerContact
        );
        assert_eq!(
            style(Some(0x1234_5678)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::UnknownAddress(0x1234_5678)
        );
        assert_eq!(
            style(Some(0x0040_2DA0)).pair_contact_callback_policy(),
            PairContactCallbackPolicy::UnknownAddress(0x0040_2DA0),
            "the component callback must not acquire behavior return semantics"
        );
    }

    #[test]
    fn none_behavior_clears_terrain_contact_but_preserves_pair_enable() {
        let state = apply_initial_behavior_state(0x0001_8000, behavior_program(0).unwrap());
        assert_eq!(state, 0x0000_8000);
    }

    #[test]
    fn one_always_candidate_still_consumes_exactly_one_random_word() {
        let mut draws = 0;
        let selected = select_initial_behavior(
            &[choice(1, 1, 41)],
            |rule| i32::from(rule == BehaviorWeightRule::Always),
            || {
                draws += 1;
                0xffff
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(draws, 1);
        assert_eq!(selected.program.class_id, 41);
    }

    #[test]
    fn rng_independent_resolution_does_not_claim_weighted_world_state() {
        assert_eq!(
            resolve_rng_independent_initial_behavior(&[]).unwrap(),
            RngIndependentInitialBehavior::Known(None)
        );
        assert_eq!(
            resolve_rng_independent_initial_behavior(&[choice(1, 1, 41)]).unwrap(),
            RngIndependentInitialBehavior::Known(Some(BehaviorSelection {
                choice_index: 0,
                program: behavior_program(41).unwrap(),
            }))
        );
        assert_eq!(
            resolve_rng_independent_initial_behavior(&[choice(1, 0, 41)]).unwrap(),
            RngIndependentInitialBehavior::Known(None)
        );
        assert_eq!(
            resolve_rng_independent_initial_behavior(&[choice(6, 1, 41)]).unwrap(),
            RngIndependentInitialBehavior::RuntimeDependent
        );
        assert_eq!(
            resolve_rng_independent_initial_behavior(&[choice(1, 1, 39), choice(1, 1, 41),])
                .unwrap(),
            RngIndependentInitialBehavior::RuntimeDependent
        );
    }

    #[test]
    fn selection_uses_strict_cumulative_threshold_and_upper_total_cap() {
        let choices = [choice(1, 20_000, 39), choice(1, 20_000, 41)];
        let first = select_initial_behavior(&choices, |_| 1, || 0)
            .unwrap()
            .unwrap();
        assert_eq!(first.choice_index, 0);

        // Capped total 32767 and sample 40002 produce threshold 20000, so the
        // strict comparison skips cumulative 20000 and chooses candidate 1.
        let second = select_initial_behavior(&choices, |_| 1, || 40_002)
            .unwrap()
            .unwrap();
        assert_eq!(second.choice_index, 1);
    }

    #[test]
    fn conditional_turret_and_unconditional_sea_level_edits_stay_distinct() {
        assert_eq!(
            apply_initial_behavior_state(0, behavior_program(29).unwrap()),
            0
        );
        assert_eq!(
            apply_component_setup_success_state(0, behavior_program(29).unwrap()),
            0x28
        );
        assert_eq!(
            apply_initial_behavior_state(0xffff, behavior_program(38).unwrap()),
            0xf7ff
        );
    }

    #[test]
    fn collapsed_behavior_policy_preserves_unrelated_unknown_domains() {
        let policy = initial_behavior_state_policy(behavior_program(24).unwrap());
        assert_eq!(policy.set_bits & policy.clear_bits, 0);
        for state in [0, 0x0060_0000, 0xdead_beef, u32::MAX] {
            assert_eq!(
                (state | policy.set_bits) & !policy.clear_bits,
                apply_initial_behavior_state(state, behavior_program(24).unwrap())
            );
        }
    }
}
