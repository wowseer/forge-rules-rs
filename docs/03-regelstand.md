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
| Keyword | 198 | 13 | 2 | 4 |
| Trigger | 137 | 0 | 0 | 11 |
| Replacement Effect | 35 | 0 | 0 | 5 |
| Static Ability | 90 | 0 | 0 | 0 |
| Effect (ApiType) | 188 | 0 | 10 | 16 |
| Cost | 42 | 5 | 2 | 1 |
| Turn Structure | 16 | 10 | 6 | 0 |
| State-Based Action | 23 | 12 | 0 | 3 |
| **Summe** | **729** | **40** | **20** | **40** |

## Keyword

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Deathtouch | ✅ ja | `combat::deathtouch_lethal_is_one_for_damage_assignment` |  | keyword/Keyword.java (SimpleKeyword) |
| Defender | ✅ ja | `combat::summoning_sickness_haste_defender` |  | keyword/Keyword.java (SimpleKeyword) |
| Double agenda | ⚪ n/a |  | Conspiracy-Draft | keyword/Keyword.java (SimpleKeyword) |
| Double Strike | ✅ ja | `combat::double_strike_deals_damage_twice` |  | keyword/Keyword.java (SimpleKeyword) |
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
| Vigilance | ✅ ja | `combat::vigilance_does_not_tap` |  | keyword/Keyword.java (SimpleKeyword) |

Offen (183): Absorb, Affinity, Afflict, Afterlife, Aftermath, Amplify, Annihilator, Ascend, Assist, Aura swap, Awaken, Backup, Banding, Bands with other, Bargain, Battle cry, Beam me up, Bestow, Blitz, Bloodthirst, Bushido, Buyback, Cascade, Casualty, Champion, Changeling, Choose a Background, Cipher, Companion, Compleated, Conspire, Convoke, Craft, Crew, Cumulative upkeep, Cycling, Dash, Daybound, Decayed, Delve, Demonstrate, Dethrone, Devour, Devoid, Disguise, Disturb, Doctor's companion, Double team, Dredge, Echo, Embalm, Emerge, Enchant, Encore, Enlist, Entwine, Epic, Equip, Escape, Escalate, Eternalize, Evoke, Evolve, Exalted, Exploit, Extort, Fabricate, Fading, Fear, Firebending, Flanking, Flashback, For Mirrodin, Foretell, Fortify, Freerunning, Frenzy, Fuse, Gift, Graft, Gravestorm, Harmonize, Haunt, Hideaway, Horsemanship, Impending, Improvise, Increment, Infect, Ingest, Intimidate, Kicker, Job select, Jump-start, Landwalk, Level up, Living metal, Living Weapon, Madness, Mayhem, Melee, Mentor, Megamorph, Miracle, Mobilize, Modular, More Than Meets the Eye, Morph, Multikicker, Mutate, Myriad, Nightbound, Ninjutsu, Outlast, Offering, Offspring, Overload, Paradigm, Partner, Partner with, Persist, Phasing, Plot, Poisonous, Protection, Prototype, Provoke, Prowess, Prowl, Rampage, Ravenous, Read ahead, Rebound, Recover, Reconfigure, Reflect, Reinforce, Renown, Replicate, Retrace, Riot, Ripple, Saddle, Scavenge, Shadow, Skulk, Sneak, Soulbond, Soulshift, Specialize, Spectacle, Splice, Split second, Spree, Squad, Start your engines, Station, Storied, Storm, Strive, Sunburst, Surge, Suspend, Teamwork, Tiered, Toxic, Training, Transfigure, Transmute, Tribute, TypeCycling, Umbra armor, Undaunted, Undying, Unearth, Unleash, Vanishing, Ward, Warp, Web-slinging, Wither, MayFlashCost, MayFlashSac

