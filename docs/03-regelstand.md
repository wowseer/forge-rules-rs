# Regelstand

Automatisch erzeugt von `python3 rust/tools/sync_regelstatus.py` aus `rust/tools/regelstatus.csv`.
Nicht von Hand bearbeiten – Status in der CSV pflegen.

- **ja**: umgesetzt und durch Regeltests (Comprehensive Rules) abgesichert → in den xlsx-Dateien `TRUE`.
- **teilweise**: Grundform umgesetzt, Varianten fehlen → `FALSE`.
- **n/a**: gehört zu einem ausgeschlossenen Sonderformat → `n/a`, zählt nicht.
- Der Abgleich mit der Java-Engine (Differential-Tests) steht noch aus, siehe `04-arbeitsprotokoll.md`.

## Übersicht

| Kategorie | Regeln (ohne n/a) | ja | teilweise | n/a |
|---|---:|---:|---:|---:|
| Keyword | 198 | 14 | 6 | 4 |
| Trigger | 137 | 0 | 15 | 11 |
| Replacement Effect | 35 | 0 | 2 | 5 |
| Static Ability | 90 | 1 | 4 | 0 |
| Effect (ApiType) | 188 | 2 | 31 | 16 |
| Cost | 42 | 5 | 4 | 1 |
| Turn Structure | 16 | 10 | 6 | 0 |
| State-Based Action | 23 | 14 | 1 | 3 |
| Combat Rule | 26 | 7 | 5 | 0 |
| Mana Rule | 14 | 8 | 2 | 1 |
| Zone | 12 | 5 | 2 | 8 |
| Mulligan | 2 | 1 | 0 | 4 |
| Game Variant | 3 | 0 | 1 | 24 |
| Game Rule Option | 3 | 0 | 1 | 4 |
| **Summe** | **789** | **67** | **80** | **81** |

## Keyword

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Cycling | 🟡 teilweise | `keywords_misc::cycling_from_hand_discards_and_draws, keywords_misc::cycling_not_offered_from_battlefield_or_without_mana` | Aktivierung aus der Hand, Abwerfen als Kosten | keyword/Keyword.java (KeywordWithCost) |
| Deathtouch | ✅ ja | `combat::deathtouch_lethal_is_one_for_damage_assignment` |  | keyword/Keyword.java (SimpleKeyword) |
| Defender | ✅ ja | `combat::summoning_sickness_haste_defender` |  | keyword/Keyword.java (SimpleKeyword) |
| Devoid | ✅ ja | `keywords_misc::devoid_is_colorless_in_all_zones` |  | keyword/Keyword.java (SimpleKeyword) |
| Double agenda | ⚪ n/a |  | Conspiracy-Draft | keyword/Keyword.java (SimpleKeyword) |
| Double Strike | ✅ ja | `combat::double_strike_deals_damage_twice` |  | keyword/Keyword.java (SimpleKeyword) |
| Enchant | 🟡 teilweise | `auras_equipment::aura_attaches_on_resolution_and_buffs` | Aura-Zauber mit Ziel, angelegt beim Auflösen, Fizzle | keyword/Keyword.java (KeywordWithType) |
| Equip | 🟡 teilweise | `auras_equipment::equip_sorcery_speed_own_creature_and_moves` | Kosten, optionaler Filter, Hexerei-Geschwindigkeit | keyword/Keyword.java (Equip) |
| First Strike | ✅ ja | `combat::first_strike_kills_before_regular_damage` |  | keyword/Keyword.java (SimpleKeyword) |
| Flash | ✅ ja | `spells::flash_allows_instant_speed_casting` |  | keyword/Keyword.java (SimpleKeyword) |
| Flying | ✅ ja | `combat::flying_needs_flying_or_reach_to_block` |  | keyword/Keyword.java (SimpleKeyword) |
| Haste | ✅ ja | `combat::summoning_sickness_haste_defender` |  | keyword/Keyword.java (SimpleKeyword) |
| Hexproof | 🟡 teilweise | `spells::hexproof_only_blocks_opponents` | „Hexproof from …“ fehlt | keyword/Keyword.java (Hexproof) |
| Hidden agenda | ⚪ n/a |  | Conspiracy-Draft | keyword/Keyword.java (SimpleKeyword) |
| Indestructible | ✅ ja | `spells::destroy_respects_indestructible` |  | keyword/Keyword.java (SimpleKeyword) |
| Lifelink | ✅ ja | `combat::deathtouch_and_lifelink_in_combat` |  | keyword/Keyword.java (SimpleKeyword) |
| Menace | ✅ ja | `combat::menace_needs_two_blockers` |  | keyword/Keyword.java (SimpleKeyword) |
| Reach | ✅ ja | `combat::flying_needs_flying_or_reach_to_block` |  | keyword/Keyword.java (SimpleKeyword) |
| Shroud | ✅ ja | `spells::shroud_blocks_all_targeting` |  | keyword/Keyword.java (SimpleKeyword) |
| Space sculptor | ⚪ n/a |  | Unstable | keyword/Keyword.java (SimpleKeyword) |
| Starting intensity | ⚪ n/a |  | Alchemy | keyword/Keyword.java (KeywordWithAmount) |
| Trample | 🟡 teilweise | `combat::trample_assigns_excess_to_player` | Trample über Planeswalker fehlt | keyword/Keyword.java (Trample) |
| TypeCycling | 🟡 teilweise | `keywords_misc::typecycling_searches_library_for_type` | Bibliothekssuche nach Typ | keyword/Keyword.java (KeywordWithCostAndType) |
| Vigilance | ✅ ja | `combat::vigilance_does_not_tap` |  | keyword/Keyword.java (SimpleKeyword) |

