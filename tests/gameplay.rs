use aige::Key;
use aige_test::Harness;
use neon_clash::{build_game, combat, model::*};

fn tap(h: &mut Harness, key: Key) {
    h.press(key).step(1).release(key).step(1);
}
fn state(h: &Harness) -> &Game {
    h.resource::<Game>()
}
fn game_mut(h: &mut Harness) -> &mut Game {
    h.world_mut().resource_mut::<Game>().unwrap()
}
fn fight(a: usize, b: usize, distance: i32) -> Harness {
    let mut h = Harness::new(build_game);
    let g = game_mut(&mut h);
    g.selected = [a, b];
    g.mode = Mode::Versus;
    g.start_match();
    g.phase = Phase::Fight;
    g.fighters[0].x = 400 * FP;
    g.fighters[1].x = (400 + distance) * FP;
    h
}
fn attack(h: &mut Harness, key: Key, ticks: u64) {
    tap(h, key);
    h.step(ticks);
}

#[test]
fn menu_selection_and_arena_work_through_real_inputs() {
    let mut h = Harness::new(build_game);
    assert_eq!(state(&h).phase, Phase::Title);
    tap(&mut h, Key::Down);
    tap(&mut h, Key::Enter);
    assert_eq!(state(&h).mode, Mode::Versus);
    for _ in 0..5 {
        tap(&mut h, Key::Right);
    }
    tap(&mut h, Key::E);
    tap(&mut h, Key::X);
    tap(&mut h, Key::Enter);
    assert_eq!(state(&h).select_slot, 1);
    tap(&mut h, Key::Down);
    tap(&mut h, Key::Enter);
    assert_eq!(state(&h).selected, [5, 4]);
    assert_eq!(state(&h).arena, 1);
    assert_eq!(state(&h).difficulty, 2);
    assert_eq!(state(&h).phase, Phase::Intro);
    tap(&mut h, Key::Enter);
    assert_eq!(state(&h).phase, Phase::Fight);
    assert_eq!(state(&h).fighters[0].character, 5);
}

#[test]
fn startup_range_and_one_hit_per_move() {
    for distance in [70, 190] {
        let mut h = fight(0, 1, distance);
        h.press(Key::J).step(5);
        assert_eq!(
            state(&h).fighters[1].health,
            MAX_HEALTH,
            "startup is not active"
        );
        h.step(1);
        let health = state(&h).fighters[1].health;
        assert_eq!(health, if distance == 70 { 945 } else { MAX_HEALTH });
        h.step(50);
        assert_eq!(
            state(&h).fighters[1].health,
            health,
            "held key and active frames cannot repeat damage"
        );
    }
}

#[test]
fn standing_guard_crouching_guard_and_overheads() {
    for (low_guard, low_attack, blocked) in [
        (false, false, true),
        (true, true, true),
        (false, true, false),
        (true, false, false),
    ] {
        let mut h = fight(0, 2, 70);
        h.press(Key::Num3);
        if low_guard {
            h.press(Key::Down);
        }
        if low_attack {
            h.press(Key::S);
        }
        attack(&mut h, Key::K, 20);
        let g = state(&h);
        assert_eq!(
            g.fighters[1].health == MAX_HEALTH,
            blocked,
            "low_guard={low_guard}, low_attack={low_attack}"
        );
        if blocked {
            assert!(g.fighters[1].guard < 100);
        }
    }
}

#[test]
fn guard_persists_during_blockstun() {
    let mut h = fight(0, 1, 65);
    h.press(Key::Num3);
    attack(&mut h, Key::J, 5);
    assert!(state(&h).fighters[1].blockstun);
    attack(&mut h, Key::J, 12);
    assert_eq!(state(&h).fighters[1].health, MAX_HEALTH);
    assert!(state(&h).fighters[1].guard <= 70);
}

#[test]
fn guard_break_is_finite_and_punishable() {
    let mut h = fight(0, 1, 70);
    game_mut(&mut h).fighters[1].guard = 10;
    h.press(Key::Num3);
    attack(&mut h, Key::J, 6);
    let f = &state(&h).fighters[1];
    assert!(!f.blockstun);
    assert!(f.stun >= 60);
    assert_eq!(state(&h).notification, "GUARD BREAK");
}

