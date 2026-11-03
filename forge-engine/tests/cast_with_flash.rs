//! Static `CastWithFlash` (wie mit Flash wirken).

mod common;
use common::*;
use forge_engine::decision::Action;
use forge_engine::{Step, Zone};

fn bears_castable_with_stack(orrery_owner: Option<forge_engine::PlayerId>) -> bool {
    let (db, mut g) = game_at(Step::Main1, 10);
    put_n(&db, &mut g, "Mountain", P1, Zone::Battlefield, 1);
    put_n(&db, &mut g, "Forest", P1, Zone::Battlefield, 2);
    if let Some(o) = orrery_owner {
        put(&db, &mut g, "Orrery Lite", o, Zone::Battlefield);
    }
    let bolt = put(&db, &mut g, "Lightning Bolt", P1, Zone::Hand);
    let bears = put(&db, &mut g, "Grizzly Bears", P1, Zone::Hand);
    let mut d = Driver::new(g);
    d.cast(bolt, &[T2]); // Stapel nicht leer → keine Hexerei-Zeit
    d.g().legal_actions(P1).contains(&Action::Cast(bears))
}

#[test]
fn creature_with_flash_permission() {
    assert!(!bears_castable_with_stack(None));
    assert!(bears_castable_with_stack(Some(P1)));
    assert!(
        !bears_castable_with_stack(Some(P2)),
        "Caster$ You = Controller der Quelle"
    );
}