Offen (178): Absorb, Affinity, Afflict, Afterlife, Aftermath, Amplify, Annihilator, Ascend, Assist, Aura swap, Awaken, Backup, Banding, Bands with other, Bargain, Battle cry, Beam me up, Bestow, Blitz, Bloodthirst, Bushido, Buyback, Cascade, Casualty, Champion, Changeling, Choose a Background, Cipher, Companion, Compleated, Conspire, Convoke, Craft, Crew, Cumulative upkeep, Dash, Daybound, Decayed, Delve, Demonstrate, Dethrone, Devour, Disguise, Disturb, Doctor's companion, Double team, Dredge, Echo, Embalm, Emerge, Encore, Enlist, Entwine, Epic, Escape, Escalate, Eternalize, Evoke, Evolve, Exalted, Exploit, Extort, Fabricate, Fading, Fear, Firebending, Flanking, Flashback, For Mirrodin, Foretell, Fortify, Freerunning, Frenzy, Fuse, Gift, Graft, Gravestorm, Harmonize, Haunt, Hideaway, Horsemanship, Impending, Improvise, Increment, Infect, Ingest, Intimidate, Kicker, Job select, Jump-start, Landwalk, Level up, Living metal, Living Weapon, Madness, Mayhem, Melee, Mentor, Megamorph, Miracle, Mobilize, Modular, More Than Meets the Eye, Morph, Multikicker, Mutate, Myriad, Nightbound, Ninjutsu, Outlast, Offering, Offspring, Overload, Paradigm, Partner, Partner with, Persist, Phasing, Plot, Poisonous, Protection, Prototype, Provoke, Prowess, Prowl, Rampage, Ravenous, Read ahead, Rebound, Recover, Reconfigure, Reflect, Reinforce, Renown, Replicate, Retrace, Riot, Ripple, Saddle, Scavenge, Shadow, Skulk, Sneak, Soulbond, Soulshift, Specialize, Spectacle, Splice, Split second, Spree, Squad, Start your engines, Station, Storied, Storm, Strive, Sunburst, Surge, Suspend, Teamwork, Tiered, Toxic, Training, Transfigure, Transmute, Tribute, Umbra armor, Undaunted, Undying, Unearth, Unleash, Vanishing, Ward, Warp, Web-slinging, Wither, MayFlashCost, MayFlashSac

## Trigger

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Abandoned | ⚪ n/a |  | Archenemy | trigger/TriggerAbandoned.java |
| AttackerBlocked | 🟡 teilweise | `triggers::blocks_attacker_blocked_and_unblocked_triggers` | ValidCard, ValidBlocker/Amount | trigger/TriggerAttackerBlocked.java |
| AttackersDeclared | 🟡 teilweise | `triggers::attackers_declared_once_with_minimum_amount` | AttackingPlayer, AttackedTarget (nur Spieler), ValidAttackers/Amount, TriggeredAttackingPlayer | trigger/TriggerAttackersDeclared.java |
| AttackersDeclaredOneTarget | 🟡 teilweise | `triggers::attackers_declared_one_target_per_attacked_player` | je angegriffenem Spieler, TriggeredAttackedTarget | trigger/TriggerAttackersDeclared.java |
| AttackerUnblocked | 🟡 teilweise | `triggers::blocks_attacker_blocked_and_unblocked_triggers` | ValidCard, ValidDefender | trigger/TriggerAttackerUnblocked.java |
| Attacks | 🟡 teilweise | `triggers::attack_trigger_with_triggered_player` | Bedingungen (Alone, Attacked …) fehlen | trigger/TriggerAttacks.java |
| Blocks | 🟡 teilweise | `triggers::blocks_attacker_blocked_and_unblocked_triggers` | ValidCard, ValidBlocked | trigger/TriggerBlocks.java |
| ChangesZone | 🟡 teilweise | `triggers::dies_trigger_looks_back_in_time` | Grundform inkl. Rückblick (603.10a) und gleichzeitiger Tode | trigger/TriggerChangesZone.java |
| ChaosEnsues | ⚪ n/a |  | Planechase | trigger/TriggerChaosEnsues.java |
| ConjureAll | ⚪ n/a |  | Alchemy | trigger/TriggerConjureAll.java |
| CrankContraption | ⚪ n/a |  | Unstable | trigger/TriggerCrankContraption.java |
| DamageDone | 🟡 teilweise | `triggers::combat_damage_trigger` | ValidSource/ValidTarget/CombatDamage | trigger/TriggerDamageDone.java |
| DamageDoneOnce | 🟡 teilweise | `triggers::damage_done_once_triggers_once_per_recipient` | einmal je Empfänger bei gleichzeitigem Schaden (Kampf, DealDamage, DamageAll) | trigger/TriggerDamageDoneOnce.java |
| Drawn | 🟡 teilweise | `triggers::drawn_number_life_gained_and_taps_triggers` | ValidCard, ValidPlayer, Number | trigger/TriggerDrawn.java |
| LifeGained | 🟡 teilweise | `triggers::drawn_number_life_gained_and_taps_triggers` | ValidPlayer | trigger/TriggerLifeGained.java |
| Phase | 🟡 teilweise | `triggers::upkeep_trigger_with_sub_ability` | Phase$ als Liste einzelner Schritte | trigger/TriggerPhase.java |
| PlanarDice | ⚪ n/a |  | Planechase | trigger/TriggerPlanarDice.java |
| PlaneswalkedFrom | ⚪ n/a |  | Planechase | trigger/TriggerPlaneswalkedFrom.java |
| PlaneswalkedTo | ⚪ n/a |  | Planechase | trigger/TriggerPlaneswalkedTo.java |
| Sacrificed | 🟡 teilweise | `sacrifice_unless::sacrificed_trigger_only_for_own_sacrifices_not_destroy, sacrifice_unless::self_sacrifice_as_cost_triggers_from_lki` | ValidCard (LKI), ValidPlayer, auch bei Opferkosten | trigger/TriggerSacrificed.java |
| SeekAll | ⚪ n/a |  | Alchemy | trigger/TriggerSeekAll.java |
| SetInMotion | ⚪ n/a |  | Archenemy | trigger/TriggerSetInMotion.java |
| SpellCast | 🟡 teilweise | `triggers::spell_cast_trigger_resolves_before_spell` | ValidCard/ValidActivatingPlayer | trigger/TriggerSpellAbilityCastOrCopy.java |
| StickerPlaced | ⚪ n/a |  | Unfinity | trigger/TriggerStickerPlaced.java |
| Taps | 🟡 teilweise | `triggers::drawn_number_life_gained_and_taps_triggers` | ValidCard, ValidPlayer | trigger/TriggerTaps.java |
| VisitAttraction | ⚪ n/a |  | Unfinity | trigger/TriggerVisitAttraction.java |