## Trigger

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Abandoned | ⚪ n/a |  | Archenemy | trigger/TriggerAbandoned.java |
| ChaosEnsues | ⚪ n/a |  | Planechase | trigger/TriggerChaosEnsues.java |
| ConjureAll | ⚪ n/a |  | Alchemy | trigger/TriggerConjureAll.java |
| CrankContraption | ⚪ n/a |  | Unstable | trigger/TriggerCrankContraption.java |
| PlanarDice | ⚪ n/a |  | Planechase | trigger/TriggerPlanarDice.java |
| PlaneswalkedFrom | ⚪ n/a |  | Planechase | trigger/TriggerPlaneswalkedFrom.java |
| PlaneswalkedTo | ⚪ n/a |  | Planechase | trigger/TriggerPlaneswalkedTo.java |
| SeekAll | ⚪ n/a |  | Alchemy | trigger/TriggerSeekAll.java |
| SetInMotion | ⚪ n/a |  | Archenemy | trigger/TriggerSetInMotion.java |
| StickerPlaced | ⚪ n/a |  | Unfinity | trigger/TriggerStickerPlaced.java |
| VisitAttraction | ⚪ n/a |  | Unfinity | trigger/TriggerVisitAttraction.java |

Offen (137): AbilityCast, AbilityResolves, AbilityTriggered, Adapt, Airbend, Always, Attached, AttackerBlocked, AttackerBlockedOnce, AttackerBlockedByCreature, AttackersDeclared, AttackersDeclaredOneTarget, AttackerUnblocked, AttackerUnblockedOnce, Attacks, BecomeMonarch, BecomeMonstrous, BecomeRenowned, BecomesCrewed, BecomesPlotted, BecomesSaddled, BecomesTarget, BecomesTargetOnce, BlockersDeclared, Blocks, CaseSolved, Championed, ChangesController, ChangesZone, ChangesZoneAll, ClaimPrize, Clashed, ClassLevelGained, CommitCrime, Connives, CollectEvidence, CounterAdded, CounterAddedOnce, CounterPlayerAddedAll, CounterTypeAddedAll, CounterAddedAll, Countered, CounterRemoved, CounterRemovedOnce, Crewed, Cycled, DamageAll, DamageDealtOnce, DamageDone, DamageDoneOnce, DamageDoneOnceByController, DamagePreventedOnce, Destroyed, Devoured, Discarded, DiscardedAll, Discover, Drawn, DungeonCompleted, Earthbend, Evolved, ExcessDamage, ExcessDamageAll, ElementalBend, Enlisted, Exerted, Exiled, Exploited, Explores, FacesDilemma, Fight, FightOnce, Firebend, FlippedCoin, FlippedCoinOnce, Forage, Foretell, FullyUnlock, GiveGift, Immediate, Investigated, LandPlayed, LifeGained, LifeLost, LifeLostAll, LosesGame, ManaAdded, ManaExpend, ManifestDread, Mentored, Milled, MilledOnce, MilledAll, Mutates, NewGame, PayCumulativeUpkeep, PayEcho, PayLife, Phase, PhaseIn, PhaseOut, PhaseOutAll, Proliferate, RingTemptsYou, RolledDie, RolledDieOnce, RoomEntered, Saddled, Sacrificed, SacrificedOnce, Scry, SearchedLibrary, Shuffled, Specializes, SpellAbilityCast, SpellAbilityCopy, SpellCast, SpellCastOrCopy, SpellCopy, Stationed, Surveil, TakesInitiative, TapAll, Taps, TapsForMana, TokenCreated, TokenCreatedOnce, Trains, Transformed, TurnBegin, TurnFaceUp, Unattached, UnlockDoor, UntapAll, Untaps, Vote, Waterbend

## Replacement Effect

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| AssembleContraption | ⚪ n/a |  | Unstable | replacement/ReplaceAssembleContraption.java |
| PlanarDiceResult | ⚪ n/a |  | Planechase | replacement/ReplacePlanarDiceResult.java |
| Planeswalk | ⚪ n/a |  | Planechase | replacement/ReplacePlaneswalk.java |
| RollPlanarDice | ⚪ n/a |  | Planechase | replacement/ReplaceRollPlanarDice.java |
| SetInMotion | ⚪ n/a |  | Archenemy | replacement/ReplaceSetInMotion.java |

