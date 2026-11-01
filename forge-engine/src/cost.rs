//! Kosten von Fähigkeiten (`Cost$`), CR 118 / CR 602.
//!
//! Java-Vorbild: `forge.game.cost.Cost` und die `CostPart*`-Klassen.
//! Format: durch Leerzeichen getrennte Teile; Teile mit Parametern stehen in
//! spitzen Klammern und dürfen Leerzeichen enthalten:
//! `1 R T Sac<1/Creature.Other/another creature> PayLife<2>`.

use crate::valid::{ValidError, ValidSpec};
use forge_types::{ManaCost, ManaParseError};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostPart {
    /// `{T}` (CR 107.5)
    Tap,
    /// `{Q}` (CR 107.6)
    Untap,
    /// `Sac<1/CARDNAME>` – sich selbst opfern
    SacrificeSelf,
    /// `Sac<n/Typ>`
    Sacrifice { amount: u32, valid: ValidSpec },
    /// `PayLife<n>`
    PayLife(u32),
    /// `Discard<n/Typ>` (`Hand` = ganze Hand ist nicht unterstützt)
    Discard { amount: u32, valid: ValidSpec },
    /// `Discard<1/CARDNAME>`: diese Karte abwerfen (Cycling u. ä., aus der Hand)
    DiscardSelf,
    /// `ExileFromGrave<1/CARDNAME>`: diese Karte aus dem Friedhof exilieren
    ExileSelfFromGraveyard,
    /// `AddCounter<n/TYP>`: Marken auf die Quelle legen (z. B. Loyalität +n, CR 606.4)
    AddCounter { amount: u32, kind: String },
    /// `SubCounter<n/TYP>`: Marken von der Quelle entfernen (CR 118.3, 606.4)
    RemoveCounter { amount: u32, kind: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cost {
    pub mana: Option<ManaCost>,
    pub parts: Vec<CostPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CostError {
    Mana(ManaParseError),
    Unsupported(String),
}

impl fmt::Display for Cost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = Vec::new();
        if let Some(m) = &self.mana {
            parts.push(m.to_string());
        }
        for p in &self.parts {
            parts.push(match p {
                CostPart::Tap => "{T}".into(),
                CostPart::Untap => "{Q}".into(),
                CostPart::SacrificeSelf => "Sac<1/CARDNAME>".into(),
                CostPart::Sacrifice { amount, valid } => format!("Sac<{amount}/{valid:?}>"),
                CostPart::PayLife(n) => format!("PayLife<{n}>"),
                CostPart::Discard { amount, valid } => format!("Discard<{amount}/{valid:?}>"),
                CostPart::DiscardSelf => "Discard<1/CARDNAME>".into(),
                CostPart::ExileSelfFromGraveyard => "ExileFromGrave<1/CARDNAME>".into(),
                CostPart::AddCounter { amount, kind } => format!("AddCounter<{amount}/{kind}>"),
                CostPart::RemoveCounter { amount, kind } => format!("SubCounter<{amount}/{kind}>"),
            });
        }
        write!(f, "{}", parts.join(" "))
    }
}

impl fmt::Display for CostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CostError::Mana(e) => write!(f, "Kosten: {e}"),
            CostError::Unsupported(s) => write!(f, "Kostenteil nicht unterstützt: {s}"),
        }
    }
}

impl From<ValidError> for CostError {
    fn from(e: ValidError) -> Self {
        CostError::Unsupported(e.to_string())
    }
}