Offen (122): AbilityCast, AbilityResolves, AbilityTriggered, Adapt, Airbend, Always, Attached, AttackerBlockedOnce, AttackerBlockedByCreature, AttackerUnblockedOnce, BecomeMonarch, BecomeMonstrous, BecomeRenowned, BecomesCrewed, BecomesPlotted, BecomesSaddled, BecomesTarget, BecomesTargetOnce, BlockersDeclared, CaseSolved, Championed, ChangesController, ChangesZoneAll, ClaimPrize, Clashed, ClassLevelGained, CommitCrime, Connives, CollectEvidence, CounterAdded, CounterAddedOnce, CounterPlayerAddedAll, CounterTypeAddedAll, CounterAddedAll, Countered, CounterRemoved, CounterRemovedOnce, Crewed, Cycled, DamageAll, DamageDealtOnce, DamageDoneOnceByController, DamagePreventedOnce, Destroyed, Devoured, Discarded, DiscardedAll, Discover, DungeonCompleted, Earthbend, Evolved, ExcessDamage, ExcessDamageAll, ElementalBend, Enlisted, Exerted, Exiled, Exploited, Explores, FacesDilemma, Fight, FightOnce, Firebend, FlippedCoin, FlippedCoinOnce, Forage, Foretell, FullyUnlock, GiveGift, Immediate, Investigated, LandPlayed, LifeLost, LifeLostAll, LosesGame, ManaAdded, ManaExpend, ManifestDread, Mentored, Milled, MilledOnce, MilledAll, Mutates, NewGame, PayCumulativeUpkeep, PayEcho, PayLife, PhaseIn, PhaseOut, PhaseOutAll, Proliferate, RingTemptsYou, RolledDie, RolledDieOnce, RoomEntered, Saddled, SacrificedOnce, Scry, SearchedLibrary, Shuffled, Specializes, SpellAbilityCast, SpellAbilityCopy, SpellCastOrCopy, SpellCopy, Stationed, Surveil, TakesInitiative, TapAll, TapsForMana, TokenCreated, TokenCreatedOnce, Trains, Transformed, TurnBegin, TurnFaceUp, Unattached, UnlockDoor, UntapAll, Untaps, Vote, Waterbend

## Replacement Effect

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| AssembleContraption | ⚪ n/a |  | Unstable | replacement/ReplaceAssembleContraption.java |
| DamageDone | 🟡 teilweise | `replacements::damage_doubling_replacement, replacements::prevent_combat_damage_to_self_but_not_noncombat, replacements::prevention_shield_next_n_damage_this_turn_after_doubling` | Prevent, Mengenänderung (ReplaceCount$DamageAmount), ValidTarget/ValidSource/IsCombat | replacement/ReplaceDamage.java |
| Moved | 🟡 teilweise | `replacements::rest_in_peace_redirects_to_exile_and_no_dies_trigger` | Umleitung (ChangeZone ReplacedCard), getappt ins Spiel inkl. Bedingung, etbCounter | replacement/ReplaceMoved.java |
| PlanarDiceResult | ⚪ n/a |  | Planechase | replacement/ReplacePlanarDiceResult.java |
| Planeswalk | ⚪ n/a |  | Planechase | replacement/ReplacePlaneswalk.java |
| RollPlanarDice | ⚪ n/a |  | Planechase | replacement/ReplaceRollPlanarDice.java |
| SetInMotion | ⚪ n/a |  | Archenemy | replacement/ReplaceSetInMotion.java |

Offen (33): AddCounter, AssignDealDamage, Attached, BeginPhase, BeginTurn, Cascade, Connive, Counter, CopySpell, CreateToken, DealtDamage, DeclareBlocker, Destroy, Draw, DrawCards, Explore, GainLife, GameLoss, GameWin, Learn, LifeReduced, LoseMana, Mill, PayLife, ProduceMana, Proliferate, RemoveCounter, RollDice, Scry, Tap, Transform, TurnFaceUp, Untap

## Static Ability

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Continuous | 🟡 teilweise | `layers::affected_set_determined_after_type_change` | Affected/AffectedDefined Self, Zonen (inkl. All), AddType, SetColor, AddKeyword, Set/AddPower/Toughness inkl. Count$-Ausdrücke, CDAs (7a), Bedingungen (IsPresent, CheckSVar, Condition) | staticability/StaticAbility.java |
| CantAttack | 🟡 teilweise | `combat_restrictions::cant_attack` | ValidCard, Bedingungen | staticability/StaticAbilityCantAttackBlock.java |
| CantBlock | ✅ ja | `combat_restrictions::cant_block` |  | staticability/StaticAbilityCantAttackBlock.java |
| CantBlockBy | 🟡 teilweise | `combat_restrictions::cant_be_blocked_by_small_creatures_via_effect` | ValidAttacker, ValidBlocker, Bedingungen | staticability/StaticAbilityCantAttackBlock.java |
| MustAttack | 🟡 teilweise | `combat_restrictions::must_attack_if_able` | ValidCreature (CR 508.1d) | staticability/StaticAbilityMustAttack.java |

Offen (85): CantAttackUnless, CantBlockUnless, OptionalAttackCost, OptionalCost, AlternativeCost, CantBeCast, CantBeActivated, CantPlayLand, DisableTriggers, Panharmonicon, MustTarget, CanAttackDefender, CanAttackIfHaste, CanBlockIfReach, CanBlockIfShadow, MinMaxBlocker, BlockTapped, AttackVigilance, PlayerMustAttack, MustBlock, AssignCombatDamageAsUnblocked, CombatDamageToughness, CombatDamageNegatePower, ColorlessDamageSource, NoCleanupDamage, BlockRestrict, CantGainLife, CantLoseLife, CantChangeLife, CantPayLife, RaiseCost, ReduceCost, SetCost, IgnoreHexproof, IgnoreShroud, AttackRestrict, AssignNoCombatDamage, CanAdapt, CantBeCopied, CantBeBeamedUp, CantBeSuspected, CantBecomeMonarch, CantGainControl, CantAttach, CantCrew, CantDraw, DrawFromBottom, CantDiscard, CantExile, CantPhaseIn, CantPhaseOut, CantPreventDamage, CantPutCounter, CantRegenerate, CantSacrifice, CantTarget, CantTransform, CantVenture, CantChangeDayTime, ActivateAbilityAsIfHaste, CastWithFlash, IgnoreLandwalk, IgnoreLegendRule, IgnorePlaneswalkerZeroLoyaltyRule, MaxCounter, InfectDamage, WitherDamage, FlipCoinMod, FlipCoinDoubler, PlotZone, NumLoyaltyAct, Activations, Devotion, GainLifeRadiation, SurveilNum, TapPowerValue, UnspentMana, ManaBurn, ManaConvert, UntapOtherPlayer, TurnReversed, PhaseReversed, AttackRequirement, CountersRemain, ManaRestriction

