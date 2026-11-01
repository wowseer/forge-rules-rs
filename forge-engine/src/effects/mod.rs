//! Typisierte Effekte (Java: `ApiType` + `*Effect.java` in `ability/effects/`).
//!
//! [`compile`] übersetzt eine [`ScriptAbility`] (rohe Parameter) einmalig beim
//! Laden in eine [`CompiledAbility`]. Nicht unterstützte ApiTypes oder
//! Parameterwerte liefern [`CompileError`] – die Karte wird dann als „nicht
//! unterstützt“ markiert (nie stilles Ignorieren).
//!
//! Die Ausführung steht in `resolve.rs`.

pub mod resolve;

use crate::cost::{Cost, CostError};
use crate::keyword::Keyword;
use crate::valid::{ValidError, ValidSpec};
use forge_script::{AbilityKind, CardFace, ScriptAbility};
use forge_types::Color;
use std::fmt;
use std::sync::Arc;

/// Zahlenwert in Parametern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Amount {
    Fixed(i32),
    /// Der für X bezahlte Wert (`Count$xPaid`).
    XPaid,
    /// Beliebiger Ausdruck (`Count$…`), siehe `expr.rs`.
    Expr(Arc<crate::expr::Expr>),
}

/// Wen/was ein Effekt betrifft (`Defined$`, Java `AbilityUtils.getDefined…`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Defined {
    You,
    SelfCard,
    Targeted,
    TargetedController,
    EachPlayer,
    EachOpponent,
    /// Trigger-Kontext (CR 603): betroffenes Objekt nach dem Ereignis.
    TriggeredCard,
    /// … vor dem Ereignis (last known information).
    TriggeredCardLki,
    TriggeredCardController,
    TriggeredPlayer,
    TriggeredTarget,
    TriggeredSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub valid: ValidSpec,
    pub min: u32,
    pub max: u32,
    pub prompt: Option<String>,
    /// Zonen, in denen Karten-Ziele liegen müssen (Standard: Battlefield).
    pub zones: Vec<crate::zone::Zone>,
}

/// Was eine Mana-Fähigkeit erzeugt (`Produced$`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManaProduce {
    /// Feste Symbole, z. B. `G` oder `R R` oder `C`. `None` = farblos.
    Fixed(Vec<Option<Color>>),
    /// Eine Farbe nach Wahl aus der Liste (`Any` = WUBRG, `Combo R G`).
    OneOf(Vec<Color>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    DealDamage {
        amount: Amount,
        to: Defined,
    },
    Draw {
        amount: Amount,
        who: Defined,
    },
    GainLife {
        amount: Amount,
        who: Defined,
    },
    LoseLife {
        amount: Amount,
        who: Defined,
    },
    Destroy {
        what: Defined,
        no_regen: bool,
    },
    Pump {
        what: Defined,
        power: Amount,
        toughness: Amount,
        keywords: Vec<Keyword>,
    },
    AddMana {
        produce: ManaProduce,
        amount: u32,
    },
    Tap {
        what: Defined,
    },
    Untap {
        what: Defined,
    },
    PutCounter {
        what: Defined,
        counter: String,
        amount: Amount,
    },
    /// Bekannte Karten (Ziel/Defined) aus `origin` nach `destination` (CR 400.7).
    ChangeZone {
        what: Defined,
        origin: Vec<crate::zone::Zone>,
        destination: crate::zone::Zone,
        place: ZonePlacement,
    },
    /// Suche in verdeckter Zone (Bibliothek/Hand) nach `change_type` (CR 701.19).
    ChangeZoneSearch {
        origin: crate::zone::Zone,
        destination: crate::zone::Zone,
        change_type: ValidSpec,
        num: Amount,
        place: ZonePlacement,
        shuffle: bool,
    },
    /// Spielsteine erzeugen (`Token`, CR 111). Skriptnamen aus `res/tokenscripts`.
    Token {
        scripts: Vec<String>,
        amount: Amount,
        owner: Defined,
        tapped: bool,
        power: Option<Amount>,
        toughness: Option<Amount>,
    },
    /// Alle passenden bleibenden Karten bis Zugende verstärken (`PumpAll`).
    PumpAll {
        valid: ValidSpec,
        power: Amount,
        toughness: Amount,
        keywords: Vec<Keyword>,
    },
    /// Alle passenden bleibenden Karten zerstören (`DestroyAll`, gleichzeitig).
    DestroyAll {
        valid: ValidSpec,
        no_regen: bool,
    },
    /// Oberste Karten ansehen, beliebig unten/oben anordnen (`Scry`, CR 701.22).
    Scry {
        who: Defined,
        num: Amount,
    },
    /// Oberste Karten in den Friedhof (`Mill`, CR 701.13).
    Mill {
        who: Defined,
        num: Amount,
    },
    /// Oberste `dig` Karten ansehen, bis zu `change` (gefiltert) nach `dest1`, Rest nach `dest2`.
    Dig {
        who: Defined,
        dig: Amount,
        change: Option<Amount>,
        valid: Option<ValidSpec>,
        dest1: crate::zone::Zone,
        dest1_bottom: bool,
        dest2: crate::zone::Zone,
        dest2_bottom: bool,
        optional: bool,
    },
    /// Regenerationsschild (`Regenerate`, CR 701.15).
    Regenerate {
        what: Defined,
    },
    /// Objekt wird (bis Zugende oder dauerhaft) zu etwas anderem (`Animate`).
    Animate {
        what: Defined,
        power: Option<Amount>,
        toughness: Option<Amount>,
        types: Vec<String>,
        remove_creature_types: bool,
        keywords: Vec<Keyword>,
        colors: Option<(forge_types::ColorSet, bool)>,
        permanent: bool,
    },
    /// Schaden an alle passenden Kreaturen/Planeswalker und Spieler (`DamageAll`).
    DamageAll {
        amount: Amount,
        valid_cards: Option<ValidSpec>,
        valid_players: Option<ValidSpec>,
    },
    /// Abwerfen (`Discard`, CR 701.9).
    Discard {
        who: Defined,
        mode: DiscardMode,
        num: Amount,
        valid: Option<ValidSpec>,
    },
    /// Zauber neutralisieren (`Counter`, CR 701.5). „Es sei denn, … zahlt“
    /// regelt das allgemeine [`UnlessSpec`] der Fähigkeit.
    Counter,
    /// Bleibende Karten opfern (`Sacrifice`, CR 701.21) bzw. mit `Destroy$` zerstören.
    Sacrifice {
        /// Spieler, die opfern (`Defined$`, Standard: du).
        who: Defined,
        /// `SacValid$`; `None` = die Quelle selbst (`Self`).
        valid: Option<ValidSpec>,
        amount: Amount,
        /// `Optional$`: der Spieler darf weniger (auch keine) wählen.
        optional: bool,
        /// `StrictAmount$`: bei zu wenigen passenden Karten wird nichts geopfert.
        strict: bool,
        /// `Destroy$`: zerstören statt opfern.
        destroy: bool,
        /// `RememberSacrificed$`: die Quelle merkt sich die geopferten Objekte.
        remember: bool,
    },
    /// Effektobjekt in der Command Zone erzeugen (`Effect`): trägt eigene
    /// Statics/Trigger/Ersatzeffekte, merkt sich Objekte, endet nach `duration`.
    CreateEffect {
        def: EffectDef,
        duration: crate::state::EffectDuration,
        remember: Option<Defined>,
    },
    /// Aura-Zauber (aus `K:Enchant`): die Aura kommt beim Auflösen an das Ziel
    /// angelegt ins Spiel (CR 303.4f) – erledigt `finish_spell`.
    AuraSpell,
    /// `Attach`: `object` (Standard: Quelle) an das Ziel anlegen (CR 701.3).
    Attach {
        object: Defined,
    },
    /// Platzhalter für modale Fähigkeiten; wird vor dem Stapeln durch die
    /// gewählten Modi ersetzt (`GameState::choose_modes`).
    Charm(CharmSpec),
    /// Alle passenden Karten der Ursprungszonen bewegen (`ChangeZoneAll`).
    ChangeZoneAll {
        origin: Vec<crate::zone::Zone>,
        destination: crate::zone::Zone,
        change_type: ValidSpec,
        place: ZonePlacement,
        shuffle: bool,
    },
}

