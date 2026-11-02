//! Testgerüst: kleine Kartenbasis, Spiel direkt in einem Schritt aufsetzen,
//! Entscheidungen beantworten.
#![allow(dead_code)]

use forge_engine::decision::{Action, Choice, Decision, PayAction, Target};
use forge_engine::state::Flow;
use forge_engine::{CardDb, CardId, Engine, GameRules, GameState, PlayerId, Status, Step, Zone};

pub const P1: PlayerId = PlayerId(0);
pub const P2: PlayerId = PlayerId(1);
/// P2 als Angriffsziel.
pub const T2: Target = Target::Player(P2);

pub const SCRIPTS: &[&str] = &[
    "Name:Plains\nManaCost:no cost\nTypes:Basic Land Plains\nOracle:\n",
    "Name:Island\nManaCost:no cost\nTypes:Basic Land Island\nOracle:\n",
    "Name:Swamp\nManaCost:no cost\nTypes:Basic Land Swamp\nOracle:\n",
    "Name:Mountain\nManaCost:no cost\nTypes:Basic Land Mountain\nOracle:\n",
    "Name:Forest\nManaCost:no cost\nTypes:Basic Land Forest\nOracle:\n",
    "Name:Grizzly Bears\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2\nOracle:\n",
    "Name:Hill Giant\nManaCost:3 R\nTypes:Creature Giant\nPT:3/3\nOracle:\n",
    "Name:Serra Angel\nManaCost:3 W W\nTypes:Creature Angel\nPT:4/4\nK:Flying\nK:Vigilance\nOracle:\n",
    "Name:Giant Spider\nManaCost:3 G\nTypes:Creature Spider\nPT:2/4\nK:Reach\nOracle:\n",
    "Name:Goblin Piker\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/1\nOracle:\n",
    "Name:Craw Wurm\nManaCost:4 G G\nTypes:Creature Wurm\nPT:6/4\nK:Trample\nOracle:\n",
    "Name:White Knight\nManaCost:W W\nTypes:Creature Human Knight\nPT:2/2\nK:First Strike\nOracle:\n",
    "Name:Fencing Ace\nManaCost:1 W\nTypes:Creature Human Soldier\nPT:1/1\nK:Double Strike\nOracle:\n",
    "Name:Typhoid Rats\nManaCost:B\nTypes:Creature Rat\nPT:1/1\nK:Deathtouch\nOracle:\n",
    "Name:Vampire Nighthawk\nManaCost:1 B B\nTypes:Creature Vampire Shaman\nPT:2/3\nK:Flying\nK:Deathtouch\nK:Lifelink\nOracle:\n",
    "Name:Raging Goblin\nManaCost:R\nTypes:Creature Goblin Berserker\nPT:1/1\nK:Haste\nOracle:\n",
    "Name:Wall of Stone\nManaCost:1 R R\nTypes:Creature Wall\nPT:0/8\nK:Defender\nOracle:\n",
    "Name:Goblin Heelcutter\nManaCost:3 R\nTypes:Creature Goblin Warrior\nPT:3/2\nK:Menace\nOracle:\n",
    "Name:Darksteel Myr\nManaCost:3\nTypes:Artifact Creature Myr\nPT:0/1\nK:Indestructible\nOracle:\n",
    "Name:Gladecover Scout\nManaCost:G\nTypes:Creature Elf Scout\nPT:1/1\nK:Hexproof\nOracle:\n",
    "Name:Isamaru, Hound of Konda\nManaCost:W\nTypes:Legendary Creature Dog\nPT:2/2\nOracle:\n",
    "Name:Llanowar Elves\nManaCost:G\nTypes:Creature Elf Druid\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\nOracle:\n",
    "Name:Lightning Bolt\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage to any target.\nOracle:\n",
    "Name:Shock\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | SpellDescription$ CARDNAME deals 2 damage to any target.\nOracle:\n",
    "Name:Murder\nManaCost:1 B B\nTypes:Instant\nA:SP$ Destroy | ValidTgts$ Creature | SpellDescription$ Destroy target creature.\nOracle:\n",
    "Name:Giant Growth\nManaCost:G\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | SpellDescription$ Target creature gets +3/+3 until end of turn.\nOracle:\n",
    "Name:Weakness\nManaCost:B\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature | NumAtt$ -2 | NumDef$ -1 | SpellDescription$ x\nOracle:\n",
    "Name:Divination\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 2 | SpellDescription$ Draw two cards.\nOracle:\n",
    "Name:Healing Salve\nManaCost:W\nTypes:Instant\nA:SP$ GainLife | ValidTgts$ Player | LifeAmount$ 3 | SpellDescription$ x\nOracle:\n",
    "Name:Fireball Lite\nManaCost:X R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SpellDescription$ x\nSVar:X:Count$xPaid\nOracle:\n",
    "Name:Gitaxian Probe Lite\nManaCost:UP\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Prodigal Pyromancer\nManaCost:2 R\nTypes:Creature Human Wizard\nPT:1/1\nA:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Nether Void Lite\nManaCost:3 B\nTypes:World Enchantment\nOracle:\n",
    "Name:Concordant Crossroads Lite\nManaCost:G\nTypes:World Enchantment\nOracle:\n",
    "Name:Liliana Lite\nManaCost:1 B B\nTypes:Legendary Planeswalker Liliana\nLoyalty:3\nOracle:\n",
    "Name:Invasion Lite\nManaCost:2 R\nTypes:Battle Siege\nDefense:3\nOracle:\n",
    "Name:Ambush Viper\nManaCost:1 G\nTypes:Creature Snake\nPT:2/1\nK:Flash\nK:Deathtouch\nOracle:\n",
    "Name:Silhana Lite\nManaCost:G\nTypes:Creature Elf\nPT:2/2\nK:Shroud\nOracle:\n",
    "Name:Mogg Fanatic Lite\nManaCost:R\nTypes:Creature Goblin\nPT:1/1\nA:AB$ DealDamage | Cost$ Sac<1/CARDNAME> | ValidTgts$ Any | NumDmg$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Untap Monk Lite\nManaCost:W\nTypes:Creature Human Monk\nPT:1/1\nA:AB$ GainLife | Cost$ Q | LifeAmount$ 2 | SpellDescription$ x\nOracle:\n",
    "Name:Elvish Visionary\nManaCost:1 G\nTypes:Creature Elf Shaman\nPT:1/1\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw | TriggerDescription$ When CARDNAME enters, draw a card.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Salve Bearer Lite\nManaCost:1 W\nTypes:Creature Human\nPT:1/1\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigGain | TriggerDescription$ When CARDNAME dies, you gain 2 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 2\nOracle:\n",
    "Name:Blood Artist Lite\nManaCost:1 B\nTypes:Creature Vampire\nPT:0/1\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ Whenever CARDNAME or another creature dies, target player loses 1 life.\nSVar:TrigDrain:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ 1\nOracle:\n",
    "Name:Phyrexian Arena Lite\nManaCost:1 B B\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ At the beginning of your upkeep, you draw a card and you lose 1 life.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1 | SubAbility$ DBLose\nSVar:DBLose:DB$ LoseLife | LifeAmount$ 1\nOracle:\n",
    "Name:Optional Upkeep Lite\nManaCost:1\nTypes:Artifact\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | OptionalDecider$ You | Execute$ TrigGain | TriggerDescription$ At the beginning of each upkeep, you may gain 1 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 1\nOracle:\n",
    "Name:Attack Drummer Lite\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/2\nT:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDmg | TriggerDescription$ Whenever CARDNAME attacks, it deals 1 damage to defending player.\nSVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1\nOracle:\n",
    "Name:Firebrand Lite\nManaCost:1 R\nTypes:Creature Human Wizard\nPT:1/3\nT:Mode$ SpellCast | ValidCard$ Card.nonCreature | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDmg | TriggerDescription$ Whenever you cast a noncreature spell, CARDNAME deals 1 damage to each opponent.\nSVar:TrigDmg:DB$ DealDamage | Defined$ Player.Opponent | NumDmg$ 1\nOracle:\n",
    "Name:Ninja Lite\nManaCost:1 U\nTypes:Creature Human Ninja\nPT:1/1\nT:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDraw | TriggerDescription$ Whenever CARDNAME deals combat damage to a player, draw a card.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Shocking ETB Lite\nManaCost:2 R\nTypes:Creature Elemental\nPT:2/2\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDmg | TriggerDescription$ When CARDNAME enters, it deals 2 damage to target creature an opponent controls.\nSVar:TrigDmg:DB$ DealDamage | ValidTgts$ Creature.OppCtrl | NumDmg$ 2\nOracle:\n",
    "Name:Unsummon\nManaCost:U\nTypes:Instant\nA:SP$ ChangeZone | ValidTgts$ Creature | TgtPrompt$ Select target creature | Origin$ Battlefield | Destination$ Hand | SpellDescription$ Return target creature to its owner's hand.\nOracle:\n",
    "Name:Raise Dead\nManaCost:B\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Creature.YouOwn | TgtPrompt$ Choose target creature card in your graveyard | SpellDescription$ Return target creature card from your graveyard to your hand.\nOracle:\n",
    "Name:Reanimate Lite\nManaCost:B\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | ValidTgts$ Creature | TgtPrompt$ Choose target creature card in a graveyard | SpellDescription$ Put target creature card from a graveyard onto the battlefield under your control.\nOracle:\n",
    "Name:Rampant Growth\nManaCost:1 G\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | ChangeNum$ 1 | SpellDescription$ Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.\nOracle:\n",
    "Name:Swords to Plowshares Lite\nManaCost:W\nTypes:Instant\nA:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBGain | SpellDescription$ Exile target creature. Its controller gains life equal to 3.\nSVar:DBGain:DB$ GainLife | Defined$ TargetedController | LifeAmount$ 3\nOracle:\n",
    "Name:Evacuation\nManaCost:3 U U\nTypes:Instant\nA:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | Destination$ Hand | SpellDescription$ Return all creatures to their owners' hands.\nOracle:\n",
    "Name:Bottom Lite\nManaCost:W\nTypes:Instant\nA:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1 | SpellDescription$ Put target creature on the bottom of its owner's library.\nOracle:\n",
    "Name:Undying Lite\nManaCost:1 B\nTypes:Creature Zombie\nPT:2/1\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigReturn | TriggerDescription$ When CARDNAME dies, return it to its owner's hand.\nSVar:TrigReturn:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Hand\nOracle:\n",
    "Name:Glorious Anthem\nManaCost:1 W W\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1 | Description$ Creatures you control get +1/+1.\nOracle:\n",
    "Name:Goblin Lord Lite\nManaCost:R R\nTypes:Creature Goblin\nPT:2/2\nS:Mode$ Continuous | Affected$ Creature.Goblin+Other | AddPower$ 1 | AddToughness$ 1 | AddKeyword$ Haste | Description$ Other Goblins get +1/+1 and have haste.\nOracle:\n",
    "Name:Humility Lite\nManaCost:2 W W\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature | SetPower$ 1 | SetToughness$ 1 | Description$ Each creature has base power and toughness 1/1.\nOracle:\n",
    "Name:Living Lands Lite\nManaCost:3 G\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Forest | AddType$ Creature | SetPower$ 1 | SetToughness$ 1 | Description$ All Forests are 1/1 creatures that are still lands.\nOracle:\n",
    "Name:Darkest Hour\nManaCost:B\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature | SetColor$ Black | Description$ All creatures are black.\nOracle:\n",
    "Name:Akroma's Memorial Lite\nManaCost:7\nTypes:Legendary Artifact\nS:Mode$ Continuous | Affected$ Creature.YouCtrl | AddKeyword$ Flying & First Strike | Description$ Creatures you control have flying and first strike.\nOracle:\n",
    "Name:Zero Field Lite\nManaCost:2\nTypes:Artifact\nS:Mode$ Continuous | Affected$ Creature.Goblin | SetToughness$ 0 | Description$ Goblins have base toughness 0.\nOracle:\n",
    "Name:Overrun Count Lite\nManaCost:2 G\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SpellDescription$ Deal damage equal to the number of creatures you control.\nSVar:X:Count$Valid Creature.YouCtrl\nOracle:\n",
    "Name:Crusade Count Lite\nManaCost:2 W\nTypes:Enchantment\nS:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ X | AddToughness$ X | Description$ Creatures you control get +1/+1 for each creature you control.\nSVar:X:Count$Valid Creature.YouCtrl\nOracle:\n",
    "Name:Goyf Lite\nManaCost:1 G\nTypes:Creature Lhurgoyf\nPT:*/1+*\nS:Mode$ Continuous | EffectZone$ All | CharacteristicDefining$ True | SetPower$ X | SetToughness$ Y | Description$ P/T equal to cards in all graveyards (+1 toughness).\nSVar:X:Count$ValidGraveyard Card\nSVar:Y:Count$ValidGraveyard Card/Plus.1\nOracle:\n",
    "Name:Turn Fighter Lite\nManaCost:1 R\nTypes:Creature Human Warrior\nPT:1/1\nS:Mode$ Continuous | Affected$ Card.Self | AddPower$ 2 | Condition$ PlayerTurn | Description$ As long as it's your turn, CARDNAME gets +2/+0.\nOracle:\n",
    "Name:Island Flyer Lite\nManaCost:1 U\nTypes:Creature Fish\nPT:2/2\nS:Mode$ Continuous | Affected$ Card.Self | AddKeyword$ Flying | IsPresent$ Island.YouCtrl | Description$ CARDNAME has flying as long as you control an Island.\nOracle:\n",
    "Name:Low Life Healer Lite\nManaCost:1 W\nTypes:Enchantment\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | CheckSVar$ L | SVarCompare$ LE10 | Execute$ TrigGain | TriggerDescription$ At the beginning of your upkeep, if you have 10 or less life, you gain 2 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 2\nSVar:L:Count$YourLifeTotal\nOracle:\n",
    "Name:Forest Insight Lite\nManaCost:1 G\nTypes:Sorcery\nA:SP$ GainLife | LifeAmount$ 1 | SubAbility$ DBDraw | SpellDescription$ Gain 1 life. If you control a Forest, draw a card.\nSVar:DBDraw:DB$ Draw | NumCards$ 1 | ConditionPresent$ Forest.YouCtrl | ConditionCompare$ GE1\nOracle:\n",
    "Name:Big Game Hunter Lite\nManaCost:B\nTypes:Instant\nA:SP$ Destroy | ValidTgts$ Creature.powerGE4 | SpellDescription$ Destroy target creature with power 4 or greater.\nOracle:\n",
    "Name:Raise the Alarm\nManaCost:1 W\nTypes:Instant\nA:SP$ Token | TokenAmount$ 2 | TokenScript$ w_1_1_soldier | TokenOwner$ You | SpellDescription$ Create two 1/1 white Soldier creature tokens.\nOracle:\n",
    "Name:Soul Warden Lite\nManaCost:W\nTypes:Creature Human Cleric\nPT:1/1\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.Other | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ Whenever another creature enters, you gain 1 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 1\nOracle:\n",
    "Name:Beast Within\nManaCost:2 G\nTypes:Instant\nA:SP$ Destroy | ValidTgts$ Permanent | SubAbility$ DBToken | SpellDescription$ Destroy target permanent. Its controller creates a 3/3 green Beast creature token.\nSVar:DBToken:DB$ Token | TokenScript$ g_3_3_beast | TokenOwner$ TargetedController | TokenTapped$ True\nOracle:\n",
    "Name:Hydra Birth Lite\nManaCost:X G\nTypes:Sorcery\nA:SP$ Token | TokenScript$ g_x_x_hydra | TokenPower$ X | TokenToughness$ X | SpellDescription$ Create an X/X green Hydra creature token.\nSVar:X:Count$xPaid\nOracle:\n",
    "Name:Treasure Map Lite\nManaCost:1\nTypes:Sorcery\nA:SP$ Token | TokenScript$ c_a_treasure_sac | SpellDescription$ Create a Treasure token.\nOracle:\n",
    "Name:Triple Charm Lite\nManaCost:U\nTypes:Instant\nA:SP$ Charm | Choices$ DBBounce,DBDraw,DBGain | CharmNum$ 2\nSVar:DBBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature | SpellDescription$ Return target creature to its owner's hand.\nSVar:DBDraw:DB$ Draw | NumCards$ 1 | SpellDescription$ Draw a card.\nSVar:DBGain:DB$ GainLife | LifeAmount$ 3 | SpellDescription$ You gain 3 life.\nOracle:\n",
    "Name:Target Only Charm Lite\nManaCost:R\nTypes:Instant\nA:SP$ Charm | Choices$ DBShock,DBKill\nSVar:DBShock:DB$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | SpellDescription$ 2 damage to target creature.\nSVar:DBKill:DB$ Destroy | ValidTgts$ Artifact | SpellDescription$ Destroy target artifact.\nOracle:\n",
    "Name:Modal Trigger Lite\nManaCost:1 G\nTypes:Creature Elf\nPT:1/1\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigCharm | TriggerDescription$ When CARDNAME enters, choose one — Draw a card; or gain 2 life.\nSVar:TrigCharm:DB$ Charm | Choices$ DBDraw,DBGain\nSVar:DBDraw:DB$ Draw | NumCards$ 1 | SpellDescription$ Draw a card.\nSVar:DBGain:DB$ GainLife | LifeAmount$ 2 | SpellDescription$ Gain 2 life.\nOracle:\n",
    "Name:Tapped Land Lite\nManaCost:no cost\nTypes:Land\nR:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplacementResult$ Updated | ReplaceWith$ ETBTapped | Description$ CARDNAME enters tapped.\nSVar:ETBTapped:DB$ Tap | Defined$ Self | ETB$ True\nA:AB$ Mana | Cost$ T | Produced$ W | SpellDescription$ Add {W}.\nOracle:\n",
    "Name:Check Land Lite\nManaCost:no cost\nTypes:Land\nR:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ CARDNAME enters tapped unless you control a Plains.\nSVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True | ConditionPresent$ Plains.YouCtrl | ConditionCompare$ EQ0\nA:AB$ Mana | Cost$ T | Produced$ U | SpellDescription$ Add {U}.\nOracle:\n",
    "Name:Rest in Peace Lite\nManaCost:1 W\nTypes:Enchantment\nR:Event$ Moved | ActiveZones$ Battlefield | Destination$ Graveyard | ValidCard$ Card | ReplaceWith$ Exile | Description$ If a card or token would be put into a graveyard from anywhere, exile it instead.\nSVar:Exile:DB$ ChangeZone | Hidden$ True | Origin$ All | Destination$ Exile | Defined$ ReplacedCard\nOracle:\n",
    "Name:Counter Bear Lite\nManaCost:1 G\nTypes:Creature Bear\nPT:1/1\nK:etbCounter:P1P1:2\nOracle:\n",
    "Name:X Ooze Lite\nManaCost:X G\nTypes:Creature Ooze\nPT:0/0\nK:etbCounter:P1P1:X\nSVar:X:Count$xPaid\nOracle:\n",
    "Name:Overrun Lite\nManaCost:2 G G G\nTypes:Sorcery\nA:SP$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ +3 | NumDef$ +3 | KW$ Trample | SpellDescription$ Creatures you control get +3/+3 and gain trample until end of turn.\nOracle:\n",
    "Name:Wrath of God\nManaCost:2 W W\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Creature | NoRegen$ True | SpellDescription$ Destroy all creatures. They can't be regenerated.\nOracle:\n",
    "Name:Pyroclasm\nManaCost:1 R\nTypes:Sorcery\nA:SP$ DamageAll | NumDmg$ 2 | ValidCards$ Creature | ValidDescription$ each creature. | SpellDescription$ CARDNAME deals 2 damage to each creature.\nOracle:\n",
    "Name:Earthquake Lite\nManaCost:X R\nTypes:Sorcery\nA:SP$ DamageAll | NumDmg$ X | ValidCards$ Creature.withoutFlying | ValidPlayers$ Player | SpellDescription$ X damage to each creature without flying and each player.\nSVar:X:Count$xPaid\nOracle:\n",
    "Name:Mind Rot\nManaCost:2 B\nTypes:Sorcery\nA:SP$ Discard | ValidTgts$ Player | NumCards$ 2 | Mode$ TgtChoose | SpellDescription$ Target player discards two cards.\nOracle:\n",
    "Name:Duress\nManaCost:B\nTypes:Sorcery\nA:SP$ Discard | ValidTgts$ Opponent | Mode$ RevealYouChoose | DiscardValid$ Card.nonCreature+nonLand | NumCards$ 1 | SpellDescription$ Target opponent reveals their hand. You choose a noncreature, nonland card from it. That player discards that card.\nOracle:\n",
    "Name:Counterspell\nManaCost:U U\nTypes:Instant\nA:SP$ Counter | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card | SpellDescription$ Counter target spell.\nOracle:\n",
    "Name:Diabolic Edict\nManaCost:1 B\nTypes:Instant\nA:SP$ Sacrifice | ValidTgts$ Player | SacValid$ Creature | SacMessage$ Creature | SpellDescription$ Target player sacrifices a creature.\nOracle:\n",
    "Name:Innocent Blood\nManaCost:B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ Player | SacValid$ Creature | SacMessage$ Creature | SpellDescription$ Each player sacrifices a creature.\nOracle:\n",
    "Name:Optional Offering Lite\nManaCost:B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | SacValid$ Creature | Amount$ 2 | Optional$ True | RememberSacrificed$ True | SpellDescription$ You may sacrifice up to two creatures.\nOracle:\n",
    "Name:Strict Pact Lite\nManaCost:B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | SacValid$ Creature | Amount$ 2 | StrictAmount$ True | SpellDescription$ Sacrifice two creatures (only if you can).\nOracle:\n",
    "Name:Upkeep Tax Lite\nManaCost:2 B\nTypes:Creature Horror\nPT:4/4\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigSac | TriggerDescription$ At the beginning of your upkeep, sacrifice CARDNAME unless you pay {1}.\nSVar:TrigSac:DB$ Sacrifice | SacValid$ Self | UnlessCost$ 1 | UnlessPayer$ You\nOracle:\n",
    "Name:Painful Choice Lite\nManaCost:1 R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 4 | UnlessCost$ Discard<1/Card> | UnlessPayer$ Targeted | SpellDescription$ CARDNAME deals 4 damage to target player unless that player discards a card.\nOracle:\n",
    "Name:Tribute Lite\nManaCost:1 G\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 2 | UnlessCost$ PayLife<3> | UnlessPayer$ You | UnlessSwitched$ True | UnlessResolveSubs$ WhenPaid | SubAbility$ DBGain | SpellDescription$ You may pay 3 life. If you do, draw two cards and gain 1 life.\nSVar:DBGain:DB$ GainLife | LifeAmount$ 1\nOracle:\n",
    "Name:Offering Draw Lite\nManaCost:B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | SacValid$ Creature | Optional$ True | RememberSacrificed$ True | SubAbility$ DBDraw | SpellDescription$ You may sacrifice a creature. If you do, draw two cards.\nSVar:DBDraw:DB$ Draw | NumCards$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ GE1 | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Mass Offering Lite\nManaCost:B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | SacValid$ Creature | Amount$ 3 | Optional$ True | RememberSacrificed$ True | SubAbility$ DBGain | SpellDescription$ Sacrifice up to three creatures. You gain 2 life for each creature sacrificed this way.\nSVar:DBGain:DB$ GainLife | LifeAmount$ X | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nSVar:X:Remembered$Amount/Times.2\nOracle:\n",
    "Name:Recycle Lite\nManaCost:B\nTypes:Sorcery\nA:SP$ Sacrifice | Defined$ You | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBReturn | SpellDescription$ Sacrifice a creature, then return it to your hand.\nSVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Graveyard | Destination$ Hand | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Sac Watcher Lite\nManaCost:1 B\nTypes:Enchantment\nT:Mode$ Sacrificed | ValidCard$ Creature | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ Whenever you sacrifice a creature, draw a card.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Martyr Lite\nManaCost:W\nTypes:Creature Human\nPT:1/1\nA:AB$ GainLife | Cost$ Sac<1/CARDNAME> | LifeAmount$ 1 | SpellDescription$ Sacrifice CARDNAME: You gain 1 life.\nT:Mode$ Sacrificed | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ When you sacrifice CARDNAME, you gain 3 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 3\nOracle:\n",
    "Name:War Horn Lite\nManaCost:2\nTypes:Artifact\nT:Mode$ AttackersDeclared | AttackingPlayer$ You | ValidAttackers$ Creature | ValidAttackersAmount$ GE2 | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ Whenever you attack with two or more creatures, each opponent loses 1 life and you gain 1 life.\nSVar:TrigDrain:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 1 | SubAbility$ DBGain\nSVar:DBGain:DB$ GainLife | Defined$ TriggeredAttackingPlayer | LifeAmount$ 1\nOracle:\n",
    "Name:Raid Banner Lite\nManaCost:2\nTypes:Artifact\nT:Mode$ AttackersDeclaredOneTarget | AttackedTarget$ Player | ValidAttackers$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDmg | TriggerDescription$ Whenever you attack a player, CARDNAME deals 1 damage to that player.\nSVar:TrigDmg:DB$ DealDamage | Defined$ TriggeredAttackedTarget | NumDmg$ 1\nOracle:\n",
    "Name:Flicker Lite\nManaCost:W\nTypes:Instant\nA:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBDelay | SpellDescription$ Exile target creature you control. Return it at the beginning of the next end step.\nSVar:DBDelay:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | RememberObjects$ Targeted | Execute$ TrigReturn | TriggerDescription$ Return the exiled card.\nSVar:TrigReturn:DB$ ChangeZone | Defined$ DelayTriggerRemembered | Origin$ Exile | Destination$ Battlefield\nOracle:\n",
    "Name:Next Upkeep Draw Lite\nManaCost:U\nTypes:Sorcery\nA:SP$ DelayedTrigger | Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigDraw | SpellDescription$ Draw a card at the beginning of your next upkeep.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:This Turn Upkeep Draw Lite\nManaCost:U\nTypes:Sorcery\nA:SP$ DelayedTrigger | Mode$ Phase | Phase$ Upkeep | ThisTurn$ True | Execute$ TrigDraw | SpellDescription$ Useless: upkeep already passed this turn.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Bite Lite\nManaCost:G\nTypes:Instant\nA:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +1 | SubAbility$ DBDamage | SpellDescription$ Target creature you control gets +1/+0 until end of turn, then deals damage equal to its power to each opponent.\nSVar:DBDamage:DB$ DealDamage | Defined$ Opponent | NumDmg$ X\nSVar:X:ParentTargeted$CardPower\nOracle:\n",
    "Name:Hand Burn Lite\nManaCost:R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Player | NumDmg$ X | SpellDescription$ Deals damage to target player equal to the number of cards in that player's hand.\nSVar:X:TargetedPlayer$CardsInHand\nOracle:\n",
    "Name:Death Echo Lite\nManaCost:1 W\nTypes:Enchantment\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ Whenever a creature you control dies, you gain life equal to its power.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ X\nSVar:X:TriggeredCard$CardPower\nOracle:\n",
    "Name:Ajani Lite\nManaCost:2 W\nTypes:Legendary Planeswalker Ajani\nLoyalty:3\nA:AB$ GainLife | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | LifeAmount$ 2 | SpellDescription$ You gain 2 life.\nA:AB$ DealDamage | Cost$ SubCounter<3/LOYALTY> | Planeswalker$ True | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ Deal 3 damage to any target.\nOracle:\n",
    "Name:Cycler Lite\nManaCost:4 U\nTypes:Creature Serpent\nPT:5/5\nK:Cycling:2\nOracle:\n",
    "Name:Plains Cycler Lite\nManaCost:4 W\nTypes:Creature Bird\nPT:3/3\nK:TypeCycling:Plains:1\nOracle:\n",
    "Name:Devoid Lite\nManaCost:1 R\nTypes:Creature Eldrazi\nPT:2/1\nK:Devoid\nOracle:\n",
    "Name:Wall Watcher Lite\nManaCost:1 W\nTypes:Creature Wall\nPT:0/4\nK:Defender\nT:Mode$ Blocks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ Whenever CARDNAME blocks, you gain 2 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 2\nOracle:\n",
    "Name:Rampager Lite\nManaCost:2 G\nTypes:Creature Beast\nPT:3/3\nT:Mode$ AttackerBlocked | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigPump | TriggerDescription$ Whenever CARDNAME becomes blocked, it gets +2/+2 until end of turn.\nSVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +2 | NumDef$ +2\nT:Mode$ AttackerUnblocked | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ Whenever CARDNAME attacks and isn't blocked, draw a card.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Second Draw Lite\nManaCost:1 U\nTypes:Enchantment\nT:Mode$ Drawn | ValidCard$ Card.YouOwn | Number$ 2 | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ Whenever you draw your second card each turn, gain 3 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 3\nOracle:\n",
    "Name:Life Echo Lite\nManaCost:W\nTypes:Enchantment\nT:Mode$ LifeGained | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ Whenever you gain life, each opponent loses 1 life.\nSVar:TrigDrain:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 1\nOracle:\n",
    "Name:Tap Watcher Lite\nManaCost:B\nTypes:Enchantment\nT:Mode$ Taps | ValidCard$ Creature.OppCtrl | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ Whenever a creature an opponent controls becomes tapped, that player loses 1 life.\nSVar:TrigDrain:DB$ LoseLife | Defined$ TriggeredCardController | LifeAmount$ 1\nOracle:\n",
    "Name:Surveil Lite\nManaCost:U\nTypes:Sorcery\nA:SP$ Surveil | Amount$ 2 | SpellDescription$ Surveil 2.\nOracle:\n",
    "Name:Combat Watcher Lite\nManaCost:1 R\nTypes:Enchantment\nT:Mode$ DamageDoneOnce | ValidTarget$ Opponent | CombatDamage$ True | ValidSource$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDraw | TriggerDescription$ Whenever one or more creatures you control deal combat damage to an opponent, draw a card.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Kill Bonus Lite\nManaCost:B\nTypes:Instant\nA:SP$ Destroy | ValidTgts$ Creature | SubAbility$ DBDraw | SpellDescription$ Destroy target creature. If it was red, draw a card.\nSVar:DBDraw:DB$ Draw | NumCards$ 1 | ConditionDefined$ Targeted | ConditionPresent$ Creature.Red | ConditionCompare$ GE1\nOracle:\n",
    "Name:Furnace Lite\nManaCost:3 R\nTypes:Enchantment\nR:Event$ DamageDone | ActiveZones$ Battlefield | ValidSource$ Card.YouCtrl | ValidTarget$ Permanent.OppCtrl,Opponent | ReplaceWith$ DmgTwice | Description$ If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals double that damage instead.\nSVar:DmgTwice:DB$ ReplaceEffect | VarName$ DamageAmount | VarValue$ X\nSVar:X:ReplaceCount$DamageAmount/Twice\nOracle:\n",
    "Name:Mist Wall Lite\nManaCost:1 W\nTypes:Creature Wall\nPT:0/3\nK:Defender\nR:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | IsCombat$ True | Prevent$ True | Description$ Prevent all combat damage that would be dealt to CARDNAME.\nOracle:\n",
    "Name:Salve Lite\nManaCost:W\nTypes:Instant\nA:SP$ PreventDamage | ValidTgts$ Any | Amount$ 3 | SpellDescription$ Prevent the next 3 damage that would be dealt to any target this turn.\nOracle:\n",
    "Name:Threaten Lite\nManaCost:2 R\nTypes:Sorcery\nA:SP$ GainControl | ValidTgts$ Creature | LoseControl$ EOT | Untap$ True | AddKWs$ Haste | SpellDescription$ Untap target creature and gain control of it until end of turn. It gains haste.\nOracle:\n",
    "Name:Steal Lite\nManaCost:U\nTypes:Sorcery\nA:SP$ GainControl | ValidTgts$ Creature | SpellDescription$ Gain control of target creature.\nOracle:\n",
    "Name:Seizer Lite\nManaCost:2 U\nTypes:Creature Human Wizard\nPT:1/1\nA:AB$ GainControl | Cost$ T | ValidTgts$ Creature | LoseControl$ LeavesPlay | SpellDescription$ Gain control of target creature for as long as CARDNAME remains on the battlefield.\nOracle:\n",
    "Name:Acidic Soil Lite\nManaCost:2 R\nTypes:Sorcery\nA:SP$ RepeatEach | RepeatPlayers$ Player | RepeatSubAbility$ DBDamage | DamageMap$ True | SpellDescription$ Deals damage to each player equal to the number of lands they control.\nSVar:DBDamage:DB$ DealDamage | Defined$ Remembered | NumDmg$ X\nSVar:X:Count$Valid Land.RememberedPlayerCtrl\nOracle:\n",
    "Name:Each Draws Lite\nManaCost:1 U\nTypes:Sorcery\nA:SP$ RepeatEach | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ DBDraw | SpellDescription$ Each opponent draws a card and loses 1 life.\nSVar:DBDraw:DB$ Draw | Defined$ Player.IsRemembered | NumCards$ 1 | SubAbility$ DBLose\nSVar:DBLose:DB$ LoseLife | Defined$ Remembered | LifeAmount$ 1\nOracle:\n",
    "Name:Creature Pinger Lite\nManaCost:R\nTypes:Sorcery\nA:SP$ RepeatEach | RepeatCards$ Creature | RepeatSubAbility$ DBDmg | SpellDescription$ For each creature, deal 1 damage to it.\nSVar:DBDmg:DB$ DealDamage | Defined$ Remembered | NumDmg$ 1\nOracle:\n",
    "Name:Mass Grief Lite\nManaCost:2 B\nTypes:Enchantment\nT:Mode$ ChangesZoneAll | ValidCards$ Creature | Origin$ Battlefield | Destination$ Graveyard | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ Whenever one or more creatures die, each opponent loses life equal to the number of creatures that died.\nSVar:TrigDrain:DB$ LoseLife | Defined$ Opponent | LifeAmount$ X\nSVar:X:TriggeredCards$Amount\nOracle:\n",
    "Name:Warded Bear Lite\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2\nK:Ward:2\nOracle:\n",
    "Name:Heroic Lite\nManaCost:W\nTypes:Creature Human Soldier\nPT:1/1\nT:Mode$ BecomesTarget | ValidTarget$ Card.Self | ValidSource$ Spell.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigCounter | TriggerDescription$ Whenever you cast a spell that targets CARDNAME, put a +1/+1 counter on it.\nSVar:TrigCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1\nOracle:\n",
    "Name:Prowess Monk Lite\nManaCost:1 R\nTypes:Creature Human Monk\nPT:1/2\nK:Prowess\nOracle:\n",
    "Name:Cycle Reward Lite\nManaCost:3 W\nTypes:Creature Bird\nPT:2/2\nK:Cycling:1\nT:Mode$ Cycled | ValidCard$ Card.Self | Execute$ TrigGain | TriggerDescription$ When you cycle CARDNAME, you gain 3 life.\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 3\nOracle:\n",
    "Name:Discard Punisher Lite\nManaCost:1 B\nTypes:Enchantment\nT:Mode$ Discarded | ValidCard$ Card.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ Whenever you discard a card, each opponent loses 1 life.\nSVar:TrigDrain:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 1\nOracle:\n",
    "Name:Prey Lite\nManaCost:G\nTypes:Sorcery\nA:SP$ Pump | ValidTgts$ Creature.YouCtrl | NumAtt$ +1 | SubAbility$ DBFight | SpellDescription$ Target creature you control gets +1/+0, then fights target creature you don't control.\nSVar:DBFight:DB$ Fight | Defined$ ParentTarget | ValidTgts$ Creature.YouDontCtrl\nOracle:\n",
    "Name:Inspire Lite\nManaCost:W\nTypes:Sorcery\nA:SP$ PutCounterAll | ValidCards$ Creature.YouCtrl | CounterType$ P1P1 | CounterNum$ 1 | SpellDescription$ Put a +1/+1 counter on each creature you control.\nOracle:\n",
    "Name:Thorn Lite\nManaCost:2 G\nTypes:Creature Plant\nPT:2/2\nT:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature | Execute$ TrigDestroy | TriggerDescription$ Whenever CARDNAME becomes blocked by a creature, destroy that creature.\nSVar:TrigDestroy:DB$ Destroy | Defined$ TriggeredBlocker\nOracle:\n",
    "Name:Grave Spark Lite\nManaCost:R\nTypes:Creature Elemental\nPT:1/1\nA:AB$ DealDamage | Cost$ 1 ExileFromGrave<1/CARDNAME> | ActivationZone$ Graveyard | ValidTgts$ Any | NumDmg$ 2 | SpellDescription$ Deal 2 damage to any target.\nOracle:\n",
    "Name:Giant Lite\nManaCost:2 R\nTypes:Creature Giant\nPT:4/3\nAlternateMode:Adventure\nOracle:\n\nALTERNATE\n\nName:Stomp Lite\nManaCost:1 R\nTypes:Instant Adventure\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | SpellDescription$ Stomp deals 2 damage to any target.\nOracle:\n",
    "Name:Valakut Lite\nManaCost:2 R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ 3 damage.\nAlternateMode:Modal\nOracle:\n\nALTERNATE\n\nName:Valakut Cliffs Lite\nManaCost:no cost\nTypes:Land\nR:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield | ReplaceWith$ ETBTapped | ReplacementResult$ Updated | Description$ CARDNAME enters tapped.\nSVar:ETBTapped:DB$ Tap | Defined$ Self | ETB$ True\nA:AB$ Mana | Cost$ T | Produced$ R | SpellDescription$ Add {R}.\nOracle:\n",
    "Name:Villager Lite\nManaCost:1 G\nTypes:Creature Human Werewolf\nPT:2/2\nA:AB$ SetState | Cost$ 2 | Defined$ Self | Mode$ Transform | SorcerySpeed$ True | SpellDescription$ Transform CARDNAME.\nAlternateMode:DoubleFaced\nOracle:\n\nALTERNATE\n\nName:Howler Lite\nManaCost:no cost\nColors:green\nTypes:Creature Werewolf\nPT:4/4\nK:Trample\nOracle:\n",
    "Name:Reckoner Lite\nManaCost:2 R\nTypes:Creature Minotaur\nPT:3/3\nT:Mode$ DamageDoneOnce | Execute$ TrigDamage | ValidTarget$ Card.Self | TriggerZones$ Battlefield | TriggerDescription$ x\nSVar:TrigDamage:DB$ DealDamage | NumDmg$ X | ValidTgts$ Any\nSVar:X:TriggerCount$DamageAmount\nOracle:\n",
    "Name:Life Mirror Lite\nManaCost:1 B\nTypes:Enchantment\nT:Mode$ LifeGained | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDrain | TriggerDescription$ x\nSVar:TrigDrain:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ X\nSVar:X:TriggerCount$LifeAmount/Twice\nOracle:\n",
    "Name:Fling Lite\nManaCost:1 R\nTypes:Instant\nA:SP$ DealDamage | Cost$ 1 R Sac<1/Creature> | ValidTgts$ Any | NumDmg$ X | SpellDescription$ x\nSVar:X:Sacrificed$CardPower\nOracle:\n",
    "Name:Morbid Shock Lite\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SpellDescription$ x\nSVar:X:Count$Morbid.4.1\nOracle:\n",
    "Name:Saga Lite\nManaCost:W\nTypes:Enchantment Saga\nK:Chapter:3:DBGain,DBGain,DBDraw\nSVar:DBGain:DB$ GainLife | LifeAmount$ 2 | SpellDescription$ x\nSVar:DBDraw:DB$ Draw | NumCards$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Blink Lite\nManaCost:W\nTypes:Instant\nA:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBReturn | SpellDescription$ x\nSVar:DBReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Once Pump Lite\nManaCost:G\nTypes:Creature Elf\nPT:1/1\nA:AB$ Pump | Cost$ G | Defined$ Self | NumAtt$ +1 | NumDef$ +1 | ActivationLimit$ 1 | SpellDescription$ x\nA:AB$ GainLife | Cost$ G | LifeAmount$ 1 | GameActivationLimit$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Exile Self Lite\nManaCost:U\nTypes:Creature Spirit\nPT:1/1\nA:AB$ Draw | Cost$ Exile<1/CARDNAME> | NumCards$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Tap Crew Lite\nManaCost:W\nTypes:Enchantment\nA:AB$ GainLife | Cost$ tapXType<2/Creature> | LifeAmount$ X | SpellDescription$ x\nSVar:X:Tapped$Amount\nOracle:\n",
    "Name:Grave Eater Lite\nManaCost:B\nTypes:Enchantment\nA:AB$ DealDamage | Cost$ ExileFromGrave<1/Creature> | ValidTgts$ Any | NumDmg$ X | SpellDescription$ x\nSVar:X:Exiled$CardPower\nOracle:\n",
    "Name:Land Bounce Lite\nManaCost:U\nTypes:Enchantment\nA:AB$ Draw | Cost$ Return<1/Land;Creature> | NumCards$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Reflex Lite\nManaCost:R\nTypes:Enchantment\nA:AB$ Discard | Cost$ 1 | Defined$ You | Mode$ TgtChoose | NumCards$ 1 | RememberDiscarded$ True | SubAbility$ DBImmediate | SpellDescription$ x\nSVar:DBImmediate:DB$ ImmediateTrigger | Execute$ TrigDmg | ConditionDefined$ Remembered | ConditionPresent$ Card | TriggerAmount$ 2 | SubAbility$ DBCleanup | TriggerDescription$ When you do, deal 1 damage to any target twice.\nSVar:TrigDmg:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Tapper Lite\nManaCost:W\nTypes:Creature Human\nPT:1/1\nA:AB$ GainLife | Cost$ T tapXType<1/Creature> | LifeAmount$ 1 | SpellDescription$ x\nOracle:\n",
    "Name:Peek Lite\nManaCost:G\nTypes:Sorcery\nA:SP$ PeekAndReveal | PeekAmount$ 1 | RevealValid$ Land | RevealOptional$ True | RememberRevealed$ True | SubAbility$ DBTake | SpellDescription$ x\nSVar:DBTake:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Peek Creature Lite\nManaCost:G\nTypes:Sorcery\nA:SP$ PeekAndReveal | PeekAmount$ 1 | RevealValid$ Creature | RememberRevealed$ True | SubAbility$ DBTake | SpellDescription$ x\nSVar:DBTake:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand | SubAbility$ DBCleanup\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Spare Lite\nManaCost:W\nTypes:Sorcery\nA:SP$ ChooseCard | Defined$ You | Choices$ Creature.YouCtrl | Mandatory$ True | SubAbility$ DBWrath | SpellDescription$ x\nSVar:DBWrath:DB$ DestroyAll | ValidCards$ Creature.nonChosenCard | SubAbility$ DBPump\nSVar:DBPump:DB$ Pump | Defined$ ChosenCard | NumAtt$ +2 | ConditionDefined$ ChosenCard | ConditionPresent$ Creature | ConditionCompare$ GE1\nOracle:\n",
    "Name:Heroic Soldier Lite\nManaCost:W\nTypes:Creature Human Soldier\nPT:1/1\nT:Mode$ SpellCast | ValidActivatingPlayer$ You | TargetsValid$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigPut | TriggerDescription$ x\nSVar:TrigPut:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1\nOracle:\n",
    "Name:Frozen Giant Lite\nManaCost:3 R\nTypes:Creature Giant\nPT:5/5\nR:Event$ Untap | ActiveZones$ Battlefield | ValidCard$ Card.Self | ValidStepTurnToController$ You | Layer$ CantHappen | Description$ CARDNAME doesn't untap during your untap step.\nOracle:\n",
    "Name:Impulse Exile Lite\nManaCost:R\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect | SpellDescription$ x\nSVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ STPlay | Duration$ EndOfTurn | SubAbility$ DBCleanup\nSVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile | Description$ x\nSVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nOracle:\n",
    "Name:Free Grave Lite\nManaCost:2 U\nTypes:Enchantment\nS:Mode$ Continuous | MayPlay$ True | MayPlayWithoutManaCost$ True | MayPlayLimit$ 1 | Affected$ Instant.YouOwn | AffectedZone$ Graveyard | Description$ x\nOracle:\n",
    "Name:Hand Exile Lite\nManaCost:2 W\nTypes:Creature Spirit\nPT:2/2\nA:AB$ GainLife | Cost$ ExileFromHand<1/CARDNAME> | ActivationZone$ Hand | LifeAmount$ 2 | SubAbility$ DBShuffle | SpellDescription$ x\nSVar:DBShuffle:DB$ Shuffle | Defined$ You\nOracle:\n",
    "Name:Force Lite\nManaCost:3 U U\nTypes:Instant\nS:Mode$ AlternativeCost | ValidSA$ Spell.Self | EffectZone$ All | Cost$ PayLife<1> ExileFromHand<1/Card.Blue+Other> | Description$ x\nA:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SpellDescription$ x\nOracle:\n",
    "Name:Lone Wolf Lite\nManaCost:1 G\nTypes:Creature Wolf\nPT:2/2\nT:Mode$ Attacks | ValidCard$ Card.Self | Alone$ True | Execute$ TrigPump | TriggerDescription$ x\nSVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +2 | NumDef$ +2\nOracle:\n",
    "Name:Second Spell Lite\nManaCost:1 U\nTypes:Enchantment\nT:Mode$ SpellCast | ValidActivatingPlayer$ You | ActivatorThisTurnCast$ EQ2 | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ x\nSVar:TrigGain:DB$ GainLife | LifeAmount$ 3\nOracle:\n",
    "Name:Dealt Once Lite\nManaCost:1 R\nTypes:Enchantment\nT:Mode$ DamageDealtOnce | ValidSource$ Card.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigGain | TriggerDescription$ x\nSVar:TrigGain:DB$ GainLife | LifeAmount$ X\nSVar:X:TriggerCount$DamageAmount\nOracle:\n",
    "Name:Shapeshifter Lite\nManaCost:1 U\nTypes:Creature Shapeshifter\nPT:1/1\nK:Changeling\nOracle:\n",
    "Name:Elf Lord Lite\nManaCost:1 G\nTypes:Creature Elf\nPT:2/2\nS:Mode$ Continuous | Affected$ Elf.Other+YouCtrl | AddPower$ 1 | Description$ x\nOracle:\n",
    "Name:Kind Lord Lite\nManaCost:1 W\nTypes:Creature Human\nPT:1/1\nK:ETBReplacement:Other:ChooseCT\nSVar:ChooseCT:DB$ ChooseType | Defined$ You | Type$ Creature | SpellDescription$ x\nS:Mode$ Continuous | Affected$ Creature.ChosenType+YouCtrl | AddPower$ 1 | Description$ x\nOracle:\n",
    "Name:Hue Land Lite\nManaCost:no cost\nTypes:Land\nK:ETBReplacement:Other:ChooseColor\nSVar:ChooseColor:DB$ ChooseColor | Defined$ You | SpellDescription$ x\nA:AB$ Mana | Cost$ T | Produced$ ChosenColor | SpellDescription$ x\nS:Mode$ Continuous | Affected$ Creature.ChosenColor+YouCtrl | AddPower$ 1 | Description$ x\nOracle:\n",
    "Name:Cradle Lite\nManaCost:no cost\nTypes:Legendary Land\nA:AB$ Mana | Cost$ T | Produced$ G | Amount$ X | SpellDescription$ x\nSVar:X:Count$Valid Creature.YouCtrl\nOracle:\n",
    "Name:Clone Lite\nManaCost:U\nTypes:Creature Shapeshifter\nPT:0/0\nK:ETBReplacement:Copy:DBCopy:Optional\nSVar:DBCopy:DB$ Clone | Choices$ Creature.Other | SpellDescription$ x\nOracle:\n",
    "Name:Holy Day\nManaCost:W\nTypes:Instant\nA:SP$ Fog | SpellDescription$ Prevent all combat damage that would be dealt this turn.\nOracle:\n",
    "Name:Stoke the Flames\nManaCost:2 R R\nTypes:Instant\nK:Convoke\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 4 | SpellDescription$ x\nOracle:\n",
    "Name:Treasure Cruise\nManaCost:7 U\nTypes:Sorcery\nK:Delve\nA:SP$ Draw | NumCards$ 3 | SpellDescription$ x\nOracle:\n",
    "Name:Frogmite\nManaCost:4\nTypes:Artifact Creature Frog\nPT:2/2\nK:Affinity:Artifact\nOracle:\n",
    "Name:Reverse Engineer\nManaCost:3 U U\nTypes:Sorcery\nK:Improvise\nA:SP$ Draw | NumCards$ 3 | SpellDescription$ x\nOracle:\n",
    "Name:Ornithopter\nManaCost:0\nTypes:Artifact Creature Thopter\nPT:0/2\nK:Flying\nOracle:\n",
    "Name:All Is Dust\nManaCost:7\nTypes:Kindred Sorcery Eldrazi\nA:SP$ SacrificeAll | ValidCards$ Permanent.nonColorless | SpellDescription$ x\nOracle:\n",
    "Name:Thraben Inspector\nManaCost:W\nTypes:Creature Human Soldier\nPT:1/2\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigInvestigate | TriggerDescription$ x\nSVar:TrigInvestigate:DB$ Investigate\nOracle:\n",
    "Name:Proliferate Lite\nManaCost:U\nTypes:Sorcery\nA:SP$ Proliferate | SpellDescription$ Proliferate.\nOracle:\n",
    "Name:Mass Animate Lite\nManaCost:G\nTypes:Sorcery\nA:SP$ AnimateAll | ValidCards$ Land.YouCtrl | Types$ Creature,Elemental | Power$ 3 | Toughness$ 3 | SpellDescription$ x\nOracle:\n",
    "Name:Mass Tap Lite\nManaCost:U\nTypes:Instant\nA:SP$ TapAll | ValidTgts$ Player | ValidCards$ Creature | SubAbility$ DBUntap | SpellDescription$ x\nSVar:DBUntap:DB$ UntapAll | ValidCards$ Land.YouCtrl\nOracle:\n",
    "Name:Kiki Lite\nManaCost:2 R\nTypes:Creature Goblin Shaman\nPT:2/2\nK:Haste\nA:AB$ CopyPermanent | Cost$ T | ValidTgts$ Creature.nonLegendary+YouCtrl+Other | AddKeywords$ Haste | AtEOT$ Sacrifice | SpellDescription$ x\nOracle:\n",
    "Name:Mana Leak\nManaCost:1 U\nTypes:Instant\nA:SP$ Counter | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card | UnlessCost$ 3 | SpellDescription$ Counter target spell unless its controller pays {3}.\nOracle:\n",
    "Name:Essence Scatter Lite\nManaCost:1 U\nTypes:Instant\nA:SP$ Counter | TargetType$ Spell | ValidTgts$ Creature | TgtPrompt$ Select target creature spell | SpellDescription$ Counter target creature spell.\nOracle:\n",
    "Name:Blessing Lite\nManaCost:W\nTypes:Instant\nA:SP$ Effect | ValidTgts$ Creature | StaticAbilities$ STHexproof | RememberObjects$ Targeted | SpellDescription$ Target creature gains hexproof until end of turn.\nSVar:STHexproof:Mode$ Continuous | Affected$ Creature.IsRemembered | AddKeyword$ Hexproof | Description$ Hexproof.\nOracle:\n",
    "Name:Emblem Anthem Lite\nManaCost:2 W\nTypes:Sorcery\nA:SP$ Effect | StaticAbilities$ STPump | Duration$ Permanent | SpellDescription$ You get an emblem with \"Creatures you control get +1/+1.\"\nSVar:STPump:Mode$ Continuous | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1 | Description$ Creatures you control get +1/+1.\nOracle:\n",
    "Name:Fog Wall Lite\nManaCost:1 W\nTypes:Sorcery\nA:SP$ Effect | StaticAbilities$ STPump | Duration$ UntilYourNextTurn | SpellDescription$ Until your next turn, creatures you control get +0/+3.\nSVar:STPump:Mode$ Continuous | Affected$ Creature.YouCtrl | AddToughness$ 3 | Description$ +0/+3.\nOracle:\n",
    "Name:Grim Harvest Lite\nManaCost:1 B\nTypes:Instant\nA:SP$ Effect | Triggers$ TrigDies | SpellDescription$ This turn, whenever a creature you control dies, draw a card.\nSVar:TrigDies:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl | Execute$ TrigDraw | TriggerDescription$ Whenever a creature you control dies, draw a card.\nSVar:TrigDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Phantom Warrior Lite\nManaCost:1 U U\nTypes:Creature Illusion Warrior\nPT:2/2\nS:Mode$ Continuous | Affected$ Card.Self | AddPower$ 0 | Description$ dummy\nS:Mode$ CantBlockBy | ValidAttacker$ Creature.Self | Description$ CARDNAME can't be blocked.\nOracle:\n",
    "Name:Skulk Lite\nManaCost:1 B\nTypes:Instant\nA:SP$ Effect | ValidTgts$ Creature | StaticAbilities$ BlockRestrict | RememberObjects$ Targeted | SpellDescription$ Target creature can't be blocked by creatures with power 2 or less this turn.\nSVar:BlockRestrict:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered | ValidBlocker$ Creature.powerLE2 | Description$ That creature can't be blocked by creatures with power 2 or less this turn.\nOracle:\n",
    "Name:Craven Lite\nManaCost:1 R\nTypes:Creature Goblin\nPT:3/1\nS:Mode$ CantBlock | ValidCard$ Card.Self | Description$ CARDNAME can't block.\nOracle:\n",
    "Name:Pacifism Field Lite\nManaCost:1 W\nTypes:Enchantment\nS:Mode$ CantAttack | ValidCard$ Creature.OppCtrl | Description$ Creatures your opponents control can't attack.\nOracle:\n",
    "Name:Berserker Lite\nManaCost:R\nTypes:Creature Human Berserker\nPT:2/1\nS:Mode$ MustAttack | ValidCreature$ Card.Self | Description$ CARDNAME attacks each combat if able.\nOracle:\n",
    "Name:Rancor Lite\nManaCost:G\nTypes:Enchantment Aura\nK:Enchant:Creature\nS:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ 2 | AddKeyword$ Trample | Description$ Enchanted creature gets +2/+0 and has trample.\nT:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigChangeZone | TriggerDescription$ When CARDNAME is put into a graveyard from the battlefield, return CARDNAME to its owner's hand.\nSVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ TriggeredNewCardLKICopy\nOracle:\n",
    "Name:Pacifism\nManaCost:1 W\nTypes:Enchantment Aura\nK:Enchant:Creature\nS:Mode$ CantAttack,CantBlock | ValidCard$ Creature.EnchantedBy | Description$ Enchanted creature can't attack or block.\nOracle:\n",
    "Name:Bonesplitter\nManaCost:1\nTypes:Artifact Equipment\nK:Equip:1\nS:Mode$ Continuous | AffectedDefined$ Equipped | Affected$ Creature | AddPower$ 2 | Description$ Equipped creature gets +2/+0.\nOracle:\n",
    "Name:Opt Lite\nManaCost:U\nTypes:Instant\nA:SP$ Scry | ScryNum$ 2 | SubAbility$ DBDraw | SpellDescription$ Scry 2. Draw a card.\nSVar:DBDraw:DB$ Draw | NumCards$ 1\nOracle:\n",
    "Name:Mind Sculpt Lite\nManaCost:1 U\nTypes:Sorcery\nA:SP$ Mill | ValidTgts$ Player | NumCards$ 3 | Defined$ Targeted | SpellDescription$ Target player mills three cards.\nOracle:\n",
    "Name:Impulse Lite\nManaCost:1 U\nTypes:Instant\nA:SP$ Dig | DigNum$ 4 | ChangeNum$ 1 | SpellDescription$ Look at the top four cards of your library. Put one of them into your hand and the rest on the bottom of your library.\nOracle:\n",
    "Name:Land Dig Lite\nManaCost:G\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 3 | ChangeNum$ All | ChangeValid$ Land | DestinationZone2$ Graveyard | SpellDescription$ Reveal the top three cards. Put all lands among them into your hand and the rest into your graveyard.\nOracle:\n",
    "Name:River Boa Lite\nManaCost:1 G\nTypes:Creature Snake\nPT:2/1\nA:AB$ Regenerate | Cost$ G | SpellDescription$ Regenerate CARDNAME.\nOracle:\n",
    "Name:Animate Land Lite\nManaCost:G\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Land | Power$ 3 | Toughness$ 3 | Types$ Creature,Elemental | Keywords$ Haste | Colors$ Red | OverwriteColors$ True | SpellDescription$ Until end of turn, target land becomes a 3/3 red Elemental creature with haste. It's still a land.\nOracle:\n",
    "Name:Wizard Bolt Lite\nManaCost:2 R\nTypes:Instant\nS:Mode$ ReduceCost | ValidCard$ Card.Self | Type$ Spell | Amount$ 2 | EffectZone$ All | IsPresent$ Wizard.YouCtrl | Description$ This spell costs {2} less to cast if you control a Wizard.\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ 3 damage.\nOracle:\n",
    "Name:Electromancer Lite\nManaCost:R\nTypes:Creature Goblin Wizard\nPT:1/1\nS:Mode$ ReduceCost | ValidCard$ Instant,Sorcery | Type$ Spell | Activator$ You | Amount$ 1 | Description$ Instant and sorcery spells you cast cost {1} less to cast.\nOracle:\n",
    "Name:Thalia Lite\nManaCost:1 W\nTypes:Legendary Creature Human Soldier\nPT:2/1\nK:First Strike\nS:Mode$ RaiseCost | ValidCard$ Card.nonCreature | Type$ Spell | Amount$ 1 | Description$ Noncreature spells cost {1} more to cast.\nOracle:\n",
    "Name:Mind Rot Lite\nManaCost:2 B\nTypes:Sorcery\nA:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ 2 | Defined$ Targeted | SpellDescription$ x\nOracle:\n",
];

