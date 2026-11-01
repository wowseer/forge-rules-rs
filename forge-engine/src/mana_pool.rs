//! Manapool und Bezahlen von Manakosten (CR 106, CR 601.2g–h).
//!
//! Java-Vorbild: `forge.game.mana.ManaPool`, `ManaCostBeingPaid`.

use forge_types::{Color, ManaCost, ManaShard};

/// Ein Mana im Pool. `color == None` = farbloses Mana.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaUnit {
    pub color: Option<Color>,
    pub snow: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManaPool {
    pub units: Vec<ManaUnit>,
}

/// Wie ein phyrexianisches Symbol bezahlt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhyrexianPay {
    Mana,
    Life,
}

fn unit_pays(shard: &ManaShard, u: &ManaUnit) -> bool {
    if shard.snow {
        return u.snow;
    }
    if shard.colorless && u.color.is_none() {
        return true;
    }
    match u.color {
        Some(c) => shard.colors.contains(c),
        None => false,
    }
}

impl ManaPool {
    pub fn add(&mut self, color: Option<Color>, snow: bool) {
        self.units.push(ManaUnit { color, snow });
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// CR 500.4: Pool leert sich am Ende jedes Schritts und jeder Phase.
    pub fn clear(&mut self) {
        self.units.clear();
    }

    /// Versucht `cost` (mit festgelegtem X und Phyrexian-Entscheidungen)
    /// vollständig aus dem Pool zu bezahlen. Bei Erfolg wird das Mana
    /// entfernt und `true` geliefert; sonst bleibt der Pool unverändert.
    ///
    /// Die Zuordnung ist eine Suche mit Backtracking über die (wenigen)
    /// farbigen Symbole; generische Kosten werden zuletzt mit dem Rest bezahlt,
    /// wobei farbloses Mana bevorzugt wird.
    pub fn try_pay(&mut self, cost: &ManaCost, x: u32, phyrexian: &[PhyrexianPay]) -> bool {
        let mut generic = cost.generic + x * cost.x_count() as u32;
        let mut shards: Vec<ManaShard> = Vec::new();
        let mut phy = phyrexian.iter();
        for s in cost.shards.iter().filter(|s| !s.x) {
            if s.phyrexian {
                match phy.next() {
                    Some(PhyrexianPay::Life) => continue,
                    _ => {
                        shards.push(ManaShard {
                            phyrexian: false,
                            ..*s
                        });
                        continue;
                    }
                }
            }
            shards.push(*s);
        }
        // Restriktivste Symbole zuerst.
        shards.sort_by_key(|s| (s.or_two_generic, s.colors.count() + u32::from(s.colorless)));

        fn search(
            shards: &[ManaShard],
            used: &mut Vec<bool>,
            units: &[ManaUnit],
            generic_extra: &mut Vec<u32>,
        ) -> bool {
            let Some((first, rest)) = shards.split_first() else {
                return true;
            };
            for (i, u) in units.iter().enumerate() {
                if !used[i] && unit_pays(first, u) {
                    used[i] = true;
                    if search(rest, used, units, generic_extra) {
                        return true;
                    }
                    used[i] = false;
                }
            }
            if first.or_two_generic {
                generic_extra.push(2);
                if search(rest, used, units, generic_extra) {
                    return true;
                }
                generic_extra.pop();
            }
            false
        }

        let mut used = vec![false; self.units.len()];
        let mut extra = Vec::new();
        if !search(&shards, &mut used, &self.units, &mut extra) {
            return false;
        }
        generic += extra.iter().sum::<u32>();
        let mut free: Vec<usize> = (0..self.units.len()).filter(|i| !used[*i]).collect();
        if (free.len() as u32) < generic {
            return false;
        }
        // farbloses Mana zuerst für generische Kosten
        free.sort_by_key(|i| self.units[*i].color.is_some());
        for i in free.into_iter().take(generic as usize) {
            used[i] = true;
        }
        let mut idx = 0;
        self.units.retain(|_| {
            let keep = !used[idx];
            idx += 1;
            keep
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool(syms: &[Option<Color>]) -> ManaPool {
        let mut p = ManaPool::default();
        for s in syms {
            p.add(*s, false);
        }
        p
    }

    #[test]
    fn pays_colored_and_generic() {
        let mut p = pool(&[Some(Color::Green), Some(Color::Red), None]);
        assert!(p.try_pay(&ManaCost::parse("1 G").unwrap(), 0, &[]));
        assert_eq!(
            p.units,
            vec![ManaUnit {
                color: Some(Color::Red),
                snow: false
            }]
        );
    }

    #[test]
    fn fails_without_change() {
        let mut p = pool(&[Some(Color::Red), Some(Color::Red)]);
        assert!(!p.try_pay(&ManaCost::parse("G").unwrap(), 0, &[]));
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn hybrid_backtracking() {
        // {R/G}{R}: R muss für {R} bleiben.
        let mut p = pool(&[Some(Color::Red), Some(Color::Green)]);
        assert!(p.try_pay(&ManaCost::parse("RG R").unwrap(), 0, &[]));
        assert!(p.is_empty());
    }

    #[test]
    fn x_phyrexian_twogeneric() {
        let mut p = pool(&[Some(Color::Red), None, None, None]);
        assert!(p.try_pay(&ManaCost::parse("X R").unwrap(), 3, &[]));
        let mut q = pool(&[]);
        assert!(q.try_pay(&ManaCost::parse("BP").unwrap(), 0, &[PhyrexianPay::Life]));
        let mut r = pool(&[None, None]);
        assert!(r.try_pay(&ManaCost::parse("2W").unwrap(), 0, &[]));
    }
}