/// Synthetische Definition eines Effektobjekts; Gleichheit = dieselbe Definition.
#[derive(Debug, Clone)]
pub struct EffectDef(pub Arc<crate::definition::CardDefinition>);

impl PartialEq for EffectDef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for EffectDef {}

/// Wer wählt beim Abwerfen?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardMode {
    /// Der abwerfende Spieler wählt (`TgtChoose`).
    PlayerChooses,
    /// Die ganze Hand (`Hand`).
    Hand,
    /// Zufällig (`Random`).
    Random,
    /// Der Controller der Fähigkeit wählt aus der aufgedeckten Hand (`RevealYouChoose`).
    YouChoose,
}

/// „… es sei denn, [Spieler] zahlt [Kosten]“ (`UnlessCost$`, CR 118.12).
///
/// Java: `AbilityUtils.handleUnlessCost`. Zahlt einer der Spieler, wird der
/// Effekt dieser Fähigkeit nicht ausgeführt (bei `switched` umgekehrt);
/// Unterfähigkeiten laufen je nach `subs` weiter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlessSpec {
    pub cost: Cost,
    /// `UnlessPayer$` (Standard: Controller des Ziels).
    pub payer: Defined,
    /// `UnlessSwitched$`: der Effekt passiert nur, *wenn* bezahlt wird.
    pub switched: bool,
    pub subs: UnlessSubs,
}

/// `UnlessResolveSubs$`: wann Unterfähigkeiten noch ausgeführt werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlessSubs {
    Always,
    WhenPaid,
    WhenNotPaid,
}

/// Modaler Effekt (`Charm`, CR 700.2): Modi werden beim Wirken/Stapeln gewählt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmSpec {
    pub choices: Vec<Arc<CompiledAbility>>,
    pub num: u32,
    pub min: u32,
    pub can_repeat: bool,
}

/// Wie ein Objekt in der Zielzone ankommt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZonePlacement {
    /// `Tapped$ True` (nur Battlefield)
    pub tapped: bool,
    /// `GainControl$ True`: Controller der Fähigkeit statt Besitzer (nur Battlefield)
    pub gain_control: bool,
    /// `LibraryPosition$`: 0 = oben, -1 = unten (nur Bibliothek)
    pub library_bottom: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledAbility {
    pub kind: AbilityKind,
    pub api: String,
    pub cost: Option<Cost>,
    pub target: Option<TargetSpec>,
    pub effect: Effect,
    pub sub: Option<Arc<CompiledAbility>>,
    pub description: String,
    /// `SorcerySpeed$ True` bei aktivierten Fähigkeiten.
    pub sorcery_speed: bool,
    /// `ConditionPresent$`, `ConditionCheckSVar$` …: nur ausführen, wenn erfüllt.
    pub conditions: Vec<crate::expr::Condition>,
    /// `UnlessCost$`: „… es sei denn, … zahlt“.
    pub unless: Option<UnlessSpec>,
}