pub const TOKENS: &[(&str, &str)] = &[
    (
        "c_a_clue_draw",
        "Name:Clue Token\nManaCost:no cost\nTypes:Artifact Clue\nA:AB$ Draw | Cost$ 2 Sac<1/CARDNAME/this token> | NumCards$ 1 | SpellDescription$ Draw a card.\nOracle:\n",
    ),
    (
        "w_1_1_soldier",
        "Name:Soldier Token\nManaCost:no cost\nColors:white\nTypes:Creature Soldier\nPT:1/1\nOracle:\n",
    ),
    (
        "g_3_3_beast",
        "Name:Beast Token\nManaCost:no cost\nColors:green\nTypes:Creature Beast\nPT:3/3\nOracle:\n",
    ),
    (
        "g_x_x_hydra",
        "Name:Hydra Token\nManaCost:no cost\nColors:green\nTypes:Creature Hydra\nPT:0/0\nOracle:\n",
    ),
    (
        "c_a_treasure_sac",
        "Name:Treasure Token\nManaCost:no cost\nTypes:Artifact Treasure\nA:AB$ Mana | Cost$ T Sac<1/CARDNAME/this token> | Produced$ Any | Amount$ 1 | SpellDescription$ Add one mana of any color.\nOracle:\n",
    ),
];