#[test]
fn throw_beats_guard_and_simultaneous_throws_are_teched() {
    let mut h = fight(3, 1, 55);
    h.press(Key::Num3);
    attack(&mut h, Key::I, 10);
    assert!(state(&h).fighters[1].health < MAX_HEALTH);
    let mut h = fight(3, 1, 55);
    h.press(Key::I).press(Key::Num5).step(10);
    assert_eq!(state(&h).fighters[0].health, MAX_HEALTH);
    assert_eq!(state(&h).fighters[1].health, MAX_HEALTH);
    assert_eq!(state(&h).notification, "THROW ESCAPE");
}

#[test]
fn simultaneous_attacks_trade() {
    let mut h = fight(0, 0, 70);
    h.press(Key::J).press(Key::Num1).step(6);
    assert!(state(&h).fighters.iter().all(|f| f.health < MAX_HEALTH));
}

#[test]
fn launcher_air_physics_and_landing() {
    let mut h = fight(0, 1, 70);
    h.press(Key::S);
    attack(&mut h, Key::U, 16);
    assert!(state(&h).fighters[1].y > 0);
    h.step(100);
    assert_eq!(state(&h).fighters[1].y, 0);
    assert_eq!(state(&h).fighters[1].vy, 0);
}

#[test]
fn jump_evades_low_sweep() {
    let mut h = fight(0, 1, 80);
    h.press(Key::Up).step(5).release(Key::Up);
    h.press(Key::S);
    attack(&mut h, Key::K, 20);
    assert_eq!(state(&h).fighters[1].health, MAX_HEALTH);
}

#[test]
fn special_projectiles_for_every_zoner_and_crouch_evasion() {
    for character in [0, 2, 4] {
        for crouch in [false, true] {
            let mut h = fight(character, 3, 300);
            if crouch {
                h.press(Key::Down);
            }
            attack(&mut h, Key::U, 60);
            assert_eq!(state(&h).fighters[1].health < MAX_HEALTH, !crouch);
            let health = state(&h).fighters[1].health;
            h.step(120);
            assert_eq!(state(&h).fighters[1].health, health);
            assert!(state(&h).projectiles.is_empty());
        }
    }
}

#[test]
fn rush_specials_close_distance_and_damage() {
    for character in [1, 3, 5] {
        let mut h = fight(character, 0, 180);
        attack(&mut h, Key::U, 35);
        assert!(state(&h).fighters[0].x > 400 * FP);
        assert!(
            state(&h).fighters[1].health < MAX_HEALTH,
            "character {character}"
        );
    }
}

#[test]
fn super_requires_and_consumes_meter() {
    let mut h = fight(0, 1, 100);
    attack(&mut h, Key::O, 25);
    assert_eq!(state(&h).fighters[1].health, MAX_HEALTH);
    game_mut(&mut h).fighters[0].meter = 100;
    attack(&mut h, Key::O, 30);
    assert!(state(&h).fighters[1].health <= 730);
    assert_eq!(state(&h).fighters[0].meter, 0);
}

#[test]
fn four_hit_cancel_combo_and_scaling() {
    let mut h = fight(0, 1, 55);
    attack(&mut h, Key::J, 4);
    assert_eq!(state(&h).fighters[0].combo_hits, 1);
    attack(&mut h, Key::J, 11);
    assert_eq!(state(&h).fighters[0].combo_hits, 2);
    attack(&mut h, Key::K, 18);
    assert_eq!(state(&h).fighters[0].combo_hits, 3);
    attack(&mut h, Key::U, 25);
    let g = state(&h);
    assert_eq!(g.fighters[0].combo_hits, 4);
    assert!(g.fighters[0].combo_damage < 55 + 65 + 110 + 130);
    assert!(g.fighters[0].combo_damage > 250);
}

#[test]
fn motion_special_and_double_tap_dash_work_on_both_sides() {
    for side in 0..2 {
        let mut h = fight(0, 2, 250);
        let (down, forward, punch) = if side == 0 {
            (Key::S, Key::D, Key::J)
        } else {
            (Key::Down, Key::Left, Key::Num1)
        };
        tap(&mut h, down);
        h.press(forward).press(punch).step(1);
        assert_eq!(state(&h).fighters[side].action, Action::Special);
        let mut h = fight(0, 2, 250);
        tap(&mut h, forward);
        h.press(forward).step(1);
        assert_eq!(state(&h).fighters[side].action, Action::Dash);
    }
}