Offen (35): AddCounter, AssignDealDamage, Attached, BeginPhase, BeginTurn, Cascade, Connive, Counter, CopySpell, CreateToken, DamageDone, DealtDamage, DeclareBlocker, Destroy, Draw, DrawCards, Explore, GainLife, GameLoss, GameWin, Learn, LifeReduced, LoseMana, Mill, Moved, PayLife, ProduceMana, Proliferate, RemoveCounter, RollDice, Scry, Tap, Transform, TurnFaceUp, Untap

## Static Ability

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|

Offen (90): Continuous, CantAttackUnless, CantBlockUnless, OptionalAttackCost, OptionalCost, AlternativeCost, CantBeCast, CantBeActivated, CantPlayLand, DisableTriggers, Panharmonicon, MustTarget, CantAttack, CanAttackDefender, CantBlock, CantBlockBy, CanAttackIfHaste, CanBlockIfReach, CanBlockIfShadow, MinMaxBlocker, BlockTapped, AttackVigilance, MustAttack, PlayerMustAttack, MustBlock, AssignCombatDamageAsUnblocked, CombatDamageToughness, CombatDamageNegatePower, ColorlessDamageSource, NoCleanupDamage, BlockRestrict, CantGainLife, CantLoseLife, CantChangeLife, CantPayLife, RaiseCost, ReduceCost, SetCost, IgnoreHexproof, IgnoreShroud, AttackRestrict, AssignNoCombatDamage, CanAdapt, CantBeCopied, CantBeBeamedUp, CantBeSuspected, CantBecomeMonarch, CantGainControl, CantAttach, CantCrew, CantDraw, DrawFromBottom, CantDiscard, CantExile, CantPhaseIn, CantPhaseOut, CantPreventDamage, CantPutCounter, CantRegenerate, CantSacrifice, CantTarget, CantTransform, CantVenture, CantChangeDayTime, ActivateAbilityAsIfHaste, CastWithFlash, IgnoreLandwalk, IgnoreLegendRule, IgnorePlaneswalkerZeroLoyaltyRule, MaxCounter, InfectDamage, WitherDamage, FlipCoinMod, FlipCoinDoubler, PlotZone, NumLoyaltyAct, Activations, Devotion, GainLifeRadiation, SurveilNum, TapPowerValue, UnspentMana, ManaBurn, ManaConvert, UntapOtherPlayer, TurnReversed, PhaseReversed, AttackRequirement, CountersRemain, ManaRestriction

## Effect (ApiType)

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Abandon | ⚪ n/a |  | Archenemy | ability/effects/AbandonEffect.java |
| AdvanceCrank | ⚪ n/a |  | Unstable | ability/effects/AdvanceCrankEffect.java |
| AssembleContraption | ⚪ n/a |  | Unstable | ability/effects/AssembleContraptionEffect.java |
| ChaosEnsues | ⚪ n/a |  | Planechase | ability/effects/ChaosEnsuesEffect.java |
| ChooseSector | ⚪ n/a |  | Unstable | ability/effects/ChooseSectorEffect.java |
| DealDamage | 🟡 teilweise | `spells::bolt_face_and_creature_and_fizzle` | Grundform | ability/effects/DamageDealEffect.java |
| Destroy | 🟡 teilweise | `spells::destroy_respects_indestructible` | Grundform | ability/effects/DestroyEffect.java |
| Draft | ⚪ n/a |  | Conspiracy-Draft | ability/effects/DraftEffect.java |
| Draw | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/DrawEffect.java |
| GainLife | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/LifeGainEffect.java |
| Heist | ⚪ n/a |  | Alchemy | ability/effects/HeistEffect.java |
| Intensify | ⚪ n/a |  | Alchemy | ability/effects/IntensifyEffect.java |
| LoseLife | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/LifeLoseEffect.java |
| LosePerpetual | ⚪ n/a |  | Alchemy | ability/effects/LosePerpetualEffect.java |
| Mana | 🟡 teilweise | `spells::manual_mana_payment_and_cancel` |  | ability/effects/ManaEffect.java |
| OpenAttraction | ⚪ n/a |  | Unfinity | ability/effects/OpenAttractionEffect.java |
| Planeswalk | ⚪ n/a |  | Planechase | ability/effects/PlaneswalkEffect.java |
| Pump | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | NumAtt/NumDef/KW bis Zugende | ability/effects/PumpEffect.java |
| PutCounter | 🟡 teilweise | `sba::cr_704_5q_plus_and_minus_counters_annihilate` | Grundform | ability/effects/CountersPutEffect.java |
| PutSticker | ⚪ n/a |  | Unfinity | ability/effects/PutStickerEffect.java |
| RollPlanarDice | ⚪ n/a |  | Planechase | ability/effects/RollPlanarDiceEffect.java |
| RunChaos | ⚪ n/a |  | Planechase | ability/effects/RunChaosEffect.java |
| Seek | ⚪ n/a |  | Alchemy | ability/effects/SeekEffect.java |
| SetInMotion | ⚪ n/a |  | Archenemy | ability/effects/SetInMotionEffect.java |
| Tap | 🟡 teilweise |  | Grundform | ability/effects/TapEffect.java |
| Untap | 🟡 teilweise |  | Grundform | ability/effects/UntapEffect.java |