pub fn db() -> CardDb {
    let mut db = CardDb::default();
    for (name, text) in TOKENS {
        let t = db.add_token_script(name, text).expect("Token-Skript parst");
        assert!(t.is_supported(), "Token {name}: {:?}", t.unsupported);
    }
    let mut names = std::collections::BTreeSet::new();
    for s in SCRIPTS {
        let name = s.lines().next().unwrap_or("");
        assert!(names.insert(name), "doppelte Testkarte: {name}");
        let d = db.add_script(s).expect("Testskript parst");
        let pw_or_battle = d
            .face(0)
            .types
            .has_core(forge_types::CoreType::Planeswalker)
            || d.face(0).types.has_core(forge_types::CoreType::Battle);
        assert!(
            pw_or_battle || d.is_supported(),
            "{} nicht unterstützt: {:?}",
            d.name(),
            d.unsupported
        );
    }
    db
}

/// Spiel mit `lib_size` Inseln als Bibliothek je Spieler, fertig gemulliganed,
/// Zug 2, P1 aktiv, Beginn von `step`. Startspieler war P2.
pub fn game_at(step: Step, lib_size: usize) -> (CardDb, GameState) {
    let db = db();
    let island = db.get("Island").unwrap().clone();
    let decks = vec![
        ("P1".to_string(), vec![island.clone(); lib_size]),
        ("P2".to_string(), vec![island; lib_size]),
    ];
    let mut g = GameState::new(GameRules::default(), decks, 7, Some(P2));
    for p in &mut g.players {
        p.kept_hand = true;
    }
    g.token_defs = db.token_map();
    g.turn = 2;
    g.active = P1;
    g.step = step;
    g.flow = Flow::BeginStep;
    if step.is_combat() {
        g.combat = Some(Default::default());
    }
    (db, g)
}