#[test]
fn arena_bounds_and_pushboxes_do_not_overlap() {
    let mut h = fight(0, 3, 70);
    h.press(Key::D).press(Key::Left).step(200);
    let g = state(&h);
    assert!((g.fighters[0].x - g.fighters[1].x).abs() >= 50 * FP);
    h.release(Key::D)
        .release(Key::Left)
        .press(Key::A)
        .press(Key::Right)
        .step(300);
    assert!(
        state(&h)
            .fighters
            .iter()
            .all(|f| f.x >= 45 * FP && f.x <= 915 * FP)
    );
}

#[test]
fn pause_and_move_list_freeze_combat() {
    let mut h = fight(0, 1, 70);
    tap(&mut h, Key::Escape);
    let clock = state(&h).remaining;
    h.press(Key::J).step(30);
    assert_eq!(state(&h).remaining, clock);
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::F);
    let clock = state(&h).remaining;
    h.step(30);
    assert_eq!(state(&h).remaining, clock);
    tap(&mut h, Key::F);
    h.step(2);
    assert!(state(&h).remaining < clock);
}

#[test]
fn knockout_best_of_three_and_draw_timeout() {
    let mut h = fight(0, 1, 70);
    for round in 0..2 {
        let g = game_mut(&mut h);
        g.phase = Phase::Fight;
        g.fighters[1].health = 1;
        g.fighters[0].x = 400 * FP;
        g.fighters[1].x = 470 * FP;
        attack(&mut h, Key::J, 6);
        assert_eq!(state(&h).phase, Phase::RoundOver);
        assert_eq!(state(&h).wins[0], round + 1);
        tap(&mut h, Key::Enter);
    }
    assert_eq!(state(&h).phase, Phase::MatchOver);
    assert_eq!(h.probe("match_finished"), true);
    let mut h = fight(0, 1, 70);
    game_mut(&mut h).remaining = 1;
    h.step(1);
    assert_eq!(state(&h).winner, -1);
    assert_eq!(state(&h).wins, [0, 0]);
}

#[test]
fn training_reset_dummy_and_no_round_ending() {
    let mut h = fight(0, 1, 70);
    game_mut(&mut h).mode = Mode::Training;
    tap(&mut h, Key::B);
    assert_eq!(state(&h).dummy, 1);
    tap(&mut h, Key::T);
    assert_eq!(state(&h).fighters[0].meter, 100);
    assert_eq!(state(&h).fighters[1].health, MAX_HEALTH);
    let g = game_mut(&mut h);
    g.fighters[1].health = 0;
    g.remaining = 0;
    h.step(1);
    assert_eq!(state(&h).phase, Phase::Fight);
    assert_eq!(state(&h).fighters[1].health, 1);
}

#[test]
fn arcade_visits_each_other_character_once() {
    for player in 0..6 {
        let mut g = Game {
            selected: [player, 0],
            ..Game::default()
        };
        let mut opponents = Vec::new();
        for stage in 0..5 {
            g.arcade_stage = stage;
            g.start_match();
            opponents.push(g.selected[1]);
        }
        opponents.sort_unstable();
        opponents.dedup();
        assert_eq!(opponents.len(), 5);
        assert!(!opponents.contains(&player));
    }
}

#[test]
fn armor_absorbs_strikes_but_throws_interrupt_it() {
    let mut h = fight(0, 3, 70);
    h.press(Key::J).press(Key::Num4).step(6);
    assert_eq!(state(&h).fighters[1].action, Action::Special);
    assert_eq!(state(&h).fighters[1].stun, 0);
    assert!(state(&h).fighters[1].health > 945);
    assert!(state(&h).fighters[1].health < MAX_HEALTH);
    let mut h = fight(0, 3, 55);
    h.press(Key::I).press(Key::Num4).step(8);
    assert_eq!(state(&h).fighters[1].action, Action::Idle);
    assert!(state(&h).fighters[1].stun > 0);
    assert!(state(&h).fighters[1].health <= 860);
}

