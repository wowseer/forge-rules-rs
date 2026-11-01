//! Testgerüst: kleine Kartenbasis, Spiel direkt in einem Schritt aufsetzen,
//! Entscheidungen beantworten.
#![allow(dead_code)]

use forge_engine::decision::{Action, Choice, Decision, PayAction, Target};
use forge_engine::state::Flow;
use forge_engine::{CardDb, CardId, Engine, GameRules, GameState, PlayerId, Status, Step, Zone};

pub const P1: PlayerId = PlayerId(0);
pub const P2: PlayerId = PlayerId(1);

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
    "Name:Mana Leak\nManaCost:1 U\nTypes:Instant\nA:SP$ Counter | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card | UnlessCost$ 3 | SpellDescription$ Counter target spell unless its controller pays {3}.\nOracle:\n",
    "Name:Essence Scatter Lite\nManaCost:1 U\nTypes:Instant\nA:SP$ Counter | TargetType$ Spell | ValidTgts$ Creature | TgtPrompt$ Select target creature spell | SpellDescription$ Counter target creature spell.\nOracle:\n",
    "Name:Mind Rot Lite\nManaCost:2 B\nTypes:Sorcery\nA:SP$ LoseLife | ValidTgts$ Player | LifeAmount$ 2 | Defined$ Targeted | SpellDescription$ x\nOracle:\n",
];

pub const TOKENS: &[(&str, &str)] = &[
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
    for s in SCRIPTS {
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
        Decision::PayUnlessCost { .. } => Choice::Yes,
        Decision::OrderTriggers { descriptions, .. } => {
            Choice::Order((0..descriptions.len()).collect())
        }
        Decision::OptionalTrigger { .. } => Choice::Yes,
        Decision::ChooseCards { options, min, .. } => {
            Choice::Cards(options[..*min as usize].to_vec())
        }
        Decision::ChooseColor { options, .. } => Choice::Color(options[0]),
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