/// Legt eine Karte in eine Zone. Auf dem Battlefield gilt sie als seit Zugbeginn kontrolliert.
pub fn put(db: &CardDb, g: &mut GameState, name: &str, p: PlayerId, z: Zone) -> CardId {
    let def = db
        .get(name)
        .unwrap_or_else(|| panic!("Karte {name} fehlt"))
        .clone();
    let id = g.create_card(def, p, z, false);
    if z == Zone::Battlefield {
        g.card_mut(id).controlled_since_turn_start = true;
    }
    id
}

pub fn put_n(
    db: &CardDb,
    g: &mut GameState,
    name: &str,
    p: PlayerId,
    z: Zone,
    n: usize,
) -> Vec<CardId> {
    (0..n).map(|_| put(db, g, name, p, z)).collect()
}

/// Aktuelles Objekt mit Namen in einer Zone (IDs ändern sich bei Zonenwechseln).
pub fn find(g: &GameState, name: &str, z: Zone) -> Option<CardId> {
    g.cards
        .iter()
        .find(|c| c.zone == Some(z) && c.def.name() == name)
        .map(|c| c.id)
}

pub fn count(g: &GameState, name: &str, z: Zone) -> usize {
    g.cards
        .iter()
        .filter(|c| c.zone == Some(z) && c.def.name() == name)
        .count()
}