## Effect (ApiType)

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Abandon | ⚪ n/a |  | Archenemy | ability/effects/AbandonEffect.java |
| AdvanceCrank | ⚪ n/a |  | Unstable | ability/effects/AdvanceCrankEffect.java |
| Animate | 🟡 teilweise | `library_regen_animate::animate_land_until_end_of_turn` | Power/Toughness, Types, RemoveCreatureTypes, Keywords, Colors/OverwriteColors, Duration Permanent | ability/effects/AnimateEffect.java |
| Attach | 🟡 teilweise | `auras_equipment::equip_sorcery_speed_own_creature_and_moves` | Object (Standard Self) an Ziel anlegen | ability/effects/AttachEffect.java |
| AssembleContraption | ⚪ n/a |  | Unstable | ability/effects/AssembleContraptionEffect.java |
| ChangeZone | 🟡 teilweise | `change_zone::search_library_tapped_then_shuffle` | Ziele in beliebigen Zonen, Suche (verdeckt: darf nicht finden), Tapped, GainControl, LibraryPosition 0/-1 | ability/effects/ChangeZoneEffect.java |
| ChangeZoneAll | 🟡 teilweise | `change_zone::change_zone_all_bounces_everything` | ChangeType + Origin/Destination, gleichzeitig (Rückblick für Trigger) | ability/effects/ChangeZoneAllEffect.java |
| ChaosEnsues | ⚪ n/a |  | Planechase | ability/effects/ChaosEnsuesEffect.java |
| Charm | 🟡 teilweise | `tokens_charm::charm_choose_two_resolve_in_printed_order` | CharmNum/MinCharmNum, CanRepeatModes, nur Modi mit legalen Zielen, Druckreihenfolge, bei Triggern beim Stapeln | ability/effects/CharmEffect.java |
| ChooseSector | ⚪ n/a |  | Unstable | ability/effects/ChooseSectorEffect.java |
| Cleanup | 🟡 teilweise | `sacrifice_unless::condition_defined_remembered_and_cleanup` | ClearRemembered (übrige Clear…-Flags ohne Wirkung, da der Zustand nicht existiert) | ability/effects/CleanUpEffect.java |
| Counter | 🟡 teilweise | `mass_discard_counter::mana_leak_pay_or_countered` | Zauber neutralisieren, allgemeines UnlessCost | ability/effects/CounterEffect.java |
| DamageAll | 🟡 teilweise | `mass_discard_counter::damage_all_creatures_and_players` | ValidCards/ValidPlayers | ability/effects/DamageAllEffect.java |
| DealDamage | 🟡 teilweise | `spells::bolt_face_and_creature_and_fizzle` | Grundform | ability/effects/DamageDealEffect.java |
| DelayedTrigger | 🟡 teilweise | `delayed_triggers::flicker_returns_at_next_end_step_following_the_card, delayed_triggers::delayed_trigger_fires_once_at_your_next_upkeep, delayed_triggers::this_turn_delayed_trigger_expires_at_cleanup` | alle Trigger-Modi, einmaliges Auslösen (CR 603.7c), ThisTurn, NextTurn, RememberObjects + DelayTriggerRemembered[LKI] | ability/effects/DelayedTriggerEffect.java |
| Destroy | 🟡 teilweise | `spells::destroy_respects_indestructible` | Grundform | ability/effects/DestroyEffect.java |
| DestroyAll | 🟡 teilweise | `mass_discard_counter::wrath_destroys_all_simultaneously_with_dies_triggers` | gleichzeitig mit Rückblick | ability/effects/DestroyAllEffect.java |
| Dig | 🟡 teilweise | `library_regen_animate::dig_one_to_hand_rest_bottom` | DigNum, ChangeNum (auch All), ChangeValid, Destination(2), LibraryPosition(2), Optional | ability/effects/DigEffect.java |
| Discard | 🟡 teilweise | `mass_discard_counter::duress_caster_chooses_noncreature_nonland` | TgtChoose, Hand, Random, RevealYouChoose, DiscardValid | ability/effects/DiscardEffect.java |
| Draft | ⚪ n/a |  | Conspiracy-Draft | ability/effects/DraftEffect.java |
| Draw | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/DrawEffect.java |
| Effect | 🟡 teilweise | `effects_objects::effect_grants_hexproof_to_remembered_until_end_of_turn` | Effektobjekt in Command Zone mit Statics/Triggern/Ersatzeffekten, RememberObjects (Targeted/Self/Triggered…), Duration (Zugende, Permanent, UntilYourNextTurn, UntilTheEndOfYourNextTurn) | ability/effects/EffectEffect.java |
| GainControl | 🟡 teilweise | `control::threaten_until_end_of_turn_untaps_and_grants_haste, control::permanent_control_change, control::control_ends_when_source_leaves_battlefield` | Kontrolleffekte nach Zeitstempel (Layer 2), LoseControl EOT/UntilTheEndOfYourNextTurn/LeavesPlay/LoseControl, Untap, AddKWs, NewController | ability/effects/ControlGainEffect.java |
| GainLife | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/LifeGainEffect.java |
| Heist | ⚪ n/a |  | Alchemy | ability/effects/HeistEffect.java |
| Intensify | ⚪ n/a |  | Alchemy | ability/effects/IntensifyEffect.java |
| LoseLife | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/LifeLoseEffect.java |
| LosePerpetual | ⚪ n/a |  | Alchemy | ability/effects/LosePerpetualEffect.java |
| Mana | 🟡 teilweise | `spells::manual_mana_payment_and_cancel` |  | ability/effects/ManaEffect.java |
| Mill | 🟡 teilweise | `library_regen_animate::mill_target_player` | NumCards, Defined/Ziel | ability/effects/MillEffect.java |
| OpenAttraction | ⚪ n/a |  | Unfinity | ability/effects/OpenAttractionEffect.java |
| Planeswalk | ⚪ n/a |  | Planechase | ability/effects/PlaneswalkEffect.java |
| PreventDamage | 🟡 teilweise | `replacements::prevention_shield_next_n_damage_this_turn_after_doubling` | nächste N Schaden in diesem Zug (Karten, Spieler) | ability/effects/DamagePreventEffect.java |
| Pump | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | NumAtt/NumDef/KW bis Zugende | ability/effects/PumpEffect.java |
| PumpAll | 🟡 teilweise | `mass_discard_counter::pump_all_own_creatures_until_end_of_turn` | ValidCards, NumAtt/NumDef, KW bis Zugende | ability/effects/PumpAllEffect.java |
| PutCounter | 🟡 teilweise | `sba::cr_704_5q_plus_and_minus_counters_annihilate` | Grundform | ability/effects/CountersPutEffect.java |
| PutSticker | ⚪ n/a |  | Unfinity | ability/effects/PutStickerEffect.java |
| Regenerate | ✅ ja | `library_regen_animate::regenerate_replaces_destruction` | inkl. SBA-Zerstörung, NoRegen bei Destroy/DestroyAll | ability/effects/RegenerateEffect.java |
| RepeatEach | 🟡 teilweise | `expressions::repeat_each_player_with_remembered_player, expressions::repeat_each_card_remembers_each_card_in_turn` | RepeatPlayers, RepeatCards (+Zone), DefinedCards, StartingWith You, ClearRememberedBeforeLoop, DamageMap | ability/effects/RepeatEachEffect.java |
| RollPlanarDice | ⚪ n/a |  | Planechase | ability/effects/RollPlanarDiceEffect.java |
| RunChaos | ⚪ n/a |  | Planechase | ability/effects/RunChaosEffect.java |
| Sacrifice | 🟡 teilweise | `sacrifice_unless::edict_target_player_chooses_and_indestructible_can_be_sacrificed, sacrifice_unless::each_player_sacrifices_simultaneously_dies_triggers_see_all, sacrifice_unless::optional_up_to_amount_and_remembered, sacrifice_unless::strict_amount_sacrifices_nothing_if_not_enough` | SacValid, Defined, Amount, Optional, StrictAmount, Destroy, RememberSacrificed, gleichzeitig (CR 101.4) | ability/effects/SacrificeEffect.java |
| Scry | ✅ ja | `library_regen_animate::scry_bottom_and_reorder_then_draw` |  | ability/effects/ScryEffect.java |
| Seek | ⚪ n/a |  | Alchemy | ability/effects/SeekEffect.java |
| SetInMotion | ⚪ n/a |  | Archenemy | ability/effects/SetInMotionEffect.java |
| Surveil | 🟡 teilweise | `library_regen_animate::surveil_puts_chosen_cards_into_graveyard` | Amount, Defined | ability/effects/SurveilEffect.java |
| Tap | 🟡 teilweise |  | Grundform | ability/effects/TapEffect.java |
| Token | 🟡 teilweise | `tokens_charm::tokens_enter_and_trigger_etb` | TokenScript (Liste), TokenAmount, TokenOwner, TokenTapped, TokenPower/Toughness | ability/effects/TokenEffect.java |
| Untap | 🟡 teilweise |  | Grundform | ability/effects/UntapEffect.java |