#[test]
fn character_specials_have_distinct_frames_and_frost_stun() {
    let bolt = combat::move_for(2, Action::Special);
    let frost = combat::move_for(4, Action::Special);
    assert!(bolt.startup < frost.startup);
    assert!(frost.stun > bolt.stun);
    assert_eq!(
        combat::move_for(5, Action::Special).height,
        Height::Overhead
    );
    let mut h = fight(4, 1, 80);
    attack(&mut h, Key::U, 17);
    assert_eq!(state(&h).notification, "FROZEN");
    assert!(state(&h).fighters[1].stun >= 45);
}

#[test]
fn connected_projectile_allows_super_cancel() {
    let mut h = fight(0, 1, 80);
    game_mut(&mut h).fighters[0].meter = 100;
    attack(&mut h, Key::U, 11);
    assert!(state(&h).fighters[0].connected);
    attack(&mut h, Key::O, 6);
    assert_eq!(state(&h).fighters[0].action, Action::Super);
}

#[test]
fn move_list_can_be_read_while_paused() {
    let mut h = fight(0, 1, 70);
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::F);
    assert!(state(&h).paused && state(&h).help);
    let clock = state(&h).remaining;
    h.step(10);
    assert_eq!(state(&h).remaining, clock);
    tap(&mut h, Key::Escape);
    assert!(!state(&h).help && state(&h).paused);
}

#[test]
fn cpu_completes_bouts_across_roster_difficulties_and_seeds() {
    for character in 0..6 {
        for difficulty in 0..3 {
            let mut g = Game {
                mode: Mode::Demo,
                selected: [character, (character + 3) % 6],
                difficulty,
                ..Game::default()
            };
            g.rng ^= (character * 17 + difficulty) as u64;
            g.start_match();
            g.phase = Phase::Fight;
            let mut hits = 0;
            for _ in 0..4000 {
                combat::tick(&mut g, &aige::Input::default());
                hits += i32::from(g.hitstop > 0);
                if g.phase == Phase::RoundOver {
                    break;
                }
            }
            assert_eq!(
                g.phase,
                Phase::RoundOver,
                "character {character}, difficulty {difficulty}"
            );
            assert!(hits > 0, "CPU must actually engage");
            assert!(
                g.fighters
                    .iter()
                    .all(|f| (0..=100).contains(&f.meter) && (0..=MAX_HEALTH).contains(&f.health))
            );
        }
    }
}

#[test]
fn snapshot_restore_is_exact_and_replaying_inputs_is_deterministic() {
    let mut h = Harness::new(build_game).seed(42);
    tap(&mut h, Key::Enter);
    tap(&mut h, Key::Enter);
    tap(&mut h, Key::Enter);
    h.press(Key::D).step(36).release(Key::D);
    attack(&mut h, Key::J, 16);
    attack(&mut h, Key::U, 30);
    h.assert_deterministic(180);
    let snapshot = h.snapshot();
    let hash = h.world().hash().unwrap();
    h.step(100).restore(&snapshot);
    assert_eq!(h.world().hash().unwrap(), hash);
}

#[test]
fn gpu_renders_every_screen_and_arena_without_mutating_world() {
    if !aige_render::gpu_available() {
        eprintln!("SKIP: no GPU adapter");
        return;
    }
    eprintln!(
        "GPU: {}",
        aige_render::gpu_adapter_info().unwrap_or_default()
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(root.join("artifacts")).unwrap();
    let mut h = Harness::new(build_game).with_renderer(root.join("assets"));
    for (name, phase, arena) in [
        ("title", Phase::Title, 0),
        ("select", Phase::Select, 0),
        ("rain-district", Phase::Fight, 0),
        ("red-sanctum", Phase::Fight, 1),
        ("reactor-zero", Phase::Fight, 2),
        ("result", Phase::MatchOver, 2),
    ] {
        let g = game_mut(&mut h);
        g.phase = phase;
        g.mode = Mode::Versus;
        g.arena = arena;
        g.wins = [2, 0];
        g.fighters[0].x = 360 * FP;
        g.fighters[1].x = 600 * FP;
        h.step(1);
        let hash = h.world().hash().unwrap();
        let png = h.screenshot(1920, 1080).unwrap();
        assert_eq!(h.world().hash().unwrap(), hash);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 1920);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 1080);
        assert!(png.len() > 8000);
        std::fs::write(root.join(format!("artifacts/{name}.png")), png).unwrap();
    }
}