/// Treiber mit Standardantworten: Passen, keine Angriffe/Blocks, AutoPay.
pub struct Driver {
    pub e: Engine,
    pub status: Status,
}

impl Driver {
    pub fn new(g: GameState) -> Driver {
        let mut e = Engine::new(g);
        let status = e.advance();
        Driver { e, status }
    }

    pub fn g(&self) -> &GameState {
        self.e.state()
    }

    pub fn decision(&self) -> &Decision {
        match &self.status {
            Status::NeedsDecision(d) => d,
            s => panic!("keine Entscheidung offen: {s:?}"),
        }
    }

    pub fn submit(&mut self, c: Choice) {
        self.status = self
            .e
            .submit(c.clone())
            .unwrap_or_else(|e| panic!("{c:?} abgelehnt bei {:?}: {e}", self.status));
    }

    pub fn try_submit(&mut self, c: Choice) -> Result<(), forge_engine::decision::InvalidChoice> {
        self.status = self.e.submit(c)?;
        Ok(())
    }

    /// Beantwortet Standardentscheidungen, bis `stop` zutrifft.
    pub fn run_until(&mut self, mut stop: impl FnMut(&GameState, &Decision) -> bool) {
        for _ in 0..10_000 {
            let d = match &self.status {
                Status::NeedsDecision(d) => d.clone(),
                Status::GameOver(_) => return,
            };
            if stop(self.e.state(), &d) {
                return;
            }
            let c = default_choice(&d);
            self.submit(c);
        }
        panic!("run_until: kein Ende");
    }

