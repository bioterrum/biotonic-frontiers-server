use biotonic_server::game::{logic, types::*};
use uuid::Uuid;

#[tokio::test]
async fn spawn_unit_and_attack() {
    let p1 = Uuid::new_v4();
    let _p2 = Uuid::new_v4();

    let light = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Light,
        owner_id: p1,
        hp: 0, // will be initialised by resolve_turn
    };

    let a_p1 = vec![TurnAction::PlayUnit {
        unit: light.clone(),
    }];
    let a_p2 = vec![TurnAction::Pass];

    let mut pool_p1 = ResourcePool {
        energy: 5,
        biomass: 5,
        gene_seeds: 2,
    };
    let mut pool_p2 = pool_p1.clone();
    let mut units_p1 = Vec::<Unit>::new();
    let mut units_p2 = Vec::<Unit>::new();

    let res = logic::resolve_turn(
        a_p1.clone(),
        a_p2.clone(),
        &mut pool_p1,
        &mut pool_p2,
        &mut units_p1,
        &mut units_p2,
    );

    // one unit spawned
    assert_eq!(res.spawned.len(), 1);
    assert_eq!(units_p1.len(), 1);
    // energy cost paid
    assert_eq!(pool_p1.energy, 4);
    // no unit destroyed
    assert!(res.destroyed.is_empty());
}