/// Zerlegt an Leerzeichen außerhalb von `<…>`.
fn tokenize(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    for ch in s.chars() {
        match ch {
            '<' => {
                depth += 1;
                cur.push(ch);
            }
            '>' => {
                depth -= 1;
                cur.push(ch);
            }
            ' ' if depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `Name<a/b/c>` → (`Name`, [`a`,`b`,`c`])
fn split_param(tok: &str) -> Option<(&str, Vec<&str>)> {
    let open = tok.find('<')?;
    let inner = tok[open + 1..].strip_suffix('>')?;
    Some((&tok[..open], inner.split('/').collect()))
}

fn amount(s: &str, tok: &str) -> Result<u32, CostError> {
    s.trim()
        .parse()
        .map_err(|_| CostError::Unsupported(tok.to_string()))
}

impl Cost {
    pub fn parse(s: &str) -> Result<Cost, CostError> {
        let mut cost = Cost::default();
        let mut mana_tokens: Vec<String> = Vec::new();
        for tok in tokenize(s.trim()) {
            match tok.as_str() {
                "T" => cost.parts.push(CostPart::Tap),
                "Q" => cost.parts.push(CostPart::Untap),
                _ => {
                    if let Some((name, args)) = split_param(&tok) {
                        let part = match (name, args.as_slice()) {
                            ("Sac", [n, "CARDNAME", ..]) | ("Sac", [n, "Self", ..])
                                if n.trim() == "1" =>
                            {
                                CostPart::SacrificeSelf
                            }
                            ("Sac", [n, v, ..]) => CostPart::Sacrifice {
                                amount: amount(n, &tok)?,
                                valid: ValidSpec::parse(v)?,
                            },
                            ("PayLife", [n]) => CostPart::PayLife(amount(n, &tok)?),
                            ("AddCounter", [n, k]) => CostPart::AddCounter {
                                amount: amount(n, &tok)?,
                                kind: k.trim().to_string(),
                            },
                            ("SubCounter", [n, k]) => CostPart::RemoveCounter {
                                amount: amount(n, &tok)?,
                                kind: k.trim().to_string(),
                            },
                            ("ExileFromGrave", [n, "CARDNAME", ..]) if n.trim() == "1" => {
                                CostPart::ExileSelfFromGraveyard
                            }
                            ("Discard", [n, "CARDNAME", ..]) if n.trim() == "1" => {
                                CostPart::DiscardSelf
                            }
                            ("Discard", [n, v, ..]) if *v != "Hand" && *v != "Random" => {
                                CostPart::Discard {
                                    amount: amount(n, &tok)?,
                                    valid: ValidSpec::parse(v)?,
                                }
                            }
                            _ => return Err(CostError::Unsupported(tok.clone())),
                        };
                        cost.parts.push(part);
                    } else if forge_types::ManaShard::parse(&tok).is_ok()
                        || tok.parse::<u32>().is_ok()
                    {
                        mana_tokens.push(tok);
                    } else {
                        return Err(CostError::Unsupported(tok));
                    }
                }
            }
        }
        if !mana_tokens.is_empty() {
            cost.mana = Some(ManaCost::parse(&mana_tokens.join(" ")).map_err(CostError::Mana)?);
        }
        Ok(cost)
    }

    pub fn has_tap(&self) -> bool {
        self.parts.contains(&CostPart::Tap)
    }

    pub fn has_untap(&self) -> bool {
        self.parts.contains(&CostPart::Untap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_costs() {
        let c = Cost::parse("1 R T").unwrap();
        assert_eq!(c.mana.unwrap().mana_value(), 2);
        assert!(c.parts.contains(&CostPart::Tap));
        let s = Cost::parse("T Sac<1/CARDNAME>").unwrap();
        assert_eq!(s.parts, vec![CostPart::Tap, CostPart::SacrificeSelf]);
        let o = Cost::parse("Sac<1/Creature.Other/another creature> PayLife<2>").unwrap();
        assert_eq!(o.parts.len(), 2);
        assert!(Cost::parse("0").unwrap().mana.unwrap().is_zero());
        let l = Cost::parse("SubCounter<2/LOYALTY>").unwrap();
        assert_eq!(
            l.parts,
            vec![CostPart::RemoveCounter {
                amount: 2,
                kind: "LOYALTY".into()
            }]
        );
        assert!(Cost::parse("SubCounter<X/LOYALTY>").is_err());
    }
}