    /// Passt, bis `player` in `step` Priorität hat.
    pub fn priority_in(&mut self, step: Step, player: PlayerId) {
        self.run_until(|g, d| {
            matches!(d, Decision::Priority { player: p, .. } if *p == player) && g.step == step
        });
    }

    pub fn act(&mut self, a: Action) {
        self.submit(Choice::Action(a));
    }

    /// Wirkt eine Handkarte mit Zielen; bezahlt automatisch.
    pub fn cast(&mut self, card: CardId, targets: &[Target]) {
        self.act(Action::Cast(card));
        self.finish_casting(targets);
    }

    pub fn finish_casting(&mut self, targets: &[Target]) {
        loop {
            match self.decision().clone() {
                Decision::ChooseTargets { .. } => self.submit(Choice::Targets(targets.to_vec())),
                Decision::PayMana { .. } => self.submit(Choice::Pay(PayAction::AutoPay)),
                Decision::PayLifeForPhyrexian { .. } => self.submit(Choice::No),
                _ => return,
            }
        }
    }

    /// Alle Spieler passen einmal (Stapel löst einmal auf bzw. Schritt endet).
    pub fn pass_all(&mut self) {
        let n = self.g().players_in_game();
        for _ in 0..n {
            if matches!(self.decision(), Decision::Priority { .. }) {
                self.act(Action::Pass);
            }
        }
    }

