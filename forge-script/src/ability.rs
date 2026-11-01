//! Klassifikation der Fähigkeitszeilen `A:`, `T:`, `S:`, `R:` und von
//! Fähigkeits-SVars.
//!
//! Java-Vorbilder: `AbilityFactory` (A/SVar mit `SP$`/`AB$`/`DB$`/`ST$`),
//! `TriggerHandler.parseTrigger` (`Mode$`), `StaticAbility` (`Mode$`),
//! `ReplacementHandler.parseReplacement` (`Event$`).
//!
//! Hier wird noch **nicht** in typisierte Effekte übersetzt – das passiert
//! im Effekt-Compiler der Engine. Dieses Modul garantiert nur, dass jede
//! Zeile die Pflicht-Schlüssel hat.

use crate::params::ParamMap;
use std::fmt;

/// Art einer Spell-Ability (Java: `AbilityRecordType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AbilityKind {
    /// `SP$` – Zauberspruch-Fähigkeit (CR 113.3a)
    Spell,
    /// `AB$` – aktivierte Fähigkeit (CR 113.3b)
    Activated,
    /// `DB$` – Unterfähigkeit (Teil einer Kette über `SubAbility$`)
    Sub,
    /// `ST$` – statisch aktivierbare Sonderaktion (nur Circling Vultures)
    StaticActivated,
}

impl AbilityKind {
    pub fn from_key(k: &str) -> Option<AbilityKind> {
        match k {
            "SP" => Some(AbilityKind::Spell),
            "AB" => Some(AbilityKind::Activated),
            "DB" => Some(AbilityKind::Sub),
            "ST" => Some(AbilityKind::StaticActivated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbilityError {
    /// Fähigkeitszeile beginnt nicht mit `SP$`/`AB$`/`DB$`/`ST$`.
    NoApi(String),
    /// Trigger/Static ohne `Mode$`, Replacement ohne `Event$`.
    MissingKey { key: &'static str, line: String },
}

impl fmt::Display for AbilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AbilityError::NoApi(l) => write!(f, "Fähigkeit ohne SP$/AB$/DB$: {l}"),
            AbilityError::MissingKey { key, line } => {
                write!(f, "fehlender Schlüssel {key}$: {line}")
            }
        }
    }
}

impl std::error::Error for AbilityError {}

/// Eine Fähigkeit (`A:` oder Fähigkeits-SVar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptAbility {
    pub kind: AbilityKind,
    /// ApiType-Name, z. B. `DealDamage` (siehe `ApiType.java`).
    pub api: String,
    pub params: ParamMap,
}

impl ScriptAbility {
    pub fn parse(params: ParamMap) -> Result<ScriptAbility, AbilityError> {
        for (k, v) in params.iter() {
            if let Some(kind) = AbilityKind::from_key(k) {
                return Ok(ScriptAbility {
                    kind,
                    api: v.to_string(),
                    params: params.clone(),
                });
            }
        }
        Err(AbilityError::NoApi(params.to_string()))
    }

    /// Versucht einen SVar-Wert als Fähigkeit zu lesen (`DB$ …`, `AB$ …`).
    /// Gibt `None` für Ausdrücke wie `Count$…` zurück.
    pub fn from_svar(value: &str) -> Option<Result<ScriptAbility, AbilityError>> {
        let head = value.split('|').next()?.split('$').next()?.trim();
        AbilityKind::from_key(head)?;
        Some(ScriptAbility::parse(ParamMap::parse(value)))
    }
}

macro_rules! moded_line {
    ($name:ident, $key:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            /// Wert des Schlüssels
            #[doc = $key]
            pub mode: String,
            pub params: ParamMap,
        }

        impl $name {
            pub fn parse(params: ParamMap) -> Result<$name, AbilityError> {
                let mode = params
                    .get($key)
                    .ok_or_else(|| AbilityError::MissingKey {
                        key: $key,
                        line: params.to_string(),
                    })?
                    .to_string();
                Ok($name { mode, params })
            }
        }
    };
}

moded_line!(
    ScriptTrigger,
    "Mode",
    "Ausgelöste Fähigkeit (`T:`), Modus = TriggerType."
);
moded_line!(
    ScriptStatic,
    "Mode",
    "Statische Fähigkeit (`S:`), Modus = StaticAbilityMode."
);
moded_line!(
    ScriptReplacement,
    "Event",
    "Ersatzeffekt (`R:`), Event = ReplacementType."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let a = ScriptAbility::parse(ParamMap::parse("AB$ Draw | Cost$ T | NumCards$ 1")).unwrap();
        assert_eq!(a.kind, AbilityKind::Activated);
        assert_eq!(a.api, "Draw");
        assert!(ScriptAbility::parse(ParamMap::parse("Cost$ T")).is_err());
        assert!(ScriptAbility::from_svar("Count$CardsInYourHand").is_none());
        assert!(
            ScriptAbility::from_svar("DB$ GainLife | LifeAmount$ 2")
                .unwrap()
                .is_ok()
        );
    }

    #[test]
    fn moded() {
        let t = ScriptTrigger::parse(ParamMap::parse(
            "Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield",
        ))
        .unwrap();
        assert_eq!(t.mode, "ChangesZone");
        assert!(ScriptReplacement::parse(ParamMap::parse("Mode$ X")).is_err());
    }
}