Offen (155): ActivateAbility, AddOrRemoveCounter, AddPhase, AddTurn, Airbend, AlterAttribute, Amass, AnimateAll, Assimilate, AssignGroup, Balance, BecomeMonarch, BecomesBlocked, BidLife, Blight, Block, Bond, Branch, Camouflage, ChangeCombatants, ChangeSpeed, ChangeTargets, ChangeText, ChangeX, ChooseCard, ChooseColor, ChooseDirection, ChooseEvenOdd, ChooseNumber, ChoosePlayer, ChooseSource, ChooseType, ClaimThePrize, Clash, ClassLevelUp, Cloak, Clone, Connive, CopyPermanent, CopySpellAbility, ControlSpell, ControlPlayer, Detain, DayTime, Debuff, DigMultiple, DigUntil, Discover, DrainMana, EachDamage, Earthbend, Empower, Encode, EndCombatPhase, EndTurn, Endure, ExchangeLife, ExchangeLifeVariant, ExchangeControl, ExchangeControlVariant, ExchangePower, ExchangeZone, ExchangeTextBox, Explore, Fight, FlipCoin, FlipOntoBattlefield, Fog, GainControlVariant, GainOwnership, GameDrawn, GenericChoice, Goad, Haunt, HealDamage, Investigate, ImmediateTrigger, Incubate, Learn, LookAt, LosesGame, MakeCard, ManaReflected, Manifest, ManifestDread, Meld, MoveCounter, MultiplePiles, MultiplyCounter, MustBlock, Mutate, NameCard, PeekAndReveal, PermanentCreature, PermanentNoncreature, Phases, Play, PlayLandVariant, Poison, Proliferate, Protection, ProtectionAll, PutCounterAll, Radiation, Recruit, RearrangeTopOfLibrary, Regeneration, RemoveCounter, RemoveCounterAll, RemoveFromCombat, RemoveFromGame, RemoveFromMatch, ReorderZone, Repeat, ReplaceCounter, ReplaceEffect, ReplaceMana, ReplaceDamage, ReplaceToken, ReplaceSplitDamage, RestartGame, Reveal, RevealHand, ReverseTurnOrder, RingTemptsYou, RollDice, SacrificeAll, SetLife, SetState, Shuffle, SkipPhase, SkipTurn, StoreSVar, Subgame, SwitchBlock, TakeInitiative, TapAll, TapOrUntap, TapOrUntapAll, TimeTravel, TwoPiles, Unattach, UnlockDoor, UntapAll, Venture, VillainousChoice, Vote, WinsGame, BlankLine, DamageResolve, ChangeZoneResolve, CompanionChoose, InternalLegendaryRule, InternalIgnoreEffect, InternalRadiation