    pub fn is_over(&self) -> Option<Option<PlayerId>> {
        match self.status {
            Status::GameOver(w) => Some(w),
            _ => None,
        }
    }
}

pub fn default_choice(d: &Decision) -> Choice {
    match d {
        Decision::Mulligan { .. } => Choice::Keep,
        Decision::MulliganBottom { hand, count, .. } => {
            Choice::Cards(hand[..*count as usize].to_vec())
        }
        Decision::Priority { .. } => Choice::Action(Action::Pass),
        Decision::DeclareAttackers { .. } => Choice::Attackers(vec![]),
        Decision::DeclareBlockers { .. } => Choice::Blocks(vec![]),
        Decision::PayMana { .. } => Choice::Pay(PayAction::AutoPay),
        Decision::PayLifeForPhyrexian { .. } => Choice::No,
        Decision::ChooseModes {
            modes,
            min,
            max,
            can_repeat,
            ..
        } => {
            let k = (*min).max(1).min(*max) as usize;
            // mit CanRepeatModes notfalls Modi wiederholen
            let pick: Vec<usize> = if *can_repeat {
                modes.iter().cycle().take(k).map(|(i, _)| *i).collect()
            } else {
                modes.iter().take(k).map(|(i, _)| *i).collect()
            };
            Choice::Modes(pick)
        }
        Decision::Scry { cards, .. } => Choice::Arrange {
            top: cards.clone(),
            bottom: vec![],
        },
        Decision::PayUnlessCost { .. } => Choice::Yes,
        Decision::OrderTriggers { descriptions, .. } => {
            Choice::Order((0..descriptions.len()).collect())
        }
        Decision::OptionalTrigger { .. } | Decision::Confirm { .. } => Choice::Yes,
        Decision::ChooseCards { options, min, .. } => {
            Choice::Cards(options[..*min as usize].to_vec())
        }
        Decision::ChooseColor { options, .. } => Choice::Color(options[0]),
        Decision::ChooseType { .. } => Choice::Number(0),
        Decision::ChooseX { .. } => Choice::Number(0),
        Decision::ChooseTargets {
            candidates, min, ..
        } => Choice::Targets(candidates[..*min as usize].to_vec()),
        Decision::AssignCombatDamage {
            amount,
            blockers,
            trample_to,
            ..
        } => {
            let mut v = vec![0; blockers.len() + usize::from(trample_to.is_some())];
            v[0] = *amount;
            Choice::Damage(v)
        }
    }
}
