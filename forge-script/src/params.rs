//! Parameterlisten der Form `Key$ Value | Key2$ Value2`.
//!
//! Java-Vorbild: `forge.util.FileSection.parseToMap(line, DOLLAR_SIGN_KV_SEPARATOR)`.
//! Wichtig: Schlüssel sind dort **case-insensitive** (`TreeMap` mit
//! `CASE_INSENSITIVE_ORDER`); Werte und Schlüssel werden getrimmt; ein
//! Schlüssel ohne `$` bekommt den leeren Wert. Bei doppelten Schlüsseln
//! gewinnt der letzte – das bilden wir nach, merken uns aber die
//! Originalreihenfolge (für Diagnose und Round-Trip).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParamMap {
    entries: Vec<(String, String)>,
}

impl ParamMap {
    pub fn parse(line: &str) -> ParamMap {
        let mut map = ParamMap::default();
        if line.trim().is_empty() {
            return map;
        }
        for part in line.split('|') {
            let (k, v) = match part.split_once('$') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => (part.trim(), ""),
            };
            map.insert(k, v);
        }
        map
    }

    /// Setzt einen Wert (ersetzt einen vorhandenen Schlüssel unabhängig von
    /// Groß-/Kleinschreibung an seiner bisherigen Position).
    pub fn insert(&mut self, key: &str, value: &str) {
        if let Some(e) = self
            .entries
            .iter_mut()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
        {
            e.1 = value.to_string();
        } else {
            self.entries.push((key.to_string(), value.to_string()));
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// `True`/`true` → wahr, wie `"True".equalsIgnoreCase(...)` in Java.
    pub fn is_true(&self, key: &str) -> bool {
        self.get(key)
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Erster Eintrag (bei Fähigkeiten `SP$`/`AB$`/`DB$`, bei Triggern meist `Mode$`).
    pub fn first(&self) -> Option<(&str, &str)> {
        self.entries.first().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

impl fmt::Display for ParamMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .entries
            .iter()
            .map(|(k, v)| format!("{k}$ {v}"))
            .collect();
        f.write_str(&parts.join(" | "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic() {
        let p = ParamMap::parse(
            "SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SpellDescription$ CARDNAME deals 3 damage.",
        );
        assert_eq!(p.first(), Some(("SP", "DealDamage")));
        assert_eq!(p.get("numdmg"), Some("3"));
        assert_eq!(p.get("ValidTgts"), Some("Any"));
        assert_eq!(p.len(), 4);
    }

    #[test]
    fn value_with_dollar_and_flags() {
        let p = ParamMap::parse(
            "DB$ Pump | Defined$ Targeted | NumAtt$ +X | Secondary | X$ Count$xPaid",
        );
        assert_eq!(p.get("X"), Some("Count$xPaid"));
        assert_eq!(p.get("Secondary"), Some(""));
    }

    #[test]
    fn duplicate_last_wins() {
        let p = ParamMap::parse("A$ 1 | a$ 2");
        assert_eq!(p.len(), 1);
        assert_eq!(p.get("A"), Some("2"));
    }
}