## Cost

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Mana cost | ✅ ja | `spells::x_spell` | inkl. Hybrid, Phyrexian, {2/W}, X | cost/CostPartMana.java |
| Add mana | 🟡 teilweise | `spells::manual_mana_payment_and_cancel` | Mana-Fähigkeiten mit Produced$ (fest, Any, Combo Farbe) | cost/CostAddMana.java |
| {T} | ✅ ja | `spells::activated_ability_with_tap_cost_uses_stack_and_sickness` |  | cost/CostTap.java |
| {Q} | ✅ ja | `spells::untap_symbol_cost` |  | cost/CostUntap.java |
| Sac | ✅ ja | `spells::sacrifice_self_cost` | Sac<1/CARDNAME> und Sac<n/Typ> | cost/CostSacrifice.java |
| Discard | 🟡 teilweise |  | Discard<n/Typ> | cost/CostDiscard.java |
| PayLife | ✅ ja | `spells::phyrexian_mana_with_life` |  | cost/CostPayLife.java |
| PayShards | ⚪ n/a |  | Adventure-Modus | cost/CostPayShards.java |
| AddCounter | 🟡 teilweise | `planeswalkers::enters_with_loyalty_and_activates_once_per_turn_at_sorcery_speed` | Marken auf die Quelle (n fest) | cost/CostPutCounter.java |
| SubCounter | 🟡 teilweise | `planeswalkers::minus_ability_to_zero_loyalty_goes_to_graveyard` | Marken von der Quelle (n fest) | cost/CostRemoveCounter.java |

Offen (33): tapXType, untapXType, Draw, DamageYou, GainLife, PayEnergy, Exile, ExileFromStack, ExiledMoveToGrave, Mill, PutCardToLib, Return, Reveal, RevealChosen, AddCounterYou, RemoveAnyCounter, GainControl, Unattach, Exert, Enlist, FlipCoin, RollDice, ChooseColor, ChooseCreatureType, CollectEvidence, Forage, PromiseGift, Behold, BeholdExile, Blight, Teamwork, Waterbend, Cost adjustment

## Turn Structure

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Untap step | 🟡 teilweise | `turn_structure::untap_step_untaps_active_players_permanents` | Enttappen + Zugbeginn-Kontrolle fertig | phase/PhaseType.java (UNTAP) |
| Upkeep step | ✅ ja | `triggers::upkeep_trigger_with_sub_ability` | inkl. „zu Beginn des Versorgungssegments“-Trigger | phase/PhaseType.java (UPKEEP) |
| Draw step | ✅ ja | `turn_structure::draw_step_draws_and_first_player_skips_first_draw` | inkl. CR 103.8a | phase/PhaseType.java (DRAW) |
| Precombat main phase | 🟡 teilweise | `spells::land_play_once_per_turn_sorcery_timing` | Länder/Hexereien fertig | phase/PhaseType.java (MAIN1) |
| Beginning of combat step | ✅ ja | `turn_structure::steps_in_order_and_combat_skips_without_attackers` |  | phase/PhaseType.java (COMBAT_BEGIN) |
| Declare attackers step | 🟡 teilweise | `combat::summoning_sickness_haste_defender` | Spieler und Planeswalker als Ziel | phase/PhaseType.java (COMBAT_DECLARE_ATTACKERS) |
| Declare blockers step | 🟡 teilweise | `combat::flying_needs_flying_or_reach_to_block` | Flying/Reach/Menace | phase/PhaseType.java (COMBAT_DECLARE_BLOCKERS) |
| First strike damage step | ✅ ja | `combat::first_strike_kills_before_regular_damage` |  | phase/PhaseType.java (COMBAT_FIRST_STRIKE_DAMAGE) |
| Combat damage step | ✅ ja | `combat::trample_assigns_excess_to_player` | Regeln ab Foundations (keine Schadenszuweisungsreihenfolge) | phase/PhaseType.java (COMBAT_DAMAGE) |
| End of combat step | ✅ ja | `turn_structure::steps_in_order_and_combat_skips_without_attackers` |  | phase/PhaseType.java (COMBAT_END) |
| Postcombat main phase | 🟡 teilweise | `turn_structure::steps_in_order_and_combat_skips_without_attackers` | wie Precombat main phase | phase/PhaseType.java (MAIN2) |
| End step | ✅ ja | `turn_structure::cleanup_discards_to_hand_size_removes_damage_ends_effects` |  | phase/PhaseType.java (END_OF_TURN) |
| Cleanup step | ✅ ja | `turn_structure::cleanup_discards_to_hand_size_removes_damage_ends_effects` | inkl. CR 514.3a (erneuter Cleanup) | phase/PhaseType.java (CLEANUP) |
| Priority and stack | ✅ ja | `turn_structure::stack_resolves_last_in_first_out` | CR 117, 405, 608 inkl. Fizzle (608.2b) | phase/ / GameAction.java |
| Layer system | 🟡 teilweise | `layers::layer_7b_before_7c_regardless_of_timestamp` | Layer 4 (Typ hinzufügen), 5, 6, 7b, 7c nach Zeitstempel, betroffene Menge nach CR 613.6 | phase/ / GameAction.java |
| Mulligan | ✅ ja | `turn_structure::london_mulligan` | London-Mulligan | phase/ / GameAction.java |

## State-Based Action

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| CR 704.5a | ✅ ja | `sba::cr_704_5a_zero_life_loses` |  | GameAction.java (checkStateEffects) |
| CR 704.5b | ✅ ja | `sba::cr_704_5b_draw_from_empty_library_loses` |  | GameAction.java (checkStateEffects) |
| CR 704.5c | ✅ ja | `sba::cr_704_5c_ten_poison_loses` |  | GameAction.java (checkStateEffects) |
| CR 704.5d | ✅ ja | `sba::cr_704_5d_token_outside_battlefield_ceases_to_exist` | Spell-Kopien folgen mit Copy-Effekten | GameAction.java (checkStateEffects) |
| CR 704.5f | ✅ ja | `sba::cr_704_5f_zero_toughness_dies_even_if_indestructible` |  | GameAction.java (checkStateEffects) |
| CR 704.5g | ✅ ja | `sba::cr_704_5g_lethal_damage_destroys_but_not_indestructible` | Regeneration fehlt | GameAction.java (checkStateEffects) |
| CR 704.5h | ✅ ja | `sba::cr_704_5h_deathtouch_damage_destroys` |  | GameAction.java (checkStateEffects) |
| CR 704.5i | ✅ ja | `sba::cr_704_5i_planeswalker_without_loyalty` |  | GameAction.java (checkStateEffects) |
| CR 704.5j | ✅ ja | `sba::cr_704_5j_legend_rule_controller_chooses` |  | GameAction.java (checkStateEffects) |
| CR 704.5k | ✅ ja | `sba::cr_704_5k_world_rule_newest_stays` |  | GameAction.java (checkStateEffects) |
| CR 704.5m | ✅ ja | `auras_equipment::aura_goes_to_graveyard_when_creature_dies_and_trigger_returns_it` |  | GameAction.java (checkStateEffects) |
| CR 704.5n | ✅ ja | `auras_equipment::equipment_stays_when_creature_dies` |  | GameAction.java (checkStateEffects) |
| CR 704.5p | 🟡 teilweise |  | Lösen angelegter Nicht-Aura/Nicht-Ausrüstung | GameAction.java (checkStateEffects) |
| CR 704.5q | ✅ ja | `sba::cr_704_5q_plus_and_minus_counters_annihilate` |  | GameAction.java (checkStateEffects) |
| CR 704.5u | ⚪ n/a |  | Space Sculptor (Unstable) | GameAction.java (checkStateEffects) |
| CR 704.5v | ✅ ja | `sba::cr_704_5v_battle_without_defense` |  | GameAction.java (checkStateEffects) |
| Contraption | ⚪ n/a |  | Unstable | GameAction.java (checkStateEffects) |
| Scheme | ⚪ n/a |  | Archenemy | GameAction.java (checkStateEffects) |