Offen (178): ActivateAbility, AddOrRemoveCounter, AddPhase, AddTurn, Airbend, AlterAttribute, Amass, Animate, AnimateAll, Attach, Assimilate, AssignGroup, Balance, BecomeMonarch, BecomesBlocked, BidLife, Blight, Block, Bond, Branch, Camouflage, ChangeCombatants, ChangeSpeed, ChangeTargets, ChangeText, ChangeX, ChangeZone, ChangeZoneAll, Charm, ChooseCard, ChooseColor, ChooseDirection, ChooseEvenOdd, ChooseNumber, ChoosePlayer, ChooseSource, ChooseType, ClaimThePrize, Clash, ClassLevelUp, Cleanup, Cloak, Clone, Connive, CopyPermanent, CopySpellAbility, ControlSpell, ControlPlayer, Counter, DamageAll, Detain, DayTime, Debuff, DelayedTrigger, DestroyAll, Dig, DigMultiple, DigUntil, Discard, Discover, DrainMana, EachDamage, Earthbend, Effect, Empower, Encode, EndCombatPhase, EndTurn, Endure, ExchangeLife, ExchangeLifeVariant, ExchangeControl, ExchangeControlVariant, ExchangePower, ExchangeZone, ExchangeTextBox, Explore, Fight, FlipCoin, FlipOntoBattlefield, Fog, GainControl, GainControlVariant, GainOwnership, GameDrawn, GenericChoice, Goad, Haunt, HealDamage, Investigate, ImmediateTrigger, Incubate, Learn, LookAt, LosesGame, MakeCard, ManaReflected, Manifest, ManifestDread, Meld, Mill, MoveCounter, MultiplePiles, MultiplyCounter, MustBlock, Mutate, NameCard, PeekAndReveal, PermanentCreature, PermanentNoncreature, Phases, Play, PlayLandVariant, Poison, PreventDamage, Proliferate, Protection, ProtectionAll, PumpAll, PutCounterAll, Radiation, Recruit, RearrangeTopOfLibrary, Regenerate, Regeneration, RemoveCounter, RemoveCounterAll, RemoveFromCombat, RemoveFromGame, RemoveFromMatch, ReorderZone, Repeat, RepeatEach, ReplaceCounter, ReplaceEffect, ReplaceMana, ReplaceDamage, ReplaceToken, ReplaceSplitDamage, RestartGame, Reveal, RevealHand, ReverseTurnOrder, RingTemptsYou, RollDice, Sacrifice, SacrificeAll, Scry, SetLife, SetState, Shuffle, SkipPhase, SkipTurn, StoreSVar, Subgame, Surveil, SwitchBlock, TakeInitiative, TapAll, TapOrUntap, TapOrUntapAll, TimeTravel, Token, TwoPiles, Unattach, UnlockDoor, UntapAll, Venture, VillainousChoice, Vote, WinsGame, BlankLine, DamageResolve, ChangeZoneResolve, CompanionChoose, InternalLegendaryRule, InternalIgnoreEffect, InternalRadiation

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

