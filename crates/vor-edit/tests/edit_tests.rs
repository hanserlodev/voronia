use vor_core::entities::burg::Burg;
use vor_core::entities::province::Province;
use vor_core::entities::state::{GovernmentForm, State};
use vor_core::pack::Pack;
use vor_core::world::World;

#[test]
fn rename_state_found_by_id() {
    let mut world = World::default();
    // The loader assigns real ids skipping 0; simulate the same
    world.states.push(State {
        id: 1,
        name: "Old".into(),
        ..State::placeholder()
    });
    world.states.push(State {
        id: 2,
        name: "Other".into(),
        ..State::placeholder()
    });

    vor_edit::rename_state(&mut world, 1, "NewName").unwrap();
    assert_eq!(world.states[0].name, "NewName");
    assert_eq!(world.states[1].name, "Other");
}

#[test]
fn rename_state_empty_name_fails() {
    let mut world = World::default();
    world.states.push(State {
        id: 1,
        name: "X".into(),
        ..State::placeholder()
    });
    let err = vor_edit::rename_state(&mut world, 1, "  ").unwrap_err();
    assert!(
        matches!(err, vor_edit::EditError::EmptyName { .. }),
        "empty name should return EmptyName, got {err:?}"
    );
}

#[test]
fn rename_state_id_zero_not_found() {
    let mut world = World::default();
    let err = vor_edit::rename_state(&mut world, 0, "Whatever").unwrap_err();
    assert!(
        matches!(err, vor_edit::EditError::EntityNotFound { what, .. } if what == "state"),
        "id=0 should give EntityNotFound, got {err:?}"
    );
}

#[test]
fn rename_state_nonexistent_id_fails() {
    let mut world = World::default();
    world.states.push(State {
        id: 1,
        name: "X".into(),
        ..State::placeholder()
    });
    let err = vor_edit::rename_state(&mut world, 99, "X").unwrap_err();
    assert!(
        matches!(err, vor_edit::EditError::EntityNotFound { .. }),
        "{err:?}"
    );
}

#[test]
fn set_state_color_normalizes() {
    let mut world = World::default();
    world.states.push(State {
        id: 1,
        name: "X".into(),
        color: String::new(),
        ..State::placeholder()
    });

    vor_edit::set_state_color(&mut world, 1, "#AABBCC").unwrap();
    assert_eq!(world.states[0].color, "#aabbcc");

    vor_edit::set_state_color(&mut world, 1, "DDEEFF").unwrap();
    assert_eq!(world.states[0].color, "#ddeeff");
}

#[test]
fn set_state_color_invalid_hex() {
    let mut world = World::default();
    world.states.push(State {
        id: 1,
        name: "X".into(),
        ..State::placeholder()
    });
    let err = vor_edit::set_state_color(&mut world, 1, "#GGGGGG").unwrap_err();
    assert!(matches!(err, vor_edit::EditError::InvalidHexColor(_)));
}

#[test]
fn set_state_form() {
    let mut world = World::default();
    world.states.push(State {
        id: 1,
        name: "X".into(),
        form: GovernmentForm::Monarchy,
        ..State::placeholder()
    });
    vor_edit::set_state_form(&mut world, 1, GovernmentForm::Republic).unwrap();
    assert_eq!(world.states[0].form, GovernmentForm::Republic);
}

#[test]
fn rename_burg() {
    let mut world = World::default();
    world.burgs.push(Burg {
        id: 1,
        name: "Old".into(),
        ..Burg::placeholder()
    });
    vor_edit::rename_burg(&mut world, 1, "New").unwrap();
    assert_eq!(world.burgs[0].name, "New");
}

#[test]
fn set_burg_population_updates_cell() {
    let mut world = World::default();
    world.pack = Pack {
        cells: vor_core::cells::PackCells {
            population: vec![100.0, 200.0],
            ..vor_core::cells::PackCells::default()
        },
        points: vec![[0.0, 0.0], [10.0, 10.0]],
        ..Pack::default()
    };
    world.burgs.push(Burg {
        id: 1,
        name: "Test".into(),
        cell: 1,
        ..Burg::placeholder()
    });
    vor_edit::set_burg_population(&mut world, 1, 500.0).unwrap();
    assert!((world.burgs[0].population - 500.0).abs() < 1e-6);
    assert!((world.pack.cells.population[1] - 500.0).abs() < 1e-6);
}

#[test]
fn toggle_burg_capital_clears_others() {
    let mut world = World::default();
    world.states.push(State {
        id: 10,
        name: "Kingdom".into(),
        ..State::placeholder()
    });
    world.burgs.push(Burg {
        id: 1,
        name: "A".into(),
        state: 10,
        is_capital: false,
        cell: 0,
        ..Burg::placeholder()
    });
    world.burgs.push(Burg {
        id: 2,
        name: "B".into(),
        state: 10,
        is_capital: true,
        cell: 1,
        ..Burg::placeholder()
    });
    // Promote burg 1 to capital — should remove capital status from burg 2
    vor_edit::toggle_burg_capital(&mut world, 1, true).unwrap();
    assert!(world.burgs[0].is_capital);
    assert!(!world.burgs[1].is_capital);
    // state center_cell should follow the capital
    assert_eq!(world.states[0].center_cell, 0);
}

#[test]
fn rename_province() {
    let mut world = World::default();
    world.provinces.push(Province {
        id: 1,
        name: "Old".into(),
        ..Province::default()
    });
    vor_edit::rename_province(&mut world, 1, "New").unwrap();
    assert_eq!(world.provinces[0].name, "New");
}

#[test]
fn set_province_color() {
    let mut world = World::default();
    world.provinces.push(Province {
        id: 1,
        name: "P".into(),
        color: String::new(),
        ..Province::default()
    });
    vor_edit::set_province_color(&mut world, 1, "#112233").unwrap();
    assert_eq!(world.provinces[0].color, "#112233");
}

#[test]
fn normalize_hex_ok() {
    assert_eq!(vor_edit::normalize_hex("#ABC123").unwrap(), "#abc123");
    assert_eq!(vor_edit::normalize_hex("DEF456").unwrap(), "#def456");
}

#[test]
fn normalize_hex_invalid() {
    assert!(vor_edit::normalize_hex("#ZZZZZZ").is_err());
    assert!(vor_edit::normalize_hex("#ABCDE").is_err());
    assert!(vor_edit::normalize_hex("").is_err());
}