impl CompiledAbility {
    pub fn is_mana_ability(&self) -> bool {
        // CR 605.1a: kein Ziel, erzeugt Mana, keine Loyalitätsfähigkeit.
        self.kind == AbilityKind::Activated
            && matches!(self.effect, Effect::AddMana { .. })
            && self.target.is_none()
    }

    /// Alle Fähigkeiten der Kette (diese + SubAbilities), in Auflösungsreihenfolge.
    pub fn chain(&self) -> Vec<&CompiledAbility> {
        let mut v = vec![self];
        let mut cur = self;
        while let Some(s) = &cur.sub {
            v.push(s);
            cur = s;
        }
        v
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    UnsupportedApi(String),
    UnsupportedParam {
        api: String,
        key: String,
        value: String,
    },
    MissingParam {
        api: String,
        key: &'static str,
    },
    MissingSvar(String),
    Cost(CostError),
    Valid(ValidError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::UnsupportedApi(a) => write!(f, "Effekt nicht unterstützt: {a}"),
            CompileError::UnsupportedParam { api, key, value } => {
                write!(f, "{api}: {key}$ {value} nicht unterstützt")
            }
            CompileError::MissingParam { api, key } => {
                write!(f, "{api}: Pflichtparameter {key}$ fehlt")
            }
            CompileError::MissingSvar(s) => write!(f, "SVar {s} fehlt"),
            CompileError::Cost(e) => write!(f, "{e}"),
            CompileError::Valid(e) => write!(f, "{e}"),
        }
    }
}

impl From<CostError> for CompileError {
    fn from(e: CostError) -> Self {
        CompileError::Cost(e)
    }
}

impl From<ValidError> for CompileError {
    fn from(e: ValidError) -> Self {
        CompileError::Valid(e)
    }
}

/// Parameter, die das Verhalten nicht ändern (Texte, KI-Hinweise) und daher
/// ignoriert werden dürfen. Alle anderen unbekannten Parameter sind Fehler.
const NEUTRAL_PARAMS: &[&str] = &[
    "SP",
    "AB",
    "DB",
    "SpellDescription",
    "StackDescription",
    "TgtPrompt",
    "AILogic",
    "AITgts",
    "AIPreference",
    "PrecostDesc",
    "CostDesc",
    "PlayerTurn",
    "IsCurse",
    "AiAttackController",
    "AIManaPref",
    "IsPresent",
];

struct Ctx<'a> {
    face: &'a CardFace,
    ab: &'a ScriptAbility,
    used: Vec<&'static str>,
}