Offen (35): tapXType, untapXType, Draw, DamageYou, GainLife, PayEnergy, Exile, ExileFromStack, ExiledMoveToGrave, Mill, PutCardToLib, Return, Reveal, RevealChosen, AddCounter, AddCounterYou, SubCounter, RemoveAnyCounter, GainControl, Unattach, Exert, Enlist, FlipCoin, RollDice, ChooseColor, ChooseCreatureType, CollectEvidence, Forage, PromiseGift, Behold, BeholdExile, Blight, Teamwork, Waterbend, Cost adjustment

## Turn Structure

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Untap step | 🟡 teilweise | `turn_structure::untap_step_untaps_active_players_permanents` | Enttappen + Zugbeginn-Kontrolle fertig | phase/PhaseType.java (UNTAP) |
| Upkeep step | ✅ ja | `turn_structure::steps_in_order_and_combat_skips_without_attackers` | Trigger „zu Beginn des Versorgungssegments“ kommen mit dem Trigger-System | phase/PhaseType.java (UPKEEP) |
| Draw step | ✅ ja | `turn_structure::draw_step_draws_and_first_player_skips_first_draw` | inkl. CR 103.8a | phase/PhaseType.java (DRAW) |
| Precombat main phase | 🟡 teilweise | `spells::land_play_once_per_turn_sorcery_timing` | Länder/Hexereien fertig | phase/PhaseType.java (MAIN1) |
| Beginning of combat step | ✅ ja | `turn_structure::steps_in_order_and_combat_skips_without_attackers` |  | phase/PhaseType.java (COMBAT_BEGIN) |
| Declare attackers step | 🟡 teilweise | `combat::summoning_sickness_haste_defender` | nur Spieler als Verteidiger | phase/PhaseType.java (COMBAT_DECLARE_ATTACKERS) |
| Declare blockers step | 🟡 teilweise | `combat::flying_needs_flying_or_reach_to_block` | Flying/Reach/Menace | phase/PhaseType.java (COMBAT_DECLARE_BLOCKERS) |
| First strike damage step | ✅ ja | `combat::first_strike_kills_before_regular_damage` |  | phase/PhaseType.java (COMBAT_FIRST_STRIKE_DAMAGE) |
| Combat damage step | ✅ ja | `combat::trample_assigns_excess_to_player` | Regeln ab Foundations (keine Schadenszuweisungsreihenfolge) | phase/PhaseType.java (COMBAT_DAMAGE) |
| End of combat step | ✅ ja | `turn_structure::steps_in_order_and_combat_skips_without_attackers` |  | phase/PhaseType.java (COMBAT_END) |
| Postcombat main phase | 🟡 teilweise | `turn_structure::steps_in_order_and_combat_skips_without_attackers` | wie Precombat main phase | phase/PhaseType.java (MAIN2) |
| End step | ✅ ja | `turn_structure::cleanup_discards_to_hand_size_removes_damage_ends_effects` |  | phase/PhaseType.java (END_OF_TURN) |
| Cleanup step | ✅ ja | `turn_structure::cleanup_discards_to_hand_size_removes_damage_ends_effects` | inkl. CR 514.3a (erneuter Cleanup) | phase/PhaseType.java (CLEANUP) |
| Priority and stack | ✅ ja | `turn_structure::stack_resolves_last_in_first_out` | CR 117, 405, 608 inkl. Fizzle (608.2b) | phase/ / GameAction.java |
| Layer system | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | nur Layer 6 (Fähigkeiten hinzufügen) und 7c | phase/ / GameAction.java |
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
| CR 704.5q | ✅ ja | `sba::cr_704_5q_plus_and_minus_counters_annihilate` |  | GameAction.java (checkStateEffects) |
| CR 704.5u | ⚪ n/a |  | Space Sculptor (Unstable) | GameAction.java (checkStateEffects) |
| CR 704.5v | ✅ ja | `sba::cr_704_5v_battle_without_defense` |  | GameAction.java (checkStateEffects) |
| Contraption | ⚪ n/a |  | Unstable | GameAction.java (checkStateEffects) |
| Scheme | ⚪ n/a |  | Archenemy | GameAction.java (checkStateEffects) |

Offen (11): CR 704.5m, CR 704.5n, CR 704.5p, CR 704.5r, CR 704.5s, CR 704.5t, CR 704.5w/x, CR 704.5y, CR 704.5z, CR 704.6c, CR 704.6d