Offen (8): CR 704.5r, CR 704.5s, CR 704.5t, CR 704.5w/x, CR 704.5y, CR 704.5z, CR 704.6c, CR 704.6d

## Combat Rule

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Attack legality | ✅ ja | `combat::summoning_sickness_haste_defender, combat_restrictions::cant_attack` | getappt/Einsatzbereitschaft/CantAttack | combat/CombatUtil.java (canAttack) |
| Summoning sickness | ✅ ja | `combat::summoning_sickness_haste_defender, spells::activated_ability_with_tap_cost_uses_stack_and_sickness` |  | combat/CombatUtil.java (isAttackerSick) |
| Attack requirements | 🟡 teilweise | `combat_restrictions::must_attack_if_able` | MustAttack geprüft, keine Maximierung über mehrere Anforderungen | combat/AttackRequirement.java, AttackConstraints.java |
| Attack targets | 🟡 teilweise | `planeswalkers::creatures_can_attack_planeswalkers_and_damage_removes_loyalty, planeswalkers::attacked_planeswalker_removed_means_no_combat_damage` | Spieler und Planeswalker der Gegner | combat/CombatUtil.java (getAllPossibleDefenders) |
| Block legality | ✅ ja | `combat::blocked_creatures_trade, combat_restrictions::cant_block` |  | combat/CombatUtil.java (canBlock) |
| Evasion | 🟡 teilweise | `combat::flying_needs_flying_or_reach_to_block, combat::menace_needs_two_blockers, combat_restrictions::unblockable` | Flying/Reach/Menace/CantBlockBy | combat/CombatUtil.java (canBeBlocked) |
| Minimum blockers | ✅ ja | `combat::menace_needs_two_blockers` |  | combat/CombatUtil.java (getMinNumBlockersForAttacker) |
| Damage assignment order | 🟡 teilweise | `combat::deathtouch_lethal_is_one_for_damage_assignment` | tödlicher Schaden vor Weitergabe | combat/Combat.java (orderBlockersForDamageAssignment) |
| Combat damage assignment | ✅ ja | `combat::blocked_creatures_trade, combat::unblocked_attacker_damages_player_and_taps` |  | combat/Combat.java (assignCombatDamage) |
| Trample | ✅ ja | `combat::trample_assigns_excess_to_player, combat::trample_with_removed_blocker_hits_player` |  | combat/Combat.java (assignAttackersDamage) |
| Simultaneous combat damage | ✅ ja | `combat::blocked_creatures_trade` |  | combat/Combat.java (dealAssignedDamage) |
| Removal from combat | 🟡 teilweise | `combat::blocked_attacker_without_trample_deals_no_damage_if_blocker_removed` | Verlassen des Battlefields | combat/Combat.java (removeFromCombat) |

Offen (14): Attack restrictions, Attack restriction: ONLY_ALONE, Attack restriction: NOT_ALONE, Attack restriction: NEED_TWO_OTHERS, Attack restriction: NEED_GREATER_POWER, Attack restriction: NEED_BLACK_OR_GREEN, Attack restriction: NEVER, Global attack restrictions, Attack costs, Block requirements, Block costs, Banding, Combat LKI, Multiple combat phases

## Mana Rule

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Mana pool | ✅ ja | `spells::manual_mana_payment_and_cancel` |  | mana/ManaPool.java |
| Mana emptying | ✅ ja | `spells::mana_pool_empties_between_steps` |  | mana/ManaPool.java (clearPool, willManaBeLostAtEndOfPhase) |
| Mana burn | ⚪ n/a |  | alte Regel, in keinem Zielformat | mana/ManaPool.java (hasBurn), GameRules.java |
| Paying mana costs | ✅ ja | `spells::manual_mana_payment_and_cancel, mana_pool::pays_colored_and_generic` |  | mana/ManaCostBeingPaid.java (payMana, payManaFromAbility) |
| Generic mana | ✅ ja | `mana_pool::pays_colored_and_generic` |  | mana/ManaCostBeingPaid.java (getGenericManaAmount) |
| Hybrid mana | ✅ ja | `mana_pool::hybrid_backtracking` |  | mana/ManaCostBeingPaid.java (canPayForShardWithColor) |
| Phyrexian mana | ✅ ja | `spells::phyrexian_mana_with_life` |  | mana/ManaCostBeingPaid.java (payPhyrexian) |
| Snow mana | 🟡 teilweise |  | Schnee-Mana wird im Pool geführt, ohne Regeltest | mana/Mana.java |
| Colorless mana | 🟡 teilweise |  | {C} nur mit farblosem Mana, ohne Regeltest | mana/ManaCostBeingPaid.java |
| X costs | ✅ ja | `spells::x_spell` |  | mana/ManaCostBeingPaid.java (getXcounter, setXManaCostPaid) |
| Mana refund | ✅ ja | `spells::manual_mana_payment_and_cancel` |  | mana/ManaRefundService.java |

Offen (4): Mana restrictions, Mana conversion, Convoke payment, Sunburst

