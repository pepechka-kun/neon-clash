//! NEON CLASH: an original deterministic, native AIGE fighting game.
pub mod combat;
pub mod model;
mod presentation;

use aige::{App, Input, Key, Rng, World};
use model::*;

/// One definition for the native game, MCP interface and test harness.
pub fn build_game(app: &mut App) {
    aige_render::register(app);
    app.register_resource::<Game>();
    app.insert_resource(Game::default());
    app.add_startup("first_frame", |w: &mut World| {
        let camera = w.spawn();
        w.insert(
            camera,
            aige_render::Camera2D {
                pos: aige_render::Vec2::new(480.0, 270.0),
                zoom: 2.0,
            },
        )?;
        let seed = w.resource_mut::<Rng>()?.next_u32() as u64;
        w.resource_mut::<Game>()?.rng ^= seed;
        presentation::draw(w);
        Ok(())
    });
    app.add_system("gameplay", update);
    app.add_system("presentation", presentation::draw);
    app.add_probe("info", |w: &World| {
        let g = w.resource::<Game>().expect("registered game");
        serde_json::json!({
            "game": "NEON CLASH", "objective": "Win two rounds; defeat the arcade roster",
            "controls": {"p1": "A/D move, W jump, S crouch, J punch, K kick, L guard, U special, I throw, O super", "p2": "Arrows move/jump/crouch; 1 punch, 2 kick, 3 guard, 4 special, 5 throw, 6 super", "menu": "Arrows or WASD; Enter confirm; Escape pause/back; F move list; Q/E arena; Z/X difficulty", "training": "T reset; B dummy behavior"},
            "combos": ["J > J > K > U (cancel on hit)", "S, forward, J: special motion", "S+U: launcher", "S+K: low sweep", "O with 100 meter: super", "I close: throw; I to tech"],
            "state": g
        })
    });
    app.add_probe("match_finished", |w: &World| {
        serde_json::json!(w.resource::<Game>().expect("game").phase == Phase::MatchOver)
    });
    app.add_probe("hit_connected", |w: &World| {
        serde_json::json!(w.resource::<Game>().expect("game").hitstop > 0)
    });
}

fn update(w: &mut World) {
    let input = w.resource::<Input>().expect("engine input").clone();
    let g = w.resource_mut::<Game>().expect("registered game");
    g.frame += 1;
    if input.just_pressed(Key::F) {
        g.help = !g.help;
    }
    if input.just_pressed(Key::Escape) {
        if g.help {
            g.help = false;
            return;
        }
        match g.phase {
            Phase::Fight | Phase::Intro | Phase::RoundOver => g.paused = !g.paused,
            Phase::Select if g.select_slot == 1 => g.select_slot = 0,
            _ => {
                g.phase = Phase::Title;
                g.paused = false;
                g.help = false;
            }
        }
        return;
    }
    if g.help {
        return;
    }
    if g.paused {
        if input.just_pressed(Key::Enter) {
            g.paused = false;
        }
        if input.just_pressed(Key::R) {
            g.phase = Phase::Select;
            g.paused = false;
            g.select_slot = 0;
        }
        return;
    }
    let left = input.just_pressed(Key::Left) || input.just_pressed(Key::A);
    let right = input.just_pressed(Key::Right) || input.just_pressed(Key::D);
    let up = input.just_pressed(Key::Up) || input.just_pressed(Key::W);
    let down = input.just_pressed(Key::Down) || input.just_pressed(Key::S);
    let confirm = input.just_pressed(Key::Enter) || input.just_pressed(Key::Space);
    match g.phase {
        Phase::Title => {
            if up {
                g.menu = (g.menu + 3) % 4;
            }
            if down {
                g.menu = (g.menu + 1) % 4;
            }
            if confirm {
                g.mode = [Mode::Arcade, Mode::Versus, Mode::Training, Mode::Demo][g.menu];
                g.phase = Phase::Select;
                g.select_slot = 0;
                g.arcade_stage = 0;
                if g.mode == Mode::Demo {
                    g.start_match();
                }
            }
        }
        Phase::Select => {
            let slot = g.select_slot;
            let pick = &mut g.selected[slot];
            if left {
                *pick = (*pick + 5) % 6;
            }
            if right {
                *pick = (*pick + 1) % 6;
            }
            if up || down {
                *pick = (*pick + 3) % 6;
            }
            if input.just_pressed(Key::Q) {
                g.arena = (g.arena + 2) % 3;
            }
            if input.just_pressed(Key::E) {
                g.arena = (g.arena + 1) % 3;
            }
            if input.just_pressed(Key::Z) {
                g.difficulty = g.difficulty.saturating_sub(1);
            }
            if input.just_pressed(Key::X) {
                g.difficulty = (g.difficulty + 1).min(2);
            }
            if confirm {
                if slot == 0 && matches!(g.mode, Mode::Versus | Mode::Training) {
                    g.select_slot = 1;
                } else {
                    g.start_match();
                }
            }
        }
        Phase::Intro => {
            g.phase_timer -= 1;
            if g.phase_timer == 0 || confirm {
                g.phase = Phase::Fight;
            }
        }
        Phase::Fight => {
            if g.mode == Mode::Training {
                if input.just_pressed(Key::T) {
                    g.reset_round();
                    g.phase = Phase::Fight;
                }
                if input.just_pressed(Key::B) {
                    g.dummy = (g.dummy + 1) % 3;
                }
            }
            combat::tick(g, &input);
        }
        Phase::RoundOver => {
            g.phase_timer -= 1;
            if g.phase_timer == 0 || confirm {
                if g.wins.iter().any(|&wins| wins >= 2) {
                    g.phase = Phase::MatchOver;
                } else {
                    g.round += 1;
                    g.reset_round();
                }
            }
        }
        Phase::MatchOver => {
            if input.just_pressed(Key::R) {
                g.phase = Phase::Select;
                g.select_slot = 0;
            }
            if confirm {
                if g.mode == Mode::Arcade && g.wins[0] >= 2 && g.arcade_stage < 4 {
                    g.arcade_stage += 1;
                    g.arena = (g.arena + 1) % 3;
                }
                g.start_match();
            }
        }
    }
}
