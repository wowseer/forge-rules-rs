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
| Keyword | 198 | 40 | 16 | 4 |
| Trigger | 137 | 3 | 27 | 11 |
| Replacement Effect | 35 | 0 | 6 | 5 |
| Static Ability | 90 | 3 | 10 | 0 |
| Effect (ApiType) | 188 | 9 | 68 | 16 |
| Cost | 42 | 7 | 5 | 1 |
| Turn Structure | 16 | 10 | 6 | 0 |
| State-Based Action | 23 | 15 | 1 | 3 |
| Combat Rule | 26 | 7 | 5 | 0 |
| Mana Rule | 14 | 8 | 3 | 1 |
| Zone | 12 | 5 | 2 | 8 |
| Mulligan | 2 | 1 | 0 | 4 |
| Game Variant | 3 | 0 | 2 | 24 |
| Game Rule Option | 3 | 0 | 1 | 4 |
| **Summe** | **789** | **108** | **152** | **81** |

## Keyword

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Affinity | ✅ ja | `cost_helpers::affinity_reduces_cost_per_artifact` | CR 702.41a: {1} weniger je bleibender Karte des Typs unter eigener Kontrolle (Kostensenkung CR 601.2f) | keyword/Keyword.java (Affinity) |
| Aftermath | ✅ ja | `split::aftermath_half_only_from_graveyard_then_exiled` | CR 702.127: zweite Hälfte nur aus dem Friedhof, erste nicht von dort, danach ins Exil | keyword/Keyword.java (SimpleKeyword) |
| Bushido | ✅ ja | `mechanics_batch1::bushido_and_flanking_in_combat` | CR 702.45: blockt oder wird geblockt → +n/+n bis Zugende | keyword/Keyword.java (KeywordWithAmount) |
| Cascade | ✅ ja | `storm_cascade::cascade_casts_cheaper_nonland_for_free` | CR 702.85: exilieren bis Nichtland mit geringerem Manawert, gratis wirken dürfen, Rest zufällig unter die Bibliothek | keyword/Keyword.java (SimpleKeyword) |
| Changeling | ✅ ja | `triggers::attacks_alone_second_spell_damage_dealt_once_changeling` | CR 702.73a: jeder Kreaturtyp (laut Typliste), in allen Zonen | keyword/Keyword.java (SimpleKeyword) |
| Convoke | ✅ ja | `cost_helpers::convoke_taps_creatures_for_colored_and_generic_mana, cost_helpers::convoke_green_creature_cannot_pay_red` | CR 702.51a: getappte Kreatur zahlt {1} oder ein Symbol ihrer Farbe (auch Hybrid), vor dem Mana | keyword/Keyword.java (SimpleKeyword) |
| Crew | 🟡 teilweise | `crew::crew_with_enough_power_animates_until_end_of_turn, crew::crew_needs_total_power` | CR 702.122: wie Java als AB$ Animate mit tapXType<Any/Creature.Other+withTotalPowerGEn> | keyword/Keyword.java (KeywordWithAmount) |
| Cycling | 🟡 teilweise | `keywords_misc::cycling_from_hand_discards_and_draws, keywords_misc::cycling_not_offered_from_battlefield_or_without_mana` | Aktivierung aus der Hand, Abwerfen als Kosten | keyword/Keyword.java (KeywordWithCost) |
| Daybound | ✅ ja | `mechanics_batch4::day_night_transforms_werewolf` | CR 702.145/726: Tag beginnt mit Daybound-Karte, Wechsel nach Zaubern des vorigen aktiven Spielers, passende Seite | keyword/Keyword.java (SimpleKeyword) |
| Deathtouch | ✅ ja | `combat::deathtouch_lethal_is_one_for_damage_assignment` |  | keyword/Keyword.java (SimpleKeyword) |
| Defender | ✅ ja | `combat::summoning_sickness_haste_defender` |  | keyword/Keyword.java (SimpleKeyword) |
| Delve | ✅ ja | `cost_helpers::delve_exiles_graveyard_cards_for_generic` | CR 702.66a: exilierte Friedhofskarten zahlen je {1}, nur generisch | keyword/Keyword.java (SimpleKeyword) |
| Devoid | ✅ ja | `keywords_misc::devoid_is_colorless_in_all_zones` |  | keyword/Keyword.java (SimpleKeyword) |
| Disguise | 🟡 teilweise |  | CR 702.168: wie Morph, verdeckt mit Ward {2} | keyword/Keyword.java (KeywordWithCost) |
| Double agenda | ⚪ n/a |  | Conspiracy-Draft | keyword/Keyword.java (SimpleKeyword) |
| Double Strike | ✅ ja | `combat::double_strike_deals_damage_twice` |  | keyword/Keyword.java (SimpleKeyword) |
| Enchant | 🟡 teilweise | `auras_equipment::aura_attaches_on_resolution_and_buffs, aura_control::enchant_with_description_and_defined_enchanted, aura_control::control_aura_steals_until_it_leaves, aura_control::enchanted_controller_upkeep_trigger_and_enchanted_filter` | Aura-Zauber mit Ziel, angelegt beim Auflösen, Fizzle | keyword/Keyword.java (KeywordWithType) |
| Equip | 🟡 teilweise | `auras_equipment::equip_sorcery_speed_own_creature_and_moves` | Kosten, optionaler Filter, Hexerei-Geschwindigkeit | keyword/Keyword.java (Equip) |
| Exalted | ✅ ja | `evasion::exalted_pumps_lone_attacker` | CR 702.83: je Instanz +1/+1, wenn eine Kreatur allein angreift | keyword/Keyword.java (SimpleKeyword) |
| Fear | ✅ ja | `evasion::evasion_rules` | CR 702.36: nur von Artefakt- und/oder schwarzen Kreaturen blockbar | keyword/Keyword.java (SimpleKeyword) |
| First Strike | ✅ ja | `combat::first_strike_kills_before_regular_damage` |  | keyword/Keyword.java (SimpleKeyword) |
| Flanking | ✅ ja | `mechanics_batch1::bushido_and_flanking_in_combat` | CR 702.25: Blocker ohne Flanking erhält -1/-1 | keyword/Keyword.java (SimpleKeyword) |
| Flash | ✅ ja | `spells::flash_allows_instant_speed_casting` |  | keyword/Keyword.java (SimpleKeyword) |
| Flying | ✅ ja | `combat::flying_needs_flying_or_reach_to_block` |  | keyword/Keyword.java (SimpleKeyword) |
| Haste | ✅ ja | `combat::summoning_sickness_haste_defender` |  | keyword/Keyword.java (SimpleKeyword) |
| Hexproof | 🟡 teilweise | `spells::hexproof_only_blocks_opponents, player_statics::player_hexproof_only_against_opponents` | auch Spieler (CR 702.11d) | keyword/Keyword.java (Hexproof) |
| Hidden agenda | ⚪ n/a |  | Conspiracy-Draft | keyword/Keyword.java (SimpleKeyword) |
| Horsemanship | ✅ ja | `evasion::evasion_rules` | CR 702.31: nur von Kreaturen mit Horsemanship blockbar | keyword/Keyword.java (SimpleKeyword) |
| Improvise | ✅ ja | `cost_helpers::improvise_taps_artifacts_for_generic_only` | CR 702.126a: getappte Artefakte zahlen je {1}, nur generisch | keyword/Keyword.java (SimpleKeyword) |
| Indestructible | ✅ ja | `spells::destroy_respects_indestructible` |  | keyword/Keyword.java (SimpleKeyword) |
| Infect | ✅ ja | `infect::infect_and_wither_damage` | CR 702.90: Giftmarken an Spieler, -1/-1-Marken an Kreaturen | keyword/Keyword.java (SimpleKeyword) |
| Intimidate | ✅ ja | `evasion::evasion_rules` | CR 702.13: nur von Artefaktkreaturen oder Kreaturen mit gemeinsamer Farbe blockbar | keyword/Keyword.java (SimpleKeyword) |
| Kicker | 🟡 teilweise | `kicker::kicked_spell_pays_more_and_does_more, kicker::unkicked_spell_and_no_offer_when_unaffordable, kicker::kicked_creature_enters_with_counters_and_triggers, kicker::unkicked_creature_plain` | CR 702.33: ein oder zwei Kicker-Kosten (Mana und Nicht-Mana), Ankündigung 601.2b, gekickt-Status vom Stapel auf die bleibende Karte, Filter kicked/kicked 1/2, Condition Kicked, Count$Kicked, etbCounter mit CheckSVar | keyword/Keyword.java (Kicker) |
| Landwalk | ✅ ja | `evasion::evasion_rules` | CR 702.14: nicht blockbar, wenn der Verteidiger ein passendes Land kontrolliert (Landwalk:<Filter>) | keyword/Keyword.java (Landwalk) |
| Lifelink | ✅ ja | `combat::deathtouch_and_lifelink_in_combat` |  | keyword/Keyword.java (SimpleKeyword) |
| Menace | ✅ ja | `combat::menace_needs_two_blockers` |  | keyword/Keyword.java (SimpleKeyword) |
| Megamorph | 🟡 teilweise | `morph::megamorph_adds_counter_and_leaving_reveals` | CR 702.37b: wie Morph, beim Umdrehen +1/+1-Marke | keyword/Keyword.java (KeywordWithCost) |
| Morph | 🟡 teilweise | `morph::cast_face_down_then_turn_up_with_trigger` | CR 702.37: verdeckt für {3} wirken, als Sonderaktion für die Morph-Kosten umdrehen, verdeckt 2/2 ohne Name/Farbe/Fähigkeiten (CR 708.2), beim Verlassen aufgedeckt | keyword/Keyword.java (KeywordWithCost) |
| Nightbound | ✅ ja | `mechanics_batch4::day_night_transforms_werewolf` | CR 702.145/726 | keyword/Keyword.java (SimpleKeyword) |
| Partner | 🟡 teilweise | `storm_cascade::partner_is_supported` | Deckbau-Keyword ohne Spielwirkung (auch Choose a Background, Doctor's companion, Partner:<Gruppe>) | keyword/Keyword.java (Partner) |
| Persist | ✅ ja | `mechanics_batch1::undying_and_persist_return_once` | CR 702.79: stirbt ohne -1/-1-Marke → kehrt mit einer zurück | keyword/Keyword.java (SimpleKeyword) |
| Protection | 🟡 teilweise | `protection::cannot_be_targeted_by_black_spell, protection::cannot_be_blocked_by_black_creatures_and_prevents_their_damage, protection::black_aura_falls_off, protection::protection_effect_with_chosen_color` | CR 702.16 DEBT: Schaden verhindert, Auren/Ausrüstung fallen ab, nicht blockbar, nicht Ziel | keyword/Keyword.java (Protection) |
| Prowess | ✅ ja | `keywords_misc::prowess_pumps_on_noncreature_spell` |  | keyword/Keyword.java (SimpleKeyword) |
| Reach | ✅ ja | `combat::flying_needs_flying_or_reach_to_block` |  | keyword/Keyword.java (SimpleKeyword) |
| Rebound | 🟡 teilweise | `storm_cascade::rebound_recasts_next_upkeep` | CR 702.88: aus der Hand gewirkt → Exil, im nächsten eigenen Upkeep gratis wirkbar (ohne verzögerten Trigger auf dem Stapel) | keyword/Keyword.java (SimpleKeyword) |
| Shadow | ✅ ja | `evasion::evasion_rules` | CR 702.28: blockt und wird geblockt nur von Shadow | keyword/Keyword.java (SimpleKeyword) |
| Shroud | ✅ ja | `spells::shroud_blocks_all_targeting` |  | keyword/Keyword.java (SimpleKeyword) |
| Skulk | ✅ ja | `evasion::evasion_rules` | CR 702.118: nicht von Kreaturen mit größerer Stärke blockbar | keyword/Keyword.java (SimpleKeyword) |
| Space sculptor | ⚪ n/a |  | Unstable | keyword/Keyword.java (SimpleKeyword) |
| Split second | ✅ ja | `split::split_second_locks_out_responses` | CR 702.61: solange auf dem Stapel nur Mana-Fähigkeiten | keyword/Keyword.java (SimpleKeyword) |
| Start your engines | 🟡 teilweise | `mechanics_batch4::speed_rises_when_opponent_loses_life_on_your_turn` | CR 702.179: Geschwindigkeit 1, +1 je Zug bei Lebensverlust eines Gegners im eigenen Zug (max. 4), Condition MaxSpeed, Count$YourSpeed | keyword/Keyword.java (SimpleKeyword) |
| Starting intensity | ⚪ n/a |  | Alchemy | keyword/Keyword.java (KeywordWithAmount) |
| Storm | ✅ ja | `storm_cascade::storm_copies_for_each_earlier_spell` | CR 702.40: SpellCast-Trigger der Karte, Kopie je früher in diesem Zug gewirktem Zauber, neue Ziele wählbar | keyword/Keyword.java (SimpleKeyword) |
| Toxic | ✅ ja | `infect::toxic_adds_poison_on_combat_damage_only` | CR 702.164: zusätzlich n Giftmarken bei Kampfschaden an Spielern | keyword/Keyword.java (KeywordWithAmount) |
| Trample | 🟡 teilweise | `combat::trample_assigns_excess_to_player` | Trample über Planeswalker fehlt | keyword/Keyword.java (Trample) |
| TypeCycling | 🟡 teilweise | `keywords_misc::typecycling_searches_library_for_type` | Bibliothekssuche nach Typ | keyword/Keyword.java (KeywordWithCostAndType) |
| Undying | ✅ ja | `mechanics_batch1::undying_and_persist_return_once` | CR 702.93: stirbt ohne +1/+1-Marke → kehrt mit einer zurück | keyword/Keyword.java (SimpleKeyword) |
| Vigilance | ✅ ja | `combat::vigilance_does_not_tap` |  | keyword/Keyword.java (SimpleKeyword) |
| Ward | 🟡 teilweise | `triggers::ward_counters_unless_paid` | Ward:<Kosten> mit Mana/PayLife/Discard/Sac | keyword/Keyword.java (Ward) |
| Wither | ✅ ja | `infect::infect_and_wither_damage` | CR 702.80: -1/-1-Marken an Kreaturen | keyword/Keyword.java (SimpleKeyword) |

Offen (142): Absorb, Afflict, Afterlife, Amplify, Annihilator, Ascend, Assist, Aura swap, Awaken, Backup, Banding, Bands with other, Bargain, Battle cry, Beam me up, Bestow, Blitz, Bloodthirst, Buyback, Casualty, Champion, Choose a Background, Cipher, Companion, Compleated, Conspire, Craft, Cumulative upkeep, Dash, Decayed, Demonstrate, Dethrone, Devour, Disturb, Doctor's companion, Double team, Dredge, Echo, Embalm, Emerge, Encore, Enlist, Entwine, Epic, Escape, Escalate, Eternalize, Evoke, Evolve, Exploit, Extort, Fabricate, Fading, Firebending, Flashback, For Mirrodin, Foretell, Fortify, Freerunning, Frenzy, Fuse, Gift, Graft, Gravestorm, Harmonize, Haunt, Hideaway, Impending, Increment, Ingest, Job select, Jump-start, Level up, Living metal, Living Weapon, Madness, Mayhem, Melee, Mentor, Miracle, Mobilize, Modular, More Than Meets the Eye, Multikicker, Mutate, Myriad, Ninjutsu, Outlast, Offering, Offspring, Overload, Paradigm, Partner with, Phasing, Plot, Poisonous, Prototype, Provoke, Prowl, Rampage, Ravenous, Read ahead, Recover, Reconfigure, Reflect, Reinforce, Renown, Replicate, Retrace, Riot, Ripple, Saddle, Scavenge, Sneak, Soulbond, Soulshift, Specialize, Spectacle, Splice, Spree, Squad, Station, Storied, Strive, Sunburst, Surge, Suspend, Teamwork, Tiered, Training, Transfigure, Transmute, Tribute, Umbra armor, Undaunted, Unearth, Unleash, Vanishing, Warp, Web-slinging, MayFlashCost, MayFlashSac

## Trigger

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Abandoned | ⚪ n/a |  | Archenemy | trigger/TriggerAbandoned.java |
| AbilityCast | 🟡 teilweise | `more_triggers::counter_added_once_and_ability_cast` | aktivierte Nicht-Mana-Fähigkeiten, ValidCard, ValidActivatingPlayer, ValidSA Activated.Loyalty | trigger/TriggerSpellAbilityCastOrCopy.java |
| Always | 🟡 teilweise | `mechanics_batch1::untaps_trigger_and_state_trigger` | CR 603.8 Zustandstrigger: Bedingungen, erneut erst nach Verlassen des Stapels | trigger/TriggerAlways.java |
| AttackerBlocked | 🟡 teilweise | `triggers::blocks_attacker_blocked_and_unblocked_triggers` | ValidCard, ValidBlocker/Amount | trigger/TriggerAttackerBlocked.java |
| AttackerBlockedByCreature | 🟡 teilweise | `triggers::attacker_blocked_by_creature_names_the_blocker` | ValidCard, ValidBlocker, TriggeredBlocker | trigger/TriggerAttackerBlockedByCreature.java |
| AttackersDeclared | 🟡 teilweise | `triggers::attackers_declared_once_with_minimum_amount` | AttackingPlayer, AttackedTarget (nur Spieler), ValidAttackers/Amount, TriggeredAttackingPlayer | trigger/TriggerAttackersDeclared.java |
| AttackersDeclaredOneTarget | 🟡 teilweise | `triggers::attackers_declared_one_target_per_attacked_player` | je angegriffenem Spieler, TriggeredAttackedTarget | trigger/TriggerAttackersDeclared.java |
| AttackerUnblocked | 🟡 teilweise | `triggers::blocks_attacker_blocked_and_unblocked_triggers` | ValidCard, ValidDefender | trigger/TriggerAttackerUnblocked.java |
| Attacks | 🟡 teilweise | `triggers::attack_trigger_with_triggered_player, triggers::attacks_alone_second_spell_damage_dealt_once_changeling` | ValidCard, Alone | trigger/TriggerAttacks.java |
| BecomesTarget | 🟡 teilweise | `triggers::becomes_target_only_by_own_spells, triggers::ward_counters_unless_paid` | ValidTarget, ValidSource (Spell/Ability/SpellAbility + YouCtrl/OppCtrl), TriggeredSourceSA | trigger/TriggerBecomesTarget.java |
| Blocks | 🟡 teilweise | `triggers::blocks_attacker_blocked_and_unblocked_triggers` | ValidCard, ValidBlocked | trigger/TriggerBlocks.java |
| ChangesZone | 🟡 teilweise | `triggers::dies_trigger_looks_back_in_time` | Grundform inkl. Rückblick (603.10a) und gleichzeitiger Tode | trigger/TriggerChangesZone.java |
| ChangesZoneAll | 🟡 teilweise | `triggers::changes_zone_all_triggers_once_per_simultaneous_event` | Bündel je Effekt, SBA-Durchgang und Kostenzahlung | trigger/TriggerChangesZoneAll.java |
| ChaosEnsues | ⚪ n/a |  | Planechase | trigger/TriggerChaosEnsues.java |
| ConjureAll | ⚪ n/a |  | Alchemy | trigger/TriggerConjureAll.java |
| CounterAdded | 🟡 teilweise | `sagas::adding_several_lore_counters_triggers_each_chapter` | je einzelner Marke (Java TriggerCounterAdded), ValidCard, CounterType, CounterAmount | trigger/TriggerCounterAdded.java |
| CounterAddedOnce | 🟡 teilweise | `more_triggers::counter_added_once_and_ability_cast` | einmal je Ereignis (auch Marken beim Eintreten), ValidCard, CounterType, ValidSource (Controller der Effektquelle), TriggerCount Amount | trigger/TriggerCounterAddedOnce.java |
| CrankContraption | ⚪ n/a |  | Unstable | trigger/TriggerCrankContraption.java |
| Cycled | 🟡 teilweise | `keywords_misc::cycled_and_discarded_triggers` | ValidCard (auch Self aus der Hand), ValidPlayer | trigger/TriggerCycled.java |
| DamageDealtOnce | 🟡 teilweise | `triggers::attacks_alone_second_spell_damage_dealt_once_changeling` | einmal je Quelle bei gleichzeitigem Schaden, ValidSource, ValidTarget, CombatDamage, TriggerCount$DamageAmount | trigger/TriggerDamageDealtOnce.java |
| DamageDone | 🟡 teilweise | `triggers::combat_damage_trigger` | ValidSource/ValidTarget/CombatDamage | trigger/TriggerDamageDone.java |
| DamageDoneOnce | 🟡 teilweise | `triggers::damage_done_once_triggers_once_per_recipient, triggers::trigger_count_damage_and_life_amount` | einmal je Empfänger bei gleichzeitigem Schaden (Kampf, DealDamage, DamageAll) | trigger/TriggerDamageDoneOnce.java |
| Discarded | 🟡 teilweise | `keywords_misc::cycled_and_discarded_triggers` | ValidCard | trigger/TriggerDiscarded.java |
| Drawn | 🟡 teilweise | `triggers::drawn_number_life_gained_and_taps_triggers` | ValidCard, ValidPlayer, Number | trigger/TriggerDrawn.java |
| LandPlayed | ✅ ja | `mechanics_batch3::landfall_trigger_and_extra_land_play` | ValidCard, Origin | trigger/TriggerLandPlayed.java |
| LifeGained | 🟡 teilweise | `triggers::drawn_number_life_gained_and_taps_triggers, triggers::trigger_count_damage_and_life_amount` | ValidPlayer, TriggerCount$LifeAmount | trigger/TriggerLifeGained.java |
| Phase | 🟡 teilweise | `triggers::upkeep_trigger_with_sub_ability` | Phase$ als Liste einzelner Schritte | trigger/TriggerPhase.java |
| PlanarDice | ⚪ n/a |  | Planechase | trigger/TriggerPlanarDice.java |
| PlaneswalkedFrom | ⚪ n/a |  | Planechase | trigger/TriggerPlaneswalkedFrom.java |
| PlaneswalkedTo | ⚪ n/a |  | Planechase | trigger/TriggerPlaneswalkedTo.java |
| Sacrificed | 🟡 teilweise | `sacrifice_unless::sacrificed_trigger_only_for_own_sacrifices_not_destroy, sacrifice_unless::self_sacrifice_as_cost_triggers_from_lki` | ValidCard (LKI), ValidPlayer, auch bei Opferkosten | trigger/TriggerSacrificed.java |
| SeekAll | ⚪ n/a |  | Alchemy | trigger/TriggerSeekAll.java |
| SetInMotion | ⚪ n/a |  | Archenemy | trigger/TriggerSetInMotion.java |
| SpellCast | 🟡 teilweise | `triggers::spell_cast_trigger_resolves_before_spell, triggers::spell_cast_targets_valid_heroic, triggers::attacks_alone_second_spell_damage_dealt_once_changeling` | ValidCard/ValidActivatingPlayer, TargetsValid, ActivatorThisTurnCast | trigger/TriggerSpellAbilityCastOrCopy.java |
| SpellCastOrCopy | ✅ ja | `mechanics_batch2::spell_cast_or_copy_and_damaged_by` | wie SpellCast, zusätzlich beim Kopieren | trigger/TriggerSpellAbilityCastOrCopy.java |
| StickerPlaced | ⚪ n/a |  | Unfinity | trigger/TriggerStickerPlaced.java |
| Taps | 🟡 teilweise | `triggers::drawn_number_life_gained_and_taps_triggers` | ValidCard, ValidPlayer | trigger/TriggerTaps.java |
| TapsForMana | 🟡 teilweise | `taps_for_mana::flare_doubles_land_mana_immediately, taps_for_mana::flare_mana_used_when_auto_paying` | CR 605.1b: sofort ohne Stapel nach Mana-Fähigkeiten mit {T} (auch beim automatischen Bezahlen), ValidCard, Activator, Bedingungen | trigger/TriggerTapsForMana.java |
| TurnFaceUp | 🟡 teilweise | `morph::cast_face_down_then_turn_up_with_trigger` | ValidCard | trigger/TriggerTurnFaceUp.java |
| Untaps | ✅ ja | `mechanics_batch1::untaps_trigger_and_state_trigger` | über GameState::untap (Enttappsegment, Effekte, Kosten) | trigger/TriggerUntaps.java |
| VisitAttraction | ⚪ n/a |  | Unfinity | trigger/TriggerVisitAttraction.java |

Offen (107): AbilityResolves, AbilityTriggered, Adapt, Airbend, Attached, AttackerBlockedOnce, AttackerUnblockedOnce, BecomeMonarch, BecomeMonstrous, BecomeRenowned, BecomesCrewed, BecomesPlotted, BecomesSaddled, BecomesTargetOnce, BlockersDeclared, CaseSolved, Championed, ChangesController, ClaimPrize, Clashed, ClassLevelGained, CommitCrime, Connives, CollectEvidence, CounterPlayerAddedAll, CounterTypeAddedAll, CounterAddedAll, Countered, CounterRemoved, CounterRemovedOnce, Crewed, DamageAll, DamageDoneOnceByController, DamagePreventedOnce, Destroyed, Devoured, DiscardedAll, Discover, DungeonCompleted, Earthbend, Evolved, ExcessDamage, ExcessDamageAll, ElementalBend, Enlisted, Exerted, Exiled, Exploited, Explores, FacesDilemma, Fight, FightOnce, Firebend, FlippedCoin, FlippedCoinOnce, Forage, Foretell, FullyUnlock, GiveGift, Immediate, Investigated, LifeLost, LifeLostAll, LosesGame, ManaAdded, ManaExpend, ManifestDread, Mentored, Milled, MilledOnce, MilledAll, Mutates, NewGame, PayCumulativeUpkeep, PayEcho, PayLife, PhaseIn, PhaseOut, PhaseOutAll, Proliferate, RingTemptsYou, RolledDie, RolledDieOnce, RoomEntered, Saddled, SacrificedOnce, Scry, SearchedLibrary, Shuffled, Specializes, SpellAbilityCast, SpellAbilityCopy, SpellCopy, Stationed, Surveil, TakesInitiative, TapAll, TokenCreated, TokenCreatedOnce, Trains, Transformed, TurnBegin, Unattached, UnlockDoor, UntapAll, Vote, Waterbend

## Replacement Effect

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| AddCounter | 🟡 teilweise | `mechanics_batch3::hardened_scales_and_doubling_season` | ReplaceCounter mit ReplaceCount$CounterNum-Ausdruck, ValidCard/ValidPlayer, ValidCounterType, EffectOnly | replacement/ReplaceAddCounter.java |
| AssembleContraption | ⚪ n/a |  | Unstable | replacement/ReplaceAssembleContraption.java |
| Counter | 🟡 teilweise | `cant_counter::self_uncounterable_spell_resolves, cant_counter::static_protects_only_own_creature_spells` | Layer CantHappen: „kann nicht neutralisiert werden“ (ValidCard, ValidSA Spell[.Filter], ActiveZones, Bedingungen) | replacement/ReplaceCounter.java |
| CreateToken | 🟡 teilweise | `mechanics_batch3::hardened_scales_and_doubling_season` | ReplaceToken Type Amount (Verdopplung), ValidToken, ValidPlayer, EffectOnly | replacement/ReplaceToken.java |
| DamageDone | 🟡 teilweise | `replacements::damage_doubling_replacement, replacements::prevent_combat_damage_to_self_but_not_noncombat, replacements::prevention_shield_next_n_damage_this_turn_after_doubling` | Prevent, Mengenänderung (ReplaceCount$DamageAmount), ValidTarget/ValidSource/IsCombat | replacement/ReplaceDamage.java |
| Moved | 🟡 teilweise | `replacements::rest_in_peace_redirects_to_exile_and_no_dies_trigger` | Umleitung (ChangeZone ReplacedCard), getappt ins Spiel inkl. Bedingung, etbCounter | replacement/ReplaceMoved.java |
| PlanarDiceResult | ⚪ n/a |  | Planechase | replacement/ReplacePlanarDiceResult.java |
| Planeswalk | ⚪ n/a |  | Planechase | replacement/ReplacePlaneswalk.java |
| RollPlanarDice | ⚪ n/a |  | Planechase | replacement/ReplaceRollPlanarDice.java |
| SetInMotion | ⚪ n/a |  | Archenemy | replacement/ReplaceSetInMotion.java |
| Untap | 🟡 teilweise | `turn_structure::doesnt_untap_during_your_untap_step` | „enttappt nicht während deines Enttappsegments“ (ValidStepTurnToController$ You, Layer$ CantHappen, Bedingungen, auch aus Effektobjekten) | replacement/ReplaceUntap.java |

Offen (29): AssignDealDamage, Attached, BeginPhase, BeginTurn, Cascade, Connive, CopySpell, DealtDamage, DeclareBlocker, Destroy, Draw, DrawCards, Explore, GainLife, GameLoss, GameWin, Learn, LifeReduced, LoseMana, Mill, PayLife, ProduceMana, Proliferate, RemoveCounter, RollDice, Scry, Tap, Transform, TurnFaceUp

## Static Ability

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Continuous | 🟡 teilweise | `layers::affected_set_determined_after_type_change, spells::may_play_exiled_card_this_turn_as_land_drop, spells::may_play_without_mana_cost_with_limit, player_statics::no_maximum_hand_size, player_statics::opponent_hand_size_reduced_but_not_own, player_statics::player_hexproof_only_against_opponents` | Affected/AffectedDefined Self, Zonen (inkl. All), AddType, SetColor, AddKeyword, Set/AddPower/Toughness inkl. Count$-Ausdrücke, CDAs (7a), Bedingungen (IsPresent, CheckSVar, Condition) | staticability/StaticAbility.java |
| AlternativeCost | 🟡 teilweise | `spells::alternative_cost_instead_of_mana_cost` | ValidSA$ Spell.Self mit Kosten statt Manakosten (CR 118.9), Bedingungen, Zusatzkosten gelten weiter | staticability/StaticAbilityAlternativeCost.java |
| CantBeCast | 🟡 teilweise | `cant_play::opponents_cant_cast_spells, cant_play::only_one_spell_each_turn` | ValidCard, Caster, Origin, EffectZone, Bedingungen, NumLimitEachTurn, OnlySorcerySpeed | staticability/StaticAbilityCantBeCast.java |
| CantBeActivated | 🟡 teilweise | `cant_play::opponents_cant_activate_non_mana_abilities` | ValidCard, Activator, AffectedZone, ValidSA Activated[.!ManaAbility/.Loyalty], Bedingungen, auch Mana-Fähigkeiten beim Bezahlen | staticability/StaticAbilityCantBeCast.java |
| Panharmonicon | 🟡 teilweise | `mechanics_batch2::panharmonicon_doubles_etb_triggers` | CR 603.2d: ValidMode, ValidCard, ValidCause, Destination | staticability/StaticAbilityPanharmonicon.java |
| CantAttack | 🟡 teilweise | `combat_restrictions::cant_attack` | ValidCard, Bedingungen | staticability/StaticAbilityCantAttackBlock.java |
| CanAttackDefender | ✅ ja | `mechanics_batch1::defender_may_attack_with_permission` | CR 702.3b: darf trotz Defender angreifen (ValidCard, Bedingungen) | staticability/StaticAbilityCantAttackBlock.java |
| CantBlock | ✅ ja | `combat_restrictions::cant_block` |  | staticability/StaticAbilityCantAttackBlock.java |
| CantBlockBy | 🟡 teilweise | `combat_restrictions::cant_be_blocked_by_small_creatures_via_effect` | ValidAttacker, ValidBlocker, Bedingungen | staticability/StaticAbilityCantAttackBlock.java |
| MinMaxBlocker | ✅ ja | `debuff_repeat::max_one_blocker` | CR 509.1b/c: Mindest-/Höchstzahl an Blockern je Angreifer | staticability/StaticAbilityCantAttackBlock.java |
| MustAttack | 🟡 teilweise | `combat_restrictions::must_attack_if_able` | ValidCreature (CR 508.1d) | staticability/StaticAbilityMustAttack.java |
| CantTarget | 🟡 teilweise | `mechanics_batch2::cant_target_my_creatures_with_opponent_spells` | ValidTarget (Karten/Spieler), ValidSA Spell/Activated, Activator, ValidSource, AffectedZone | staticability/StaticAbilityCantTarget.java |
| CastWithFlash | 🟡 teilweise | `cast_with_flash::creature_with_flash_permission` | ValidCard, Caster, EffectZone (auch All/Hand), Bedingungen, ValidSA Spell | staticability/StaticAbilityCastWithFlash.java |

Offen (77): CantAttackUnless, CantBlockUnless, OptionalAttackCost, OptionalCost, CantPlayLand, DisableTriggers, MustTarget, CanAttackIfHaste, CanBlockIfReach, CanBlockIfShadow, BlockTapped, AttackVigilance, PlayerMustAttack, MustBlock, AssignCombatDamageAsUnblocked, CombatDamageToughness, CombatDamageNegatePower, ColorlessDamageSource, NoCleanupDamage, BlockRestrict, CantGainLife, CantLoseLife, CantChangeLife, CantPayLife, RaiseCost, ReduceCost, SetCost, IgnoreHexproof, IgnoreShroud, AttackRestrict, AssignNoCombatDamage, CanAdapt, CantBeCopied, CantBeBeamedUp, CantBeSuspected, CantBecomeMonarch, CantGainControl, CantAttach, CantCrew, CantDraw, DrawFromBottom, CantDiscard, CantExile, CantPhaseIn, CantPhaseOut, CantPreventDamage, CantPutCounter, CantRegenerate, CantSacrifice, CantTransform, CantVenture, CantChangeDayTime, ActivateAbilityAsIfHaste, IgnoreLandwalk, IgnoreLegendRule, IgnorePlaneswalkerZeroLoyaltyRule, MaxCounter, InfectDamage, WitherDamage, FlipCoinMod, FlipCoinDoubler, PlotZone, NumLoyaltyAct, Activations, Devotion, GainLifeRadiation, SurveilNum, TapPowerValue, UnspentMana, ManaBurn, ManaConvert, UntapOtherPlayer, TurnReversed, PhaseReversed, AttackRequirement, CountersRemain, ManaRestriction

## Effect (ApiType)

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Abandon | ⚪ n/a |  | Archenemy | ability/effects/AbandonEffect.java |
| AddTurn | 🟡 teilweise | `mechanics_batch1::extra_turn_twiddle_random_discard_mill_memory` | CR 500.7: Zusatzzüge (zuletzt erzeugter zuerst) | ability/effects/AddTurnEffect.java |
| AdvanceCrank | ⚪ n/a |  | Unstable | ability/effects/AdvanceCrankEffect.java |
| AlterAttribute | 🟡 teilweise | `mechanics_batch4::suspected_has_menace_and_cant_block` | Suspected (CR 701.60: Bedrohlich, kann nicht blocken), Solved (CR 719.3), Activate False | ability/effects/AlterAttributeEffect.java |
| Amass | ✅ ja | `amass::amass_creates_army_then_grows_and_gains_type` | CR 701.47: ohne Army zuerst 0/0-Army-Spielstein, +1/+1-Marken auf eine gewählte Army, Typ zusätzlich | ability/effects/AmassEffect.java |
| Animate | 🟡 teilweise | `library_regen_animate::animate_land_until_end_of_turn` | Power/Toughness, Types, RemoveCreatureTypes, Keywords, Colors/OverwriteColors, Duration Permanent | ability/effects/AnimateEffect.java |
| AnimateAll | 🟡 teilweise | `mass_effects::animate_all_tap_all_untap_all` | ValidCards, Typen/P/T/Keywords/Farben wie Animate, Zielspieler/Defined | ability/effects/AnimateAllEffect.java |
| Attach | 🟡 teilweise | `auras_equipment::equip_sorcery_speed_own_creature_and_moves, attach_reveal::equipment_attaches_to_entering_creature` | Object (Standard Self) an Ziel anlegen | ability/effects/AttachEffect.java |
| AssembleContraption | ⚪ n/a |  | Unstable | ability/effects/AssembleContraptionEffect.java |
| BecomeMonarch | 🟡 teilweise | `monarch::monarch_draws_at_end_step_and_static_condition, monarch::combat_damage_to_monarch_steals_crown` | CR 724: ein Monarch, Ziehen im Endsegment, Übergang durch Kampfschaden, Weitergabe beim Ausscheiden, Condition Monarch/CheckDefinedPlayer You.isMonarch, Count$Monarch | ability/effects/BecomeMonarchEffect.java |
| Branch | 🟡 teilweise | `choose_player_branch::branch_picks_sub_ability_by_condition` | BranchConditionSVar/Compare, True-/FalseSubAbility ohne eigene Ziele (erben die Ziele) | ability/effects/BranchEffect.java |
| ChangeZone | 🟡 teilweise | `change_zone::search_library_tapped_then_shuffle, until_leaves::banisher_priest_exiles_until_it_leaves, change_zone_ext::shuffle_into_library, change_zone_ext::optional_move_can_be_declined, change_zone_ext::search_graveyard_and_exile, more_triggers::exile_from_opponents_hand_chosen_by_caster` | Ziele in beliebigen Zonen, Suche (verdeckt: darf nicht finden), Tapped, GainControl, LibraryPosition 0/-1 | ability/effects/ChangeZoneEffect.java |
| ChangeZoneAll | 🟡 teilweise | `change_zone::change_zone_all_bounces_everything` | ChangeType + Origin/Destination, gleichzeitig (Rückblick für Trigger) | ability/effects/ChangeZoneAllEffect.java |
| ChaosEnsues | ⚪ n/a |  | Planechase | ability/effects/ChaosEnsuesEffect.java |
| Charm | 🟡 teilweise | `tokens_charm::charm_choose_two_resolve_in_printed_order` | CharmNum/MinCharmNum, CanRepeatModes, nur Modi mit legalen Zielen, Druckreihenfolge, bei Triggern beim Stapeln | ability/effects/CharmEffect.java |
| ChooseCard | 🟡 teilweise | `effects_objects::choose_card_then_destroy_all_non_chosen` | Choices, ChoiceZone, DefinedCards, Amount, MinAmount, Mandatory, ControlledByPlayer Chooser, TargetControls, AtRandom, RememberChosen, ForgetChosen | ability/effects/ChooseCardEffect.java |
| ChooseColor | 🟡 teilweise | `effects_objects::choose_color_as_it_enters_for_mana_and_filter` | eine Farbe | ability/effects/ChooseColorEffect.java |
| ChooseNumber | 🟡 teilweise | `coin_number_goad::chosen_number_is_used` | Min/Max, Random, Count$ChosenNumber | ability/effects/ChooseNumberEffect.java |
| ChoosePlayer | 🟡 teilweise | `choose_player_branch::chosen_opponent_loses_life_and_creatures` | Defined-Wähler, Choices-Spielerfilter, Random, RememberChosen | ability/effects/ChoosePlayerEffect.java |
| ChooseSector | ⚪ n/a |  | Unstable | ability/effects/ChooseSectorEffect.java |
| ChooseSource | 🟡 teilweise | `choose_source::circle_prevents_next_damage_from_chosen_red_source_once` | Quellen auf dem Battlefield, dem Stapel und in der Befehlszone, Choices-Filter (inkl. <Farbe>Source, Emblem), RememberChosen | ability/effects/ChooseSourceEffect.java |
| ChooseType | 🟡 teilweise | `effects_objects::choose_creature_type_as_it_enters` | Type Creature/Basic/Land/Card, InvalidTypes, ValidTypes | ability/effects/ChooseTypeEffect.java |
| Cleanup | 🟡 teilweise | `sacrifice_unless::condition_defined_remembered_and_cleanup` | ClearRemembered (übrige Clear…-Flags ohne Wirkung, da der Zustand nicht existiert) | ability/effects/CleanUpEffect.java |
| Clone | 🟡 teilweise | `copies::clone_enters_as_copy_with_etb_and_reverts_when_leaving` | kopierbare Werte (CR 707.2/707.3) mit Änderungen in Layer 1 (AddTypes, AddKeywords, Set P/T, NonLegendary, SetColor, SetCreatureTypes) | ability/effects/CloneEffect.java |
| Connive | ✅ ja | `mechanics_batch3::multiply_and_connive` | CR 701.50: ziehen, abwerfen, je Nichtland +1/+1-Marke | ability/effects/ConniveEffect.java |
| CopyPermanent | 🟡 teilweise | `copies::token_copy_with_haste_sacrificed_at_end_step` | Token-Kopien mit Änderungen, NumCopies, Controller, TokenTapped, RememberTokens, AtEOT (Sacrifice/Exile/Hand/Destroy als verzögerter Trigger) | ability/effects/CopyPermanentEffect.java |
| CopySpellAbility | 🟡 teilweise | `copy_spell::copy_target_spell_with_new_target, copy_spell::triggered_copy_of_cast_spell` | CR 707.10: Kopie von Ziel-Zauber oder auslösendem Zauber (TriggeredSpellAbility), Amount, Controller, MayChooseTarget (CR 707.10c), Kopie nicht gewirkt, hört außerhalb des Stapels auf zu existieren | ability/effects/CopySpellAbilityEffect.java |
| Counter | 🟡 teilweise | `mass_discard_counter::mana_leak_pay_or_countered` | Zauber neutralisieren, allgemeines UnlessCost | ability/effects/CounterEffect.java |
| DamageAll | 🟡 teilweise | `mass_discard_counter::damage_all_creatures_and_players` | ValidCards/ValidPlayers | ability/effects/DamageAllEffect.java |
| DealDamage | 🟡 teilweise | `spells::bolt_face_and_creature_and_fizzle, damage_source::bite_uses_creature_as_source_with_its_keywords, divided::divided_damage_between_two_targets` | Grundform | ability/effects/DamageDealEffect.java |
| Debuff | ✅ ja | `debuff_repeat::debuff_removes_flying_until_end_of_turn` | Keywords bis Zugende verlieren (Layer 6 nach Zeitstempel) | ability/effects/DebuffEffect.java |
| DelayedTrigger | 🟡 teilweise | `delayed_triggers::flicker_returns_at_next_end_step_following_the_card, delayed_triggers::delayed_trigger_fires_once_at_your_next_upkeep, delayed_triggers::this_turn_delayed_trigger_expires_at_cleanup` | alle Trigger-Modi, einmaliges Auslösen (CR 603.7c), ThisTurn, NextTurn, RememberObjects + DelayTriggerRemembered[LKI] | ability/effects/DelayedTriggerEffect.java |
| Destroy | 🟡 teilweise | `spells::destroy_respects_indestructible` | Grundform | ability/effects/DestroyEffect.java |
| DestroyAll | 🟡 teilweise | `mass_discard_counter::wrath_destroys_all_simultaneously_with_dies_triggers` | gleichzeitig mit Rückblick | ability/effects/DestroyAllEffect.java |
| Dig | 🟡 teilweise | `library_regen_animate::dig_one_to_hand_rest_bottom` | DigNum, ChangeNum (auch All), ChangeValid, Destination(2), LibraryPosition(2), Optional | ability/effects/DigEffect.java |
| DigUntil | 🟡 teilweise | `dig_until::reveal_until_creature_onto_battlefield_rest_bottom, dig_until::mill_until_land_sequential_to_graveyard, dig_until::until_two_found_remembers_and_exiles_rest` | Valid, Amount, MaxRevealed, Found-/RevealedDestination mit Bibliothek oben/unten, gleiche Zielzone der Reihe nach (Java sequential), RevealRandomOrder, Shuffle, Optional, RememberFound/Revealed, Tapped, GainControl, NoMoveRevealed | ability/effects/DigUntilEffect.java |
| Discard | 🟡 teilweise | `mass_discard_counter::duress_caster_chooses_noncreature_nonland, triggers::immediate_trigger_only_if_done_and_amount_times` | TgtChoose, Hand, Random, RevealYouChoose, DiscardValid, RememberDiscarded | ability/effects/DiscardEffect.java |
| Draft | ⚪ n/a |  | Conspiracy-Draft | ability/effects/DraftEffect.java |
| Draw | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/DrawEffect.java |
| Effect | 🟡 teilweise | `effects_objects::effect_grants_hexproof_to_remembered_until_end_of_turn` | Effektobjekt in Command Zone mit Statics/Triggern/Ersatzeffekten, RememberObjects (Targeted/Self/Triggered…), Duration (Zugende, Permanent, UntilYourNextTurn, UntilTheEndOfYourNextTurn) | ability/effects/EffectEffect.java |
| Explore | ✅ ja | `mechanics_batch2::explore_land_to_hand_or_counter` | CR 701.44: Land auf die Hand, sonst +1/+1-Marke und Karte darf in den Friedhof | ability/effects/ExploreEffect.java |
| Fight | 🟡 teilweise | `mass_discard_counter::fight_uses_parent_target_and_put_counter_all` | Kämpfer aus Zielen/Defined wie Java, CR 701.14b | ability/effects/FightEffect.java |
| FlipCoin | 🟡 teilweise | `coin_number_goad::coin_flip_win_or_lose` | CR 705: Win/Lose bzw. Heads/Tails (NoCall), Event CoinFlipped | ability/effects/FlipCoinEffect.java |
| Fog | ✅ ja | `mass_effects::fog_prevents_combat_damage_only` | CR 615: aller Kampfschaden dieses Zuges verhindert, anderer Schaden nicht | ability/effects/FogEffect.java |
| GainControl | 🟡 teilweise | `control::threaten_until_end_of_turn_untaps_and_grants_haste, control::permanent_control_change, control::control_ends_when_source_leaves_battlefield` | Kontrolleffekte nach Zeitstempel (Layer 2), LoseControl EOT/UntilTheEndOfYourNextTurn/LeavesPlay/LoseControl, Untap, AddKWs, NewController | ability/effects/ControlGainEffect.java |
| GainLife | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/LifeGainEffect.java |
| GenericChoice | 🟡 teilweise | `generic_choice::opponent_chooses_draw_for_caster, generic_choice::opponent_chooses_to_lose_life_via_remembered_chooser` | Defined-Spieler wählen je eine Unterfähigkeit (Choices ohne Ziele), AtRandom, TempRemember Chooser | ability/effects/ChooseGenericEffect.java |
| Goad | 🟡 teilweise | `coin_number_goad::goaded_creature_must_attack, coin_number_goad::goaded_prefers_other_player_in_multiplayer` | CR 701.38: muss angreifen, möglichst nicht den Goader, endet mit dessen nächstem Zug, Duration Permanent | ability/effects/GoadEffect.java |
| Heist | ⚪ n/a |  | Alchemy | ability/effects/HeistEffect.java |
| Investigate | 🟡 teilweise | `mass_effects::investigate_creates_clue_that_draws` | Clue-Token (CR 701.36), Num, Defined | ability/effects/InvestigateEffect.java |
| Intensify | ⚪ n/a |  | Alchemy | ability/effects/IntensifyEffect.java |
| ImmediateTrigger | 🟡 teilweise | `triggers::immediate_trigger_only_if_done_and_amount_times` | reflexiver Trigger (CR 603.12) mit Execute, Bedingungen, RememberObjects (DelayTriggerRemembered), TriggerAmount | ability/effects/ImmediateTriggerEffect.java |
| LoseLife | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | Grundform | ability/effects/LifeLoseEffect.java |
| LosePerpetual | ⚪ n/a |  | Alchemy | ability/effects/LosePerpetualEffect.java |
| Mana | 🟡 teilweise | `spells::manual_mana_payment_and_cancel` |  | ability/effects/ManaEffect.java |
| Mill | 🟡 teilweise | `library_regen_animate::mill_target_player, mechanics_batch1::extra_turn_twiddle_random_discard_mill_memory` | NumCards, Defined/Ziel | ability/effects/MillEffect.java |
| MultiplyCounter | ✅ ja | `mechanics_batch3::multiply_and_connive` | Marken einer oder aller Arten vervielfachen (Multiplier, Standard 2) | ability/effects/CountersMultiplyEffect.java |
| NameCard | 🟡 teilweise | `name_card::named_card_abilities_are_blocked` | Defined, ValidCards, AtRandom, Filter NamedCard, Cleanup ClearNamedCard | ability/effects/ChooseCardNameEffect.java |
| OpenAttraction | ⚪ n/a |  | Unfinity | ability/effects/OpenAttractionEffect.java |
| PeekAndReveal | 🟡 teilweise | `change_zone::peek_and_reveal_remembers_only_matching_revealed_cards` | PeekAmount, RevealValid, NoReveal, RevealOptional (Decision::Confirm), RememberRevealed, RememberPeeked | ability/effects/PeekAndRevealEffect.java |
| Planeswalk | ⚪ n/a |  | Planechase | ability/effects/PlaneswalkEffect.java |
| Play | 🟡 teilweise | `play_effect::cast_spell_from_hand_without_mana_cost, play_effect::recast_from_graveyard_is_exiled_afterwards, play_effect::paid_play_can_be_cancelled_and_lands_use_land_drop, play_effect::play_land_by_effect_counts_as_land_drop` | CR 608.2g: Wirken während der Auflösung ohne Zeitbeschränkung, Defined/Ziel oder Valid+ValidZone, ValidSA, Amount (auch All), Optional, WithoutManaCost oder normal bezahlt (Abbruch nimmt zurück), Länder mit freiem Landspiel, Controller, ReplaceGraveyard Exile, RememberPlayed | ability/effects/PlayEffect.java |
| PreventDamage | 🟡 teilweise | `replacements::prevention_shield_next_n_damage_this_turn_after_doubling` | nächste N Schaden in diesem Zug (Karten, Spieler) | ability/effects/DamagePreventEffect.java |
| Proliferate | 🟡 teilweise | `mass_effects::proliferate_adds_one_of_each_kind_to_chosen` | CR 701.27: beliebige bleibende Karten mit Marken und Spieler mit Giftmarken, je Art +1, Amount | ability/effects/CountersProliferateEffect.java |
| Protection | 🟡 teilweise | `protection::protection_effect_with_chosen_color` | Gains feste Farben/Filter oder Choice AnyColor, bis Zugende | ability/effects/ProtectEffect.java |
| Pump | 🟡 teilweise | `spells::draw_gain_lose_life_and_pump` | NumAtt/NumDef/KW bis Zugende | ability/effects/PumpEffect.java |
| PumpAll | 🟡 teilweise | `mass_discard_counter::pump_all_own_creatures_until_end_of_turn` | ValidCards, NumAtt/NumDef, KW bis Zugende | ability/effects/PumpAllEffect.java |
| PutCounter | 🟡 teilweise | `sba::cr_704_5q_plus_and_minus_counters_annihilate, counters_keywords::adapt_only_without_p1p1_counters, counters_keywords::monstrosity_once_and_filter, counters_keywords::bolster_picks_least_toughness_with_choice_on_tie, counters_keywords::support_up_to_n_targets_one_counter_each, divided::single_target_gets_everything_and_counters_split` | Grundform | ability/effects/CountersPutEffect.java |
| PutCounterAll | 🟡 teilweise | `mass_discard_counter::fight_uses_parent_target_and_put_counter_all` | ValidCards, CounterType, CounterNum | ability/effects/CountersPutAllEffect.java |
| PutSticker | ⚪ n/a |  | Unfinity | ability/effects/PutStickerEffect.java |
| Regenerate | ✅ ja | `library_regen_animate::regenerate_replaces_destruction` | inkl. SBA-Zerstörung, NoRegen bei Destroy/DestroyAll | ability/effects/RegenerateEffect.java |
| RemoveCounter | 🟡 teilweise | `counters_keywords::counters_filter_grants_flying_and_remove_counter_all, counters_keywords::remove_counter_any_chooses_kind_per_counter, counters_keywords::remove_all_counters_needs_has_counters_target` | CounterType fest/Any/All, CounterNum Zahl/Ausdruck/All, UpTo, Optional, Ziel/Defined | ability/effects/CountersRemoveEffect.java |
| Repeat | 🟡 teilweise | `debuff_repeat::repeat_until_condition_fails` | RepeatSubAbility (ohne eigene Ziele), MaxRepeat, RepeatPresent/Defined/Compare, RepeatCheckSVar/Compare, RepeatOptional | ability/effects/RepeatEffect.java |
| RepeatEach | 🟡 teilweise | `expressions::repeat_each_player_with_remembered_player, expressions::repeat_each_card_remembers_each_card_in_turn` | RepeatPlayers, RepeatCards (+Zone), DefinedCards, StartingWith You, ClearRememberedBeforeLoop, DamageMap | ability/effects/RepeatEachEffect.java |
| Reveal | 🟡 teilweise | `attach_reveal::reveal_creature_from_hand_and_use_it` | Handkarten aufdecken: RevealValid/RevealAllValid, RevealDefined, NumCards, AnyNumber, Random, Optional, RememberRevealed | ability/effects/RevealEffect.java |
| RevealHand | 🟡 teilweise |  | Hand aufdecken/ansehen (Event Revealed), RememberRevealed | ability/effects/RevealHandEffect.java |
| RollDice | 🟡 teilweise | `roll_dice::kitten_gains_life_equal_to_roll, roll_dice::d20_table_picks_range` | CR 706: Sides, Amount, Modifier, ResultSubAbilities (Bereiche, Else), ResultSVar als Laufzeitwert, Event DiceRolled | ability/effects/RollDiceEffect.java |
| RollPlanarDice | ⚪ n/a |  | Planechase | ability/effects/RollPlanarDiceEffect.java |
| RunChaos | ⚪ n/a |  | Planechase | ability/effects/RunChaosEffect.java |
| Sacrifice | 🟡 teilweise | `sacrifice_unless::edict_target_player_chooses_and_indestructible_can_be_sacrificed, sacrifice_unless::each_player_sacrifices_simultaneously_dies_triggers_see_all, sacrifice_unless::optional_up_to_amount_and_remembered, sacrifice_unless::strict_amount_sacrifices_nothing_if_not_enough, copies::token_copy_with_haste_sacrificed_at_end_step` | SacValid, Defined (Spieler oder Karten), Amount, Optional, StrictAmount, Destroy, RememberSacrificed, gleichzeitig (CR 101.4) | ability/effects/SacrificeEffect.java |
| SacrificeAll | 🟡 teilweise | `mass_effects::sacrifice_all_hits_every_player_simultaneously` | ValidCards oder Defined, Controller (You/TriggeredPlayer), RememberSacrificed, gleichzeitig mit LKI | ability/effects/SacrificeAllEffect.java |
| Scry | ✅ ja | `library_regen_animate::scry_bottom_and_reorder_then_draw` |  | ability/effects/ScryEffect.java |
| Seek | ⚪ n/a |  | Alchemy | ability/effects/SeekEffect.java |
| SetInMotion | ⚪ n/a |  | Archenemy | ability/effects/SetInMotionEffect.java |
| SetState | 🟡 teilweise | `multiface::transform_keeps_object_damage_and_counters` | Mode$ Transform (CR 701.28/712.18: gleiches Objekt, neuer Zeitstempel), Optional, ChangeZone Transformed$ | ability/effects/SetStateEffect.java |
| StoreSVar | 🟡 teilweise | `mechanics_batch2::store_svar_and_remember_tokens` | Number/Calculate/Count/CountSVar in Laufzeitvariable der Quelle (Startwert 0) | ability/effects/StoreSVarEffect.java |
| Surveil | 🟡 teilweise | `library_regen_animate::surveil_puts_chosen_cards_into_graveyard` | Amount, Defined | ability/effects/SurveilEffect.java |
| Tap | 🟡 teilweise |  | Grundform | ability/effects/TapEffect.java |
| TapAll | 🟡 teilweise | `mass_effects::animate_all_tap_all_untap_all` | ValidCards, Zielspieler/Defined | ability/effects/TapAllEffect.java |
| TapOrUntap | ✅ ja | `mechanics_batch1::extra_turn_twiddle_random_discard_mill_memory` | Wahl tappen/enttappen | ability/effects/TapOrUntapEffect.java |
| Token | 🟡 teilweise | `tokens_charm::tokens_enter_and_trigger_etb, mechanics_batch2::store_svar_and_remember_tokens` | TokenScript (Liste), TokenAmount, TokenOwner, TokenTapped, TokenPower/Toughness | ability/effects/TokenEffect.java |
| Untap | 🟡 teilweise |  | Grundform | ability/effects/UntapEffect.java |
| UntapAll | 🟡 teilweise | `mass_effects::animate_all_tap_all_untap_all` | ValidCards, Zielspieler/Defined | ability/effects/UntapAllEffect.java |

Offen (111): ActivateAbility, AddOrRemoveCounter, AddPhase, Airbend, Assimilate, AssignGroup, Balance, BecomesBlocked, BidLife, Blight, Block, Bond, Camouflage, ChangeCombatants, ChangeSpeed, ChangeTargets, ChangeText, ChangeX, ChooseDirection, ChooseEvenOdd, ClaimThePrize, Clash, ClassLevelUp, Cloak, ControlSpell, ControlPlayer, Detain, DayTime, DigMultiple, Discover, DrainMana, EachDamage, Earthbend, Empower, Encode, EndCombatPhase, EndTurn, Endure, ExchangeLife, ExchangeLifeVariant, ExchangeControl, ExchangeControlVariant, ExchangePower, ExchangeZone, ExchangeTextBox, FlipOntoBattlefield, GainControlVariant, GainOwnership, GameDrawn, Haunt, HealDamage, Incubate, Learn, LookAt, LosesGame, MakeCard, ManaReflected, Manifest, ManifestDread, Meld, MoveCounter, MultiplePiles, MustBlock, Mutate, PermanentCreature, PermanentNoncreature, Phases, PlayLandVariant, Poison, ProtectionAll, Radiation, Recruit, RearrangeTopOfLibrary, Regeneration, RemoveCounterAll, RemoveFromCombat, RemoveFromGame, RemoveFromMatch, ReorderZone, ReplaceCounter, ReplaceEffect, ReplaceMana, ReplaceDamage, ReplaceToken, ReplaceSplitDamage, RestartGame, ReverseTurnOrder, RingTemptsYou, SetLife, Shuffle, SkipPhase, SkipTurn, Subgame, SwitchBlock, TakeInitiative, TapOrUntapAll, TimeTravel, TwoPiles, Unattach, UnlockDoor, Venture, VillainousChoice, Vote, WinsGame, BlankLine, DamageResolve, ChangeZoneResolve, CompanionChoose, InternalLegendaryRule, InternalIgnoreEffect, InternalRadiation

## Cost

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Mana cost | ✅ ja | `spells::x_spell` | inkl. Hybrid, Phyrexian, {2/W}, X | cost/CostPartMana.java |
| Add mana | 🟡 teilweise | `spells::manual_mana_payment_and_cancel, spells::mana_ability_with_variable_amount` | Mana-Fähigkeiten mit Produced$ (fest, Any, Combo Farbe, ChosenColor), Amount$ als Ausdruck | cost/CostAddMana.java |
| {T} | ✅ ja | `spells::activated_ability_with_tap_cost_uses_stack_and_sickness` |  | cost/CostTap.java |
| {Q} | ✅ ja | `spells::untap_symbol_cost` |  | cost/CostUntap.java |
| tapXType | ✅ ja | `sacrifice_unless::exile_tap_and_return_cost_parts` | n ungetappte eigene Permanents nach Typ (ohne Einsatzsperre, CR 302.6) | cost/CostTapType.java |
| Sac | ✅ ja | `spells::sacrifice_self_cost, sacrifice_unless::spell_additional_sacrifice_cost_is_required_and_remembered` | Sac<1/CARDNAME> und Sac<n/Typ>, auch als Zusatzkosten von Zaubern (CR 601.2f/h) | cost/CostSacrifice.java |
| Discard | 🟡 teilweise |  | Discard<n/Typ> | cost/CostDiscard.java |
| PayLife | ✅ ja | `spells::phyrexian_mana_with_life` |  | cost/CostPayLife.java |
| PayShards | ⚪ n/a |  | Adventure-Modus | cost/CostPayShards.java |
| Exile | 🟡 teilweise | `keywords_misc::exile_from_graveyard_cost_activates_from_graveyard, sacrifice_unless::exile_tap_and_return_cost_parts, sacrifice_unless::exile_from_hand_cost_and_shuffle` | ExileFromGrave<1/CARDNAME>, ExileFromGrave<n/Typ>, Exile<1/CARDNAME>, ExileFromHand<1/CARDNAME>, ExileFromHand<n/Typ> | cost/CostExile.java |
| Return | ✅ ja | `sacrifice_unless::exile_tap_and_return_cost_parts` | Return<1/CARDNAME> und Return<n/Typ> (eigene bleibende Karten auf die Hand) | cost/CostReturn.java |
| AddCounter | 🟡 teilweise | `planeswalkers::enters_with_loyalty_and_activates_once_per_turn_at_sorcery_speed` | Marken auf die Quelle (n fest) | cost/CostPutCounter.java |
| SubCounter | 🟡 teilweise | `planeswalkers::minus_ability_to_zero_loyalty_goes_to_graveyard` | Marken von der Quelle (n fest) | cost/CostRemoveCounter.java |

Offen (30): untapXType, Draw, DamageYou, GainLife, PayEnergy, ExileFromStack, ExiledMoveToGrave, Mill, PutCardToLib, Reveal, RevealChosen, AddCounterYou, RemoveAnyCounter, GainControl, Unattach, Exert, Enlist, FlipCoin, RollDice, ChooseColor, ChooseCreatureType, CollectEvidence, Forage, PromiseGift, Behold, BeholdExile, Blight, Teamwork, Waterbend, Cost adjustment

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
| CR 704.5s | ✅ ja | `sagas::saga_chapters_trigger_and_saga_is_sacrificed_after_last` | Saga mit Lore-Marken ≥ letztem Kapitel wird geopfert, solange keine Kapitelfähigkeit wartet oder auf dem Stapel liegt (CR 714.4) | GameAction.java (checkStateEffects) |
| CR 704.5u | ⚪ n/a |  | Space Sculptor (Unstable) | GameAction.java (checkStateEffects) |
| CR 704.5v | ✅ ja | `sba::cr_704_5v_battle_without_defense` |  | GameAction.java (checkStateEffects) |
| Contraption | ⚪ n/a |  | Unstable | GameAction.java (checkStateEffects) |
| Scheme | ⚪ n/a |  | Archenemy | GameAction.java (checkStateEffects) |

Offen (7): CR 704.5r, CR 704.5t, CR 704.5w/x, CR 704.5y, CR 704.5z, CR 704.6c, CR 704.6d

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
| Convoke payment | 🟡 teilweise | `cost_helpers::convoke_taps_creatures_for_colored_and_generic_mana` | Zuordnung Kreatur→Symbol automatisch (wenigfarbige zuerst) | mana/ManaCostBeingPaid.java (payManaViaConvoke) |
| Mana refund | ✅ ja | `spells::manual_mana_payment_and_cancel` |  | mana/ManaRefundService.java |

Offen (3): Mana restrictions, Mana conversion, Sunburst

## Zone

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Hand | ✅ ja | `turn_structure::cleanup_discards_to_hand_size_removes_damage_ends_effects` |  | zone/ZoneType.java (Hand) |
| Library | ✅ ja | `change_zone::search_library_tapped_then_shuffle, change_zone::library_bottom` |  | zone/ZoneType.java (Library) |
| Graveyard | 🟡 teilweise | `mass_discard_counter::target_player_chooses_discard` | Reihenfolge im Friedhof ohne eigenen Test | zone/ZoneType.java (Graveyard) |
| Battlefield | ✅ ja | `spells::creature_spell_resolves_onto_battlefield_with_summoning_sickness` |  | zone/ZoneType.java (Battlefield) |
| Exile | ✅ ja | `replacements::rest_in_peace_redirects_to_exile_and_no_dies_trigger` |  | zone/ZoneType.java (Exile) |
| Command | 🟡 teilweise | `effects_objects::effect_trigger_fires_from_command_zone, commander::cast_from_command_zone_with_tax_and_return` | Effektobjekte, Embleme, Commander | zone/ZoneType.java (Command) |
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
| Commander | 🟡 teilweise | `commander::cast_from_command_zone_with_tax_and_return, commander::tax_makes_commander_uncastable_without_enough_mana, commander::declining_return_keeps_commander_in_graveyard, commander::twenty_one_commander_damage_loses, soak::random_commander_multiplayer_games` | CR 903: 40 Leben, Befehlszone, Wirken mit Steuer, Rückkehr in die Befehlszone (auch Hand/Bibliothek als Wahl statt Ersatzeffekt), 21 Commander-Schaden, Filter IsCommander, 4-Spieler-Lasttest | GameType.java (Commander) |
| Oathbreaker | ⚪ n/a |  | ausgeschlossen | GameType.java (Oathbreaker) |
| TinyLeaders | ⚪ n/a |  | kein Zielformat | GameType.java (TinyLeaders) |
| Brawl | ⚪ n/a |  | ausgeschlossen | GameType.java (Brawl) |
| Planeswalker | ⚪ n/a |  | Forge-Variante, ausgeschlossen | GameType.java (Planeswalker) |
| Planechase | ⚪ n/a |  | ausgeschlossen | GameType.java (Planechase) |
| Archenemy | ⚪ n/a |  | ausgeschlossen | GameType.java (Archenemy) |
| ArchenemyRumble | ⚪ n/a |  | ausgeschlossen | GameType.java (ArchenemyRumble) |
| MomirBasic | ⚪ n/a |  | ausgeschlossen | GameType.java (MomirBasic) |
| MoJhoSto | ⚪ n/a |  | ausgeschlossen | GameType.java (MoJhoSto) |

Offen (1): Draft

## Game Rule Option

| Regel | Status | Test | Hinweis | Java-Quelle |
|---|---|---|---|---|
| Mana burn option | ⚪ n/a |  | alte Regel | GameRules.java (hasManaBurn) |
| Order combatants | ⚪ n/a |  | Regel vor Foundations, Zielformate nutzen die aktuelle | GameRules.java (hasOrderCombatants) |
| Poison counters to lose | 🟡 teilweise | `sba::cr_704_5c_ten_poison_loses` | fest 10, nicht konfigurierbar | GameRules.java (getPoisonCountersToLose) |
| Play for ante | ⚪ n/a |  | Ante ausgeschlossen | GameRules.java (useAnte) |
| Ante rarity matching | ⚪ n/a |  | Ante ausgeschlossen | GameRules.java (getMatchAnteRarity, getAnteIncludeBasicLands) |

Offen (2): Games per match, Applied variants
