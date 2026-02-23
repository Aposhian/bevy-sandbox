mod common;

use bevy_sandbox::health::{DamageKind, DamageKindMask, Health};
use common::TestApp;

#[test]
fn zero_health_entity_gets_despawn_event() {
    let mut app = TestApp::new();
    app.start_game_no_map();

    // Spawn an entity with zero health
    let entity = app
        .app
        .world_mut()
        .spawn(Health {
            max: 5,
            current: 0,
            vulnerable_to: DamageKind::Projectile.mask(),
        })
        .id();

    // The health_despawner system should fire a DespawnEvent
    app.tick();

    // After the DespawnEvent is processed (in Last schedule), the entity should be gone
    app.tick();

    assert!(
        app.app.world().get_entity(entity).is_err(),
        "entity with 0 health should be despawned"
    );
}

#[test]
fn damage_kind_mask_filtering() {
    let mask = DamageKind::Projectile.mask();
    assert!(mask.contains(DamageKind::Projectile));
    assert!(!mask.contains(DamageKind::Impact));

    let combined = DamageKind::Projectile.mask() | DamageKind::Impact.mask();
    assert!(combined.contains(DamageKind::Projectile));
    assert!(combined.contains(DamageKind::Impact));

    assert!(!DamageKindMask::NONE.contains(DamageKind::Projectile));
    assert!(!DamageKindMask::NONE.contains(DamageKind::Impact));
}