impl<'a> Ctx<'a> {
    fn get(&mut self, key: &'static str) -> Option<&'a str> {
        self.used.push(key);
        self.ab.params.get(key)
    }

    fn unsupported(&self, key: &str, value: &str) -> CompileError {
        CompileError::UnsupportedParam {
            api: self.ab.api.clone(),
            key: key.into(),
            value: value.into(),
        }
    }

    fn amount(&mut self, key: &'static str, default: Option<i32>) -> Result<Amount, CompileError> {
        match self.get(key) {
            None => default
                .map(Amount::Fixed)
                .ok_or(CompileError::MissingParam {
                    api: self.ab.api.clone(),
                    key,
                }),
            Some(v) => self.amount_value(key, v),
        }
    }

    fn amount_value(&self, key: &str, v: &str) -> Result<Amount, CompileError> {
        use crate::expr::Expr;
        match Expr::parse_value(self.face, v).map_err(|_| self.unsupported(key, v))? {
            Expr::Const(n) => Ok(Amount::Fixed(n)),
            Expr::XPaid => Ok(Amount::XPaid),
            e => Ok(Amount::Expr(Arc::new(e))),
        }
    }

    fn defined(&mut self, default: Defined) -> Result<Defined, CompileError> {
        self.defined_key("Defined", default)
    }

    /// Wie `defined`, aber für einen anderen Schlüssel (z. B. `TokenOwner$`).
    fn defined_key(
        &mut self,
        key: &'static str,
        default: Defined,
    ) -> Result<Defined, CompileError> {
        let has_tgt = self.ab.params.has("ValidTgts");
        match self.get(key) {
            None if has_tgt => Ok(Defined::Targeted),
            // Java (AbilityUtils.getDefinedCards): ohne Defined$ ist „Self“ gemeint.
            None if default == Defined::Targeted => Ok(Defined::SelfCard),
            None => Ok(default),
            Some(v) => Ok(match v {
                "You" => Defined::You,
                "Self" => Defined::SelfCard,
                "Targeted" | "TargetedPlayer" | "TargetedCard" | "ParentTarget" => {
                    Defined::Targeted
                }
                "TargetedController" => Defined::TargetedController,
                "Player" => Defined::EachPlayer,
                "Player.Opponent" | "Opponent" => Defined::EachOpponent,
                "TriggeredCard" | "TriggeredNewCardLKICopy" | "TriggeredAttacker" => {
                    Defined::TriggeredCard
                }
                "TriggeredCardLKICopy" | "TriggeredAttackerLKICopy" => Defined::TriggeredCardLki,
                "TriggeredCardController" | "TriggeredAttackerController" => {
                    Defined::TriggeredCardController
                }
                "TriggeredPlayer" => Defined::TriggeredPlayer,
                "TriggeredTarget" => Defined::TriggeredTarget,
                "TriggeredSource" | "TriggeredSourceLKICopy" => Defined::TriggeredSource,
                other => return Err(self.unsupported(key, other)),
            }),
        }
    }
}

fn parse_produced(s: &str) -> Option<ManaProduce> {
    let s = s.trim();
    if s == "Any" {
        return Some(ManaProduce::OneOf(Color::ALL.to_vec()));
    }
    if let Some(rest) = s.strip_prefix("Combo ") {
        if rest.trim() == "Any" {
            return None; // Kombinationen mehrerer Farben: später
        }
        let cols: Option<Vec<Color>> = rest
            .split_whitespace()
            .map(|t| t.chars().next().and_then(Color::from_symbol))
            .collect();
        return cols.map(ManaProduce::OneOf);
    }
    let syms: Option<Vec<Option<Color>>> = s
        .split_whitespace()
        .map(|t| match t {
            "C" => Some(None),
            t if t.len() == 1 => t.chars().next().and_then(Color::from_symbol).map(Some),
            _ => None,
        })
        .collect();
    syms.map(ManaProduce::Fixed)
}

fn parse_zones(
    c: &Ctx,
    key: &'static str,
    v: &str,
) -> Result<Vec<crate::zone::Zone>, CompileError> {
    v.split(',')
        .map(|z| crate::zone::Zone::parse(z).ok_or_else(|| c.unsupported(key, v)))
        .collect()
}

fn compile_change_zone(c: &mut Ctx, api: &str) -> Result<Effect, CompileError> {
    use crate::zone::Zone;
    let origin_s = c.get("Origin").ok_or(CompileError::MissingParam {
        api: api.into(),
        key: "Origin",
    })?;
    let origin = parse_zones(c, "Origin", origin_s)?;
    let dest_s = c.get("Destination").ok_or(CompileError::MissingParam {
        api: api.into(),
        key: "Destination",
    })?;
    let destination = match parse_zones(c, "Destination", dest_s)?.as_slice() {
        [z] => *z,
        _ => return Err(c.unsupported("Destination", dest_s)),
    };
    let place = ZonePlacement {
        tapped: c
            .get("Tapped")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        gain_control: c
            .get("GainControl")
            .is_some_and(|v| v.eq_ignore_ascii_case("true")),
        library_bottom: match c.get("LibraryPosition") {
            None | Some("0") => false,
            Some("-1") => true,
            Some(v) => return Err(c.unsupported("LibraryPosition", v)),
        },
    };
    // reine Text-/Hinweisparameter
    for k in [
        "ChangeTypeDesc",
        "SelectPrompt",
        "Mandatory",
        "ShuffleNonMandatory",
    ] {
        c.get(k);
    }
    let shuffle_param = c.get("Shuffle").map(|v| v.eq_ignore_ascii_case("true"));
    if api == "ChangeZoneAll" {
        let ct = c.get("ChangeType").ok_or(CompileError::MissingParam {
            api: api.into(),
            key: "ChangeType",
        })?;
        let shuffle = shuffle_param
            .unwrap_or(origin.contains(&Zone::Library) && destination != Zone::Library);
        return Ok(Effect::ChangeZoneAll {
            origin,
            destination,
            change_type: ValidSpec::parse(ct)?,
            place,
            shuffle,
        });
    }
    let hidden = c
        .get("Hidden")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        || (origin.iter().all(|z| z.is_hidden())
            && !c.ab.params.has("ValidTgts")
            && !c.ab.params.has("Defined"));
    if hidden {
        let [o] = origin.as_slice() else {
            return Err(c.unsupported("Origin", origin_s));
        };
        let ct = c.get("ChangeType").unwrap_or("Card");
        let num = c.amount("ChangeNum", Some(1))?;
        let shuffle = shuffle_param.unwrap_or(*o == Zone::Library);
        return Ok(Effect::ChangeZoneSearch {
            origin: *o,
            destination,
            change_type: ValidSpec::parse(ct)?,
            num,
            place,
            shuffle,
        });
    }
    if let Some(s) = shuffle_param
        && s
    {
        return Err(c.unsupported("Shuffle", "True"));
    }
    let what = c.defined(Defined::Targeted)?;
    Ok(Effect::ChangeZone {
        what,
        origin,
        destination,
        place,
    })
}

fn compile_effect_object(c: &mut Ctx, face: &CardFace) -> Result<Effect, CompileError> {
    use crate::state::EffectDuration;
    let list = |c: &mut Ctx, key: &'static str| -> Vec<String> {
        c.get(key)
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    let statics_n = list(c, "StaticAbilities");
    let triggers_n = list(c, "Triggers");
    let repl_n = list(c, "ReplacementEffects");
    let mut compiled = crate::definition::CompiledFace {
        keywords: Vec::new(),
        spell: None,
        activated: Vec::new(),
        triggers: Vec::new(),
        statics: Vec::new(),
        replacements: Vec::new(),
        etb_counters: Vec::new(),
        enchant: None,
    };
    let svar = |n: &str| {
        face.svar(n)
            .ok_or_else(|| CompileError::MissingSvar(n.to_string()))
    };
    for n in &statics_n {
        let st = forge_script::ScriptStatic::parse(forge_script::ParamMap::parse(svar(n)?))
            .map_err(|_| CompileError::MissingSvar(n.clone()))?;
        for mut cs in crate::statics::compile_static_multi(face, &st)? {
            // Effektobjekte wirken aus der Command Zone.
            *cs.effect_zones_mut() = vec![crate::zone::Zone::Command];
            compiled.statics.push(Arc::new(cs));
        }
    }
    for n in &triggers_n {
        let t = forge_script::ScriptTrigger::parse(forge_script::ParamMap::parse(svar(n)?))
            .map_err(|_| CompileError::MissingSvar(n.clone()))?;
        let mut ct = crate::triggers::compile_trigger(face, &t)?;
        // Effektobjekte liegen in der Command Zone.
        ct.active_zones = vec![crate::zone::Zone::Command];
        compiled.triggers.push(Arc::new(ct));
    }
    for n in &repl_n {
        let r = forge_script::ScriptReplacement::parse(forge_script::ParamMap::parse(svar(n)?))
            .map_err(|_| CompileError::MissingSvar(n.clone()))?;
        compiled
            .replacements
            .push(Arc::new(crate::replacements::compile_replacement(
                face, &r,
            )?));
    }
    let duration = match c.get("Duration") {
        None => EffectDuration::EndOfTurn,
        Some("Permanent") => EffectDuration::Permanent,
        Some("UntilYourNextTurn") => EffectDuration::UntilYourNextTurn,
        Some("UntilTheEndOfYourNextTurn") => EffectDuration::UntilTheEndOfYourNextTurn,
        Some(v) => return Err(c.unsupported("Duration", v)),
    };
    let remember = match c.ab.params.get("RememberObjects") {
        Some(_) => Some(c.defined_key("RememberObjects", Defined::SelfCard)?),
        None => None,
    };
    // Bei Verlassen der Zone ändert sich die ID (CR 400.7) – gemerkte Objekte
    // passen dann ohnehin nicht mehr; daher genügt das hier als Vereinfachung.
    for k in [
        "ExileOnMoved",
        "ForgetOnMoved",
        "Name",
        "Image",
        "EffectOwner",
    ] {
        if k == "EffectOwner" && c.ab.params.has(k) {
            return Err(c.unsupported(k, c.ab.params.get(k).unwrap_or("")));
        }
        c.get(k);
    }
    let name = c.ab.params.get("Name").unwrap_or("Effekt").to_string();
    let effect_face = CardFace {
        name,
        ..CardFace::default()
    };
    let script = forge_script::CardScript {
        split_mode: forge_script::SplitMode::None,
        faces: vec![Some(effect_face)],
        variants: Vec::new(),
        meld_with: None,
        partner_with: None,
        hand_life_modifier: None,
        tokens: Vec::new(),
        ai_hints: Vec::new(),
        deck_hints: Vec::new(),
        copy_face_from: Vec::new(),
    };
    let def = crate::definition::CardDefinition {
        script,
        faces: vec![Some(compiled)],
        unsupported: Vec::new(),
    };
    Ok(Effect::CreateEffect {
        def: EffectDef(Arc::new(def)),
        duration,
        remember,
    })
}

/// Übersetzt eine Fähigkeit samt `SubAbility$`-Kette.
pub fn compile(face: &CardFace, ab: &ScriptAbility) -> Result<CompiledAbility, CompileError> {
    compile_depth(face, ab, 0)
}

fn compile_depth(
    face: &CardFace,
    ab: &ScriptAbility,
    depth: usize,
) -> Result<CompiledAbility, CompileError> {
    let mut c = Ctx {
        face,
        ab,
        used: Vec::new(),
    };
    let api = ab.api.as_str();

    let cost = match c.get("Cost") {
        Some(s) => Some(Cost::parse(s)?),
        None => None,
    };
    let target = match c.get("ValidTgts") {
        Some(v) => {
            let min = match c.get("TargetMin") {
                Some(s) => s.parse().map_err(|_| c.unsupported("TargetMin", s))?,
                None => 1,
            };
            let max = match c.get("TargetMax") {
                Some(s) => s.parse().map_err(|_| c.unsupported("TargetMax", s))?,
                None => 1,
            };
            // Ziele außerhalb des Battlefields: TgtZone$, bei ChangeZone die Origin-Zone.
            let zone_param = if c.ab.params.get("TargetType") == Some("Spell") {
                Some("Stack")
            } else {
                c.get("TgtZone")
            };
            let zone_param = zone_param.or_else(|| {
                if api == "ChangeZone" {
                    c.ab.params.get("Origin")
                } else {
                    None
                }
            });
            let zones = match zone_param {
                Some(z) => z
                    .split(',')
                    .map(|x| crate::zone::Zone::parse(x).ok_or_else(|| c.unsupported("TgtZone", z)))
                    .collect::<Result<Vec<_>, _>>()?,
                None => vec![crate::zone::Zone::Battlefield],
            };
            Some(TargetSpec {
                valid: ValidSpec::parse(v)?,
                min,
                max,
                prompt: c.get("TgtPrompt").map(str::to_string),
                zones,
            })
        }
        None => None,
    };
    let sorcery_speed = c
        .get("SorcerySpeed")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    let description = c
        .get("SpellDescription")
        .or(c.ab.params.get("StackDescription"))
        .unwrap_or("")
        .to_string();

    let effect = match api {
        "DealDamage" => {
            let amount = c.amount("NumDmg", None)?;
            let to = c.defined(Defined::Targeted)?;
            Effect::DealDamage { amount, to }
        }
        "Draw" => {
            let amount = c.amount("NumCards", Some(1))?;
            let who = c.defined(Defined::You)?;
            Effect::Draw { amount, who }
        }
        "GainLife" => {
            let amount = c.amount("LifeAmount", None)?;
            let who = c.defined(Defined::You)?;
            Effect::GainLife { amount, who }
        }
        "LoseLife" => {
            let amount = c.amount("LifeAmount", None)?;
            let who = c.defined(Defined::You)?;
            Effect::LoseLife { amount, who }
        }
        "Destroy" => {
            let what = c.defined(Defined::Targeted)?;
            let no_regen = c
                .get("NoRegen")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::Destroy { what, no_regen }
        }
        "Pump" => {
            let what = c.defined(Defined::SelfCard)?;
            let power = c.amount("NumAtt", Some(0))?;
            let toughness = c.amount("NumDef", Some(0))?;
            let keywords = c.get("KW").map(Keyword::parse_list).unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("KW", &format!("{k:?}")));
            }
            Effect::Pump {
                what,
                power,
                toughness,
                keywords,
            }
        }
        "Mana" => {
            let produced = c.get("Produced").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "Produced",
            })?;
            let produce =
                parse_produced(produced).ok_or_else(|| c.unsupported("Produced", produced))?;
            let amount = match c.get("Amount") {
                Some(s) => s.parse().map_err(|_| c.unsupported("Amount", s))?,
                None => 1,
            };
            Effect::AddMana { produce, amount }
        }
        "Tap" => Effect::Tap {
            what: c.defined(Defined::Targeted)?,
        },
        "Untap" => Effect::Untap {
            what: c.defined(Defined::Targeted)?,
        },
        "PutCounter" => {
            let what = c.defined(Defined::SelfCard)?;
            let counter = c.get("CounterType").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "CounterType",
            })?;
            let amount = c.amount("CounterNum", Some(1))?;
            Effect::PutCounter {
                what,
                counter: counter.to_string(),
                amount,
            }
        }
        "ChangeZone" | "ChangeZoneAll" => compile_change_zone(&mut c, api)?,
        "PumpAll" => {
            let valid =
                ValidSpec::parse(c.get("ValidCards").ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "ValidCards",
                })?)?;
            let power = c.amount("NumAtt", Some(0))?;
            let toughness = c.amount("NumDef", Some(0))?;
            let keywords = c.get("KW").map(Keyword::parse_list).unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("KW", &format!("{k:?}")));
            }
            Effect::PumpAll {
                valid,
                power,
                toughness,
                keywords,
            }
        }
        "DestroyAll" => {
            let valid =
                ValidSpec::parse(c.get("ValidCards").ok_or(CompileError::MissingParam {
                    api: api.into(),
                    key: "ValidCards",
                })?)?;
            let no_regen = c
                .get("NoRegen")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            Effect::DestroyAll { valid, no_regen }
        }
        "DamageAll" => {
            let amount = c.amount("NumDmg", None)?;
            let valid_cards = c.get("ValidCards").map(ValidSpec::parse).transpose()?;
            let valid_players = c.get("ValidPlayers").map(ValidSpec::parse).transpose()?;
            c.get("ValidDescription");
            Effect::DamageAll {
                amount,
                valid_cards,
                valid_players,
            }
        }
        "Discard" => {
            let mode = match c.get("Mode").unwrap_or("TgtChoose") {
                "TgtChoose" => DiscardMode::PlayerChooses,
                "Hand" => DiscardMode::Hand,
                "Random" => DiscardMode::Random,
                "RevealYouChoose" => DiscardMode::YouChoose,
                other => return Err(c.unsupported("Mode", other)),
            };
            let num = c.amount("NumCards", Some(1))?;
            let valid = c.get("DiscardValid").map(ValidSpec::parse).transpose()?;
            let who = c.defined(Defined::You)?;
            Effect::Discard {
                who,
                mode,
                num,
                valid,
            }
        }
        "Counter" => {
            match c.get("TargetType") {
                Some("Spell") | None => {}
                Some(t) => return Err(c.unsupported("TargetType", t)),
            }
            if c.ab.params.has("Defined") {
                return Err(c.unsupported("Defined", c.ab.params.get("Defined").unwrap_or("")));
            }
            Effect::Counter
        }
        "Sacrifice" => {
            let valid = match c.get("SacValid").map(str::trim) {
                None | Some("Self") => None,
                Some(v) => Some(ValidSpec::parse(v)?),
            };
            let amount = c.amount("Amount", Some(1))?;
            let flag = |c: &mut Ctx, k: &'static str| c.get(k).is_some();
            let optional = flag(&mut c, "Optional");
            let strict = flag(&mut c, "StrictAmount");
            let destroy = flag(&mut c, "Destroy");
            let remember = flag(&mut c, "RememberSacrificed");
            c.get("SacMessage");
            let who = c.defined(Defined::You)?;
            Effect::Sacrifice {
                who,
                valid,
                amount,
                optional,
                strict,
                destroy,
                remember,
            }
        }
        "Effect" => compile_effect_object(&mut c, face)?,
        "Scry" => {
            let num = c.amount("ScryNum", Some(1))?;
            Effect::Scry {
                who: c.defined(Defined::You)?,
                num,
            }
        }
        "Mill" => {
            let num = c.amount("NumCards", Some(1))?;
            Effect::Mill {
                who: c.defined(Defined::You)?,
                num,
            }
        }
        "Dig" => {
            use crate::zone::Zone;
            let dig = c.amount("DigNum", None)?;
            let change = match c.get("ChangeNum") {
                Some("All") => None,
                Some(v) => Some(c.amount_value("ChangeNum", v)?),
                None => Some(Amount::Fixed(1)),
            };
            let valid = c.get("ChangeValid").map(ValidSpec::parse).transpose()?;
            let d1 = c.get("DestinationZone");
            let dest1 = match d1 {
                Some(z) => Zone::parse(z).ok_or_else(|| c.unsupported("DestinationZone", z))?,
                None => Zone::Hand,
            };
            let d2 = c.get("DestinationZone2");
            let dest2 = match d2 {
                Some(z) => Zone::parse(z).ok_or_else(|| c.unsupported("DestinationZone2", z))?,
                None => Zone::Library,
            };
            let mut pos = |k: &'static str| -> Result<bool, CompileError> {
                match c.get(k) {
                    None | Some("-1") => Ok(true),
                    Some("0") => Ok(false),
                    Some(v) => Err(c.unsupported(k, v)),
                }
            };
            let dest1_bottom = pos("LibraryPosition")?;
            let dest2_bottom = pos("LibraryPosition2")?;
            let optional = c
                .get("Optional")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            for k in [
                "Reveal",
                "NoReveal",
                "ForceRevealToController",
                "RestRandomOrder",
                "ChangeValidDesc",
                "PrimaryPrompt",
            ] {
                c.get(k);
            }
            Effect::Dig {
                who: c.defined(Defined::You)?,
                dig,
                change,
                valid,
                dest1,
                dest1_bottom,
                dest2,
                dest2_bottom,
                optional,
            }
        }
        "Regenerate" => Effect::Regenerate {
            what: c.defined(Defined::SelfCard)?,
        },
        "Animate" => {
            let what = c.defined(Defined::SelfCard)?;
            let power = match c.get("Power") {
                Some(v) => Some(c.amount_value("Power", v)?),
                None => None,
            };
            let toughness = match c.get("Toughness") {
                Some(v) => Some(c.amount_value("Toughness", v)?),
                None => None,
            };
            let types: Vec<String> = c
                .get("Types")
                .map(|t| {
                    t.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            if types
                .iter()
                .any(|t| t.contains(' ') || t.starts_with("All") || t.starts_with("Chosen"))
            {
                return Err(c.unsupported("Types", c.ab.params.get("Types").unwrap_or("")));
            }
            let remove_creature_types = c
                .get("RemoveCreatureTypes")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let keywords = c
                .get("Keywords")
                .map(Keyword::parse_list)
                .unwrap_or_default();
            if let Some(k) = keywords.iter().find(|k| !k.is_supported()) {
                return Err(c.unsupported("Keywords", &format!("{k:?}")));
            }
            let overwrite = c
                .get("OverwriteColors")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let colors = match c.get("Colors") {
                Some(v) => Some((
                    forge_types::ColorSet::from_names(v)
                        .ok_or_else(|| c.unsupported("Colors", v))?,
                    overwrite,
                )),
                None => None,
            };
            let permanent = match c.get("Duration") {
                None => false,
                Some("Permanent") => true,
                Some(v) => return Err(c.unsupported("Duration", v)),
            };
            Effect::Animate {
                what,
                power,
                toughness,
                types,
                remove_creature_types,
                keywords,
                colors,
                permanent,
            }
        }
        "Attach" => {
            if c.ab.params.get("ValidTgts").is_none() {
                return Err(c.unsupported("Attach", "ohne ValidTgts"));
            }
            let object = c.defined_key("Object", Defined::SelfCard)?;
            Effect::Attach { object }
        }
        "Token" => {
            let scripts_s = c.get("TokenScript").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "TokenScript",
            })?;
            let scripts = scripts_s
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            let amount = c.amount("TokenAmount", Some(1))?;
            let owner = c.defined_key("TokenOwner", Defined::You)?;
            let tapped = c
                .get("TokenTapped")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let power = match c.get("TokenPower") {
                Some(v) => Some(c.amount_value("TokenPower", v)?),
                None => None,
            };
            let toughness = match c.get("TokenToughness") {
                Some(v) => Some(c.amount_value("TokenToughness", v)?),
                None => None,
            };
            Effect::Token {
                scripts,
                amount,
                owner,
                tapped,
                power,
                toughness,
            }
        }
        "Charm" => {
            // Modi werden beim Wirken gewählt; als Unterfähigkeit nicht unterstützt.
            if depth > 0 {
                return Err(c.unsupported("Charm", "als Unterfähigkeit"));
            }
            let choices_s = c.get("Choices").ok_or(CompileError::MissingParam {
                api: api.into(),
                key: "Choices",
            })?;
            let mut choices = Vec::new();
            for name in choices_s.split(',').map(str::trim) {
                let sv = face
                    .svar(name)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
                let sub_ab = ScriptAbility::from_svar(sv)
                    .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                    .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
                choices.push(Arc::new(compile_depth(face, &sub_ab, depth + 1)?));
            }
            let num = match c.get("CharmNum") {
                Some(v) => v.trim().parse().map_err(|_| c.unsupported("CharmNum", v))?,
                None => 1,
            };
            let min = match c.get("MinCharmNum") {
                Some(v) => v
                    .trim()
                    .parse()
                    .map_err(|_| c.unsupported("MinCharmNum", v))?,
                None => num,
            };
            let can_repeat = c
                .get("CanRepeatModes")
                .is_some_and(|v| v.eq_ignore_ascii_case("true"));
            c.get("AdditionalDescription");
            Effect::Charm(CharmSpec {
                choices,
                num,
                min,
                can_repeat,
            })
        }
        other => return Err(CompileError::UnsupportedApi(other.to_string())),
    };

    let sub = match c.get("SubAbility") {
        Some(name) if depth < 32 => {
            let sv = face
                .svar(name)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?;
            let sub_ab = ScriptAbility::from_svar(sv)
                .ok_or_else(|| CompileError::MissingSvar(name.to_string()))?
                .map_err(|_| CompileError::MissingSvar(name.to_string()))?;
            Some(Arc::new(compile_depth(face, &sub_ab, depth + 1)?))
        }
        Some(name) => return Err(c.unsupported("SubAbility", name)),
        None => None,
    };

    let unless = compile_unless(&mut c)?;

    let (conditions, cond_keys) =
        crate::expr::parse_conditions(face, &ab.params, crate::expr::CondStyle::Effect)?;
    c.used.extend(cond_keys);

    // Alle übrigen Parameter müssen neutral sein.
    for (k, v) in ab.params.iter() {
        let known = c.used.iter().any(|u| u.eq_ignore_ascii_case(k))
            || NEUTRAL_PARAMS.iter().any(|n| n.eq_ignore_ascii_case(k));
        if !known {
            return Err(c.unsupported(k, v));
        }
    }

    Ok(CompiledAbility {
        kind: ab.kind,
        api: ab.api.clone(),
        cost,
        target,
        effect,
        sub,
        description,
        sorcery_speed,
        conditions,
        unless,
    })
}