## Zone

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Hand | ✅ ja | `turn_structure::cleanup_discards_to_hand_size_removes_damage_ends_effects` |  | zone/ZoneType.java (Hand) |
| Library | ✅ ja | `change_zone::search_library_tapped_then_shuffle, change_zone::library_bottom` |  | zone/ZoneType.java (Library) |
| Graveyard | 🟡 teilweise | `mass_discard_counter::target_player_chooses_discard` | Reihenfolge im Friedhof ohne eigenen Test | zone/ZoneType.java (Graveyard) |
| Battlefield | ✅ ja | `spells::creature_spell_resolves_onto_battlefield_with_summoning_sickness` |  | zone/ZoneType.java (Battlefield) |
| Exile | ✅ ja | `replacements::rest_in_peace_redirects_to_exile_and_no_dies_trigger` |  | zone/ZoneType.java (Exile) |
| Command | 🟡 teilweise | `effects_objects::effect_trigger_fires_from_command_zone` | Effektobjekte | zone/ZoneType.java (Command) |
| Stack | ✅ ja | `turn_structure::stack_resolves_last_in_first_out` |  | zone/ZoneType.java (Stack) |
| Ante | ⚪ n/a |  | Ante ausgeschlossen | zone/ZoneType.java (Ante) |
| SchemeDeck | ⚪ n/a |  | Archenemy | zone/ZoneType.java (SchemeDeck) |
| PlanarDeck | ⚪ n/a |  | Planechase | zone/ZoneType.java (PlanarDeck) |
| AttractionDeck | ⚪ n/a |  | Un-Sets | zone/ZoneType.java (AttractionDeck) |
| Junkyard | ⚪ n/a |  | Un-Sets | zone/ZoneType.java (Junkyard) |
| ContraptionDeck | ⚪ n/a |  | Un-Sets | zone/ZoneType.java (ContraptionDeck) |
| StickerSheets | ⚪ n/a |  | Un-Sets | zone/ZoneType.java (StickerSheets) |
| Subgame | ⚪ n/a |  | Shahrazad, in keinem Zielformat legal | zone/ZoneType.java (Subgame) |

Offen (5): Flashback, Sideboard, Merged, ExtraHand, None

## Mulligan

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| London | ✅ ja | `turn_structure::london_mulligan` |  | mulligan/LondonMulligan.java |
| Vancouver | ⚪ n/a |  | nicht in den Zielformaten | mulligan/VancouverMulligan.java |
| Paris | ⚪ n/a |  | nicht in den Zielformaten | mulligan/ParisMulligan.java |
| Original | ⚪ n/a |  | nicht in den Zielformaten | mulligan/OriginalMulligan.java |
| Houston | ⚪ n/a |  | nicht in den Zielformaten | mulligan/HoustonMulligan.java |

Offen (1): Free first mulligan

## Game Variant

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Constructed | 🟡 teilweise |  | Partien laufen | GameType.java (Constructed) |
| Sealed | ⚪ n/a |  | kein Zielformat | GameType.java (Sealed) |
| CommanderDraft | ⚪ n/a |  | kein Zielformat | GameType.java (CommanderDraft) |
| Winston | ⚪ n/a |  | kein Zielformat | GameType.java (Winston) |
| Gauntlet | ⚪ n/a |  | ausgeschlossen | GameType.java (Gauntlet) |
| Tournament | ⚪ n/a |  | Turnierverwaltung, nicht Teil der Engine | GameType.java (Tournament) |
| CommanderGauntlet | ⚪ n/a |  | ausgeschlossen | GameType.java (CommanderGauntlet) |
| Quest | ⚪ n/a |  | ausgeschlossen | GameType.java (Quest) |
| QuestCommander | ⚪ n/a |  | ausgeschlossen | GameType.java (QuestCommander) |
| QuestDraft | ⚪ n/a |  | ausgeschlossen | GameType.java (QuestDraft) |
| PlanarConquest | ⚪ n/a |  | ausgeschlossen | GameType.java (PlanarConquest) |
| Adventure | ⚪ n/a |  | ausgeschlossen | GameType.java (Adventure) |
| AdventureEvent | ⚪ n/a |  | ausgeschlossen | GameType.java (AdventureEvent) |
| Puzzle | ⚪ n/a |  | nur als Testfälle, nicht als Spielmodus | GameType.java (Puzzle) |
| DeckManager | ⚪ n/a |  | keine Spielregeln | GameType.java (DeckManager) |
| Vanguard | ⚪ n/a |  | ausgeschlossen | GameType.java (Vanguard) |
| Oathbreaker | ⚪ n/a |  | ausgeschlossen | GameType.java (Oathbreaker) |
| TinyLeaders | ⚪ n/a |  | kein Zielformat | GameType.java (TinyLeaders) |
| Brawl | ⚪ n/a |  | ausgeschlossen | GameType.java (Brawl) |
| Planeswalker | ⚪ n/a |  | Forge-Variante, ausgeschlossen | GameType.java (Planeswalker) |
| Planechase | ⚪ n/a |  | ausgeschlossen | GameType.java (Planechase) |
| Archenemy | ⚪ n/a |  | ausgeschlossen | GameType.java (Archenemy) |
| ArchenemyRumble | ⚪ n/a |  | ausgeschlossen | GameType.java (ArchenemyRumble) |
| MomirBasic | ⚪ n/a |  | ausgeschlossen | GameType.java (MomirBasic) |
| MoJhoSto | ⚪ n/a |  | ausgeschlossen | GameType.java (MoJhoSto) |

Offen (2): Draft, Commander

## Game Rule Option

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Mana burn option | ⚪ n/a |  | alte Regel | GameRules.java (hasManaBurn) |
| Order combatants | ⚪ n/a |  | Regel vor Foundations, Zielformate nutzen die aktuelle | GameRules.java (hasOrderCombatants) |
| Poison counters to lose | 🟡 teilweise | `sba::cr_704_5c_ten_poison_loses` | fest 10, nicht konfigurierbar | GameRules.java (getPoisonCountersToLose) |
| Play for ante | ⚪ n/a |  | Ante ausgeschlossen | GameRules.java (useAnte) |
| Ante rarity matching | ⚪ n/a |  | Ante ausgeschlossen | GameRules.java (getMatchAnteRarity, getAnteIncludeBasicLands) |

Offen (2): Games per match, Applied variants