/// `UnlessCost$`/`UnlessPayer$`/`UnlessSwitched$`/`UnlessResolveSubs$`.
fn compile_unless(c: &mut Ctx) -> Result<Option<UnlessSpec>, CompileError> {
    let Some(cost_s) = c.get("UnlessCost") else {
        return Ok(None);
    };
    let cost = Cost::parse(cost_s.trim())?;
    // {T}/{Q} beziehen sich auf die Quelle und ergeben als „Unless“-Kosten keinen Sinn.
    if cost
        .parts
        .iter()
        .any(|p| matches!(p, crate::cost::CostPart::Tap | crate::cost::CostPart::Untap))
        || cost.mana.as_ref().is_some_and(|m| m.x_count() > 0)
    {
        return Err(c.unsupported("UnlessCost", cost_s));
    }
    let payer = if c.ab.params.has("UnlessPayer") {
        c.defined_key("UnlessPayer", Defined::TargetedController)?
    } else {
        Defined::TargetedController
    };
    let switched = c.get("UnlessSwitched").is_some();
    let subs = match c.get("UnlessResolveSubs") {
        None => UnlessSubs::Always,
        Some("WhenPaid") => UnlessSubs::WhenPaid,
        Some("WhenNotPaid") => UnlessSubs::WhenNotPaid,
        Some(v) => return Err(c.unsupported("UnlessResolveSubs", v)),
    };
    // Weitere Unless-Parameter (z. B. `UnlessAI$`) sind KI-Hinweise.
    c.get("UnlessAI");
    Ok(Some(UnlessSpec {
        cost,
        payer,
        switched,
        subs,
    }))
}

impl CompiledAbility {
    /// Hängt `next` als letzte Unterfähigkeit an eine Kopie von `self` an.
    pub fn append(&self, next: Arc<CompiledAbility>) -> CompiledAbility {
        let mut me = self.clone();
        me.sub = Some(match &self.sub {
            Some(s) => Arc::new(s.append(next)),
            None => next,
        });
        me
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_script::CardScript;
    use forge_types::TypeRegistry;

    fn first(txt: &str) -> Result<CompiledAbility, CompileError> {
        let c = CardScript::parse(txt, &TypeRegistry::default()).unwrap();
        compile(c.main(), &c.main().abilities[0])
    }

    #[test]
    fn bolt_and_x() {
        let a = first("Name:B\nManaCost:R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ x\n").unwrap();
        assert_eq!(
            a.effect,
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                to: Defined::Targeted
            }
        );
        let x = first("Name:F\nManaCost:X R\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X\nSVar:X:Count$xPaid\n").unwrap();
        assert_eq!(
            x.effect,
            Effect::DealDamage {
                amount: Amount::XPaid,
                to: Defined::Targeted
            }
        );
    }

    #[test]
    fn subability_chain() {
        let a = first("Name:C\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | SubAbility$ DBGain\nSVar:DBGain:DB$ GainLife | LifeAmount$ 2\n").unwrap();
        assert_eq!(a.chain().len(), 2);
    }

    #[test]
    fn unknown_param_rejected() {
        let e = first("Name:C\nManaCost:1 U\nTypes:Instant\nA:SP$ Draw | NumCards$ 1 | Weird$ 3\n")
            .unwrap_err();
        assert!(matches!(e, CompileError::UnsupportedParam { .. }));
    }

    #[test]
    fn mana_ability() {
        let a = first("Name:E\nManaCost:G\nTypes:Creature Elf\nPT:1/1\nA:AB$ Mana | Cost$ T | Produced$ G | SpellDescription$ Add {G}.\n").unwrap();
        assert!(a.is_mana_ability());
    }
}
