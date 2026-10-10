//! Original art built entirely from AIGE primitives and its built-in bitmap font.
use crate::combat::move_for;
use crate::model::*;
use aige::World;
use aige_render::{DebugDraw, glyph_bitmap};

type Color = [f32; 4];
const WHITE: Color = [0.9, 0.95, 1.0, 1.0];
const MUTED: Color = [0.42, 0.52, 0.64, 1.0];
const CYAN: Color = [0.2, 0.9, 0.96, 1.0];
const GOLD: Color = [1.0, 0.76, 0.32, 1.0];
const INK: Color = [0.025, 0.035, 0.065, 1.0];

fn shade(mut c: Color, factor: f32) -> Color {
    for v in &mut c[..3] {
        *v *= factor;
    }
    c
}
fn alpha(mut c: Color, a: f32) -> Color {
    c[3] = a;
    c
}
fn text(d: &mut DebugDraw, x: f32, y: f32, s: &str, scale: f32, c: Color) {
    for (i, ch) in s.chars().enumerate() {
        for (row, bits) in glyph_bitmap(ch as u32).iter().enumerate() {
            let mut col = 0;
            while col < 8 {
                if bits >> col & 1 == 0 {
                    col += 1;
                    continue;
                }
                let begin = col;
                while col < 8 && bits >> col & 1 == 1 {
                    col += 1;
                }
                d.rect(
                    x + (i as f32 * 8.0 + begin as f32) * scale,
                    y + row as f32 * scale,
                    (col - begin) as f32 * scale,
                    scale,
                    c,
                );
            }
        }
    }
}
fn center(d: &mut DebugDraw, y: f32, s: &str, scale: f32, c: Color) {
    text(
        d,
        (960.0 - s.len() as f32 * 8.0 * scale) / 2.0,
        y,
        s,
        scale,
        c,
    );
}
fn panel(d: &mut DebugDraw, x: f32, y: f32, w: f32, h: f32, c: Color) {
    d.rect(x, y, w, h, alpha(INK, 0.94));
    d.rect(x, y, 3.0, h, c);
    d.rect(x + 3.0, y, w - 3.0, 1.0, shade(c, 0.3));
    d.rect(x + 3.0, y + h - 1.0, w - 3.0, 1.0, shade(c, 0.3));
}
fn ellipse(d: &mut DebugDraw, x: f32, y: f32, rx: f32, ry: f32, c: Color) {
    let mut row = -ry;
    while row < ry {
        let t = row / ry;
        let extent = rx * (1.0 - t * t).max(0.0).sqrt();
        d.rect(x - extent, y + row, extent * 2.0, 2.0, c);
        row += 2.0;
    }
}
fn polygon(d: &mut DebugDraw, pts: &[(f32, f32)], c: Color) {
    let min = pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max = pts.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    let mut y = min;
    while y < max {
        let sample = y + 0.5;
        let mut edges = Vec::new();
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            if (a.1 <= sample && b.1 > sample) || (b.1 <= sample && a.1 > sample) {
                edges.push(a.0 + (sample - a.1) * (b.0 - a.0) / (b.1 - a.1));
            }
        }
        edges.sort_by(f32::total_cmp);
        for pair in edges.chunks_exact(2) {
            d.rect(pair[0], y, pair[1] - pair[0], 2.0, c);
        }
        y += 2.0;
    }
}
fn limb(d: &mut DebugDraw, a: (f32, f32), b: (f32, f32), width: f32, c: Color) {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let nx = -dy / len * width * 0.5;
    let ny = dx / len * width * 0.5;
    polygon(
        d,
        &[
            (a.0 + nx, a.1 + ny),
            (b.0 + nx, b.1 + ny),
            (b.0 - nx, b.1 - ny),
            (a.0 - nx, a.1 - ny),
        ],
        c,
    );
}

pub fn draw(w: &mut World) {
    let g = w.resource::<Game>().expect("game").clone();
    let d = w.resource_mut::<DebugDraw>().expect("AIGE renderer");
    // The engine clears this at the start of each tick; startup also gets a complete frame.
    match g.phase {
        Phase::Title => title(d, &g),
        Phase::Select => select(d, &g),
        _ => battle(d, &g),
    }
    if g.paused {
        d.rect(0.0, 0.0, 960.0, 540.0, [0.01, 0.02, 0.04, 0.8]);
        panel(d, 284.0, 168.0, 392.0, 200.0, CYAN);
        center(d, 196.0, "PAUSED", 4.0, WHITE);
        center(d, 258.0, "ENTER / ESC   RESUME", 1.5, CYAN);
        center(d, 294.0, "R   CHARACTER SELECT", 1.5, MUTED);
        center(d, 328.0, "F   MOVE LIST", 1.0, MUTED);
    }
    if g.help {
        move_list(d, &g);
    }
}

fn backdrop(d: &mut DebugDraw, arena: usize, frame: u64) {
    let base = match arena {
        1 => [0.09, 0.025, 0.045, 1.0],
        2 => [0.015, 0.065, 0.085, 1.0],
        _ => [0.03, 0.035, 0.09, 1.0],
    };
    d.rect(0.0, 0.0, 960.0, 540.0, base);
    let accent = match arena {
        1 => [1.0, 0.3, 0.28, 1.0],
        2 => [0.3, 1.0, 0.68, 1.0],
        _ => CYAN,
    };
    if arena == 0 {
        ellipse(d, 757.0, 148.0, 58.0, 58.0, [0.16, 0.23, 0.34, 1.0]);
        ellipse(d, 742.0, 138.0, 53.0, 53.0, base);
        for layer in 0..2 {
            for i in 0..17 {
                let x = (i * 63 - 8 + layer * 19) as f32;
                let h = (75 + (i * 41 + layer * 97) % 140) as f32;
                let y = 356.0 - h + layer as f32 * 32.0;
                let c = if layer == 0 {
                    [0.055, 0.065, 0.125, 1.0]
                } else {
                    [0.07, 0.09, 0.16, 1.0]
                };
                d.rect(x, y, 54.0, h, c);
                d.rect(x + 5.0, y - 4.0, 40.0, 4.0, shade(accent, 0.22));
                for row in 0..8 {
                    for col in 0..3 {
                        if (row * 7 + col + i) % 3 != 0 && y + 13.0 + ((row * 17) as f32) < 357.0 {
                            d.rect(
                                x + 8.0 + (col * 14) as f32,
                                y + 13.0 + (row * 17) as f32,
                                5.0,
                                7.0,
                                alpha(accent, 0.15 + layer as f32 * 0.1),
                            );
                        }
                    }
                }
            }
        }
        d.rect(33.0, 200.0, 15.0, 154.0, [0.2, 0.075, 0.19, 1.0]);
        text(d, 36.0, 210.0, "N", 1.0, [1.0, 0.25, 0.65, 1.0]);
        for i in 0..8 {
            d.rect(
                38.0,
                242.0 + i as f32 * 13.0,
                6.0,
                8.0,
                [1.0, 0.25, 0.65, 0.6],
            );
        }
        for i in 0..45 {
            let x = ((i * 79 + 17) % 960) as f32;
            let y = ((i * 43 + frame as i32 * 7) % 370) as f32;
            limb(
                d,
                (x, y),
                (x - 5.0, y + 15.0),
                1.0,
                [0.25, 0.55, 0.75, 0.25],
            );
        }
    } else if arena == 1 {
        ellipse(d, 480.0, 203.0, 103.0, 103.0, [0.22, 0.065, 0.085, 1.0]);
        ellipse(d, 480.0, 203.0, 91.0, 91.0, base);
        ellipse(d, 480.0, 203.0, 73.0, 73.0, [0.29, 0.085, 0.09, 1.0]);
        polygon(
            d,
            &[
                (470.0, 133.0),
                (514.0, 188.0),
                (491.0, 242.0),
                (450.0, 188.0),
            ],
            GOLD,
        );
        for x in [100.0, 252.0, 672.0, 824.0] {
            d.rect(x, 145.0, 35.0, 236.0, [0.12, 0.06, 0.08, 1.0]);
            d.rect(x - 8.0, 134.0, 51.0, 14.0, [0.24, 0.1, 0.1, 1.0]);
            d.rect(x + 5.0, 150.0, 3.0, 230.0, [0.4, 0.12, 0.11, 1.0]);
        }
        for x in [164.0, 764.0] {
            d.rect(x, 296.0, 30.0, 81.0, [0.16, 0.08, 0.08, 1.0]);
            ellipse(
                d,
                x + 15.0,
                284.0,
                13.0,
                22.0 + (frame % 7) as f32,
                [1.0, 0.27, 0.08, 0.65],
            );
            ellipse(d, x + 15.0, 290.0, 7.0, 17.0, GOLD);
        }
        polygon(
            d,
            &[
                (50.0, 110.0),
                (480.0, 70.0),
                (910.0, 110.0),
                (910.0, 124.0),
                (480.0, 94.0),
                (50.0, 124.0),
            ],
            [0.18, 0.055, 0.07, 1.0],
        );
    } else {
        for x in [42.0, 160.0, 756.0, 874.0] {
            d.rect(x, 123.0, 43.0, 251.0, [0.055, 0.13, 0.145, 1.0]);
            d.rect(x + 13.0, 137.0, 15.0, 210.0, [0.17, 0.9, 0.57, 0.3]);
            d.rect(x + 18.0, 137.0, 4.0, 210.0, accent);
            for y in [154.0, 224.0, 294.0] {
                d.rect(x - 4.0, y, 51.0, 12.0, [0.12, 0.21, 0.23, 1.0]);
            }
        }
        ellipse(d, 480.0, 230.0, 113.0, 113.0, [0.055, 0.16, 0.18, 1.0]);
        ellipse(d, 480.0, 230.0, 97.0, 97.0, [0.13, 0.47, 0.38, 1.0]);
        ellipse(d, 480.0, 230.0, 84.0, 84.0, base);
        ellipse(d, 480.0, 230.0, 32.0, 32.0, alpha(accent, 0.8));
        for i in 0..6 {
            let y = 156.0 + i as f32 * 27.0;
            d.rect(285.0, y, 70.0, 3.0, shade(accent, 0.22));
            d.rect(605.0, y, 70.0, 3.0, shade(accent, 0.22));
        }
    }
    d.rect(0.0, 356.0, 960.0, 70.0, [0.04, 0.065, 0.095, 1.0]);
    d.rect(0.0, 363.0, 960.0, 2.0, alpha(accent, 0.6));
    for i in 0..24 {
        d.rect(i as f32 * 43.0, 381.0, 32.0, 7.0, [0.08, 0.14, 0.19, 1.0]);
        d.rect(i as f32 * 43.0 + 12.0, 405.0, 18.0, 5.0, shade(accent, 0.2));
    }
    d.rect(0.0, 426.0, 960.0, 114.0, [0.025, 0.04, 0.07, 1.0]);
    d.rect(0.0, 426.0, 960.0, 3.0, accent);
    for i in -3..15 {
        limb(
            d,
            (480.0 + i as f32 * 35.0, 430.0),
            (480.0 + i as f32 * 180.0, 540.0),
            1.0,
            [0.08, 0.19, 0.25, 1.0],
        );
    }
    for y in [442.0, 464.0, 498.0, 536.0] {
        d.rect(0.0, y, 960.0, 1.0, [0.08, 0.18, 0.23, 1.0]);
    }
    d.rect(0.0, 0.0, 960.0, 3.0, accent);
}

fn title(d: &mut DebugDraw, g: &Game) {
    backdrop(d, 0, g.frame);
    d.rect(0.0, 0.0, 960.0, 540.0, [0.02, 0.025, 0.07, 0.48]);
    text(d, 61.0, 36.0, "AIGE ORIGINAL / VOL. 01", 1.0, CYAN);
    let mut f = Fighter::new(2, 1);
    f.x = 735 * FP;
    fighter(d, &f, g.frame, 1.85, 810.0, 486.0);
    let mut f = Fighter::new(0, 0);
    f.x = 655 * FP;
    fighter(d, &f, g.frame + 10, 1.58, 661.0, 492.0);
    text(d, 58.0, 86.0, "NEON", 8.0, [0.1, 0.39, 0.5, 1.0]);
    text(d, 54.0, 82.0, "NEON", 8.0, WHITE);
    text(d, 58.0, 157.0, "CLASH", 7.0, [0.38, 0.08, 0.22, 1.0]);
    text(d, 54.0, 153.0, "CLASH", 7.0, [1.0, 0.32, 0.58, 1.0]);
    text(d, 61.0, 228.0, "SIX FIGHTERS. ONE SIGNAL.", 1.5, MUTED);
    let options = [
        ("ARCADE", "FIVE RIVALS / ONE CHAMPION"),
        ("LOCAL VERSUS", "TWO PLAYERS / ONE KEYBOARD"),
        ("TRAINING", "LAB COMBOS / FREE METER"),
        ("WATCH THE FIGHT", "CPU VS CPU / LIVE DEMO"),
    ];
    for (i, (label, desc)) in options.iter().enumerate() {
        let y = 282.0 + i as f32 * 45.0;
        if i == g.menu {
            d.rect(54.0, y - 8.0, 440.0, 40.0, [0.09, 0.22, 0.27, 0.9]);
            d.rect(54.0, y - 8.0, 4.0, 40.0, CYAN);
            text(d, 70.0, y, ">", 2.0, CYAN);
        }
        text(
            d,
            101.0,
            y,
            label,
            2.0,
            if i == g.menu { WHITE } else { MUTED },
        );
        text(
            d,
            101.0,
            y + 20.0,
            desc,
            1.0,
            if i == g.menu {
                CYAN
            } else {
                shade(MUTED, 0.75)
            },
        );
    }
    d.rect(0.0, 511.0, 960.0, 29.0, INK);
    text(
        d,
        60.0,
        521.0,
        "W/S OR ARROWS  SELECT     ENTER  CONFIRM     F  HOW TO PLAY",
        1.0,
        MUTED,
    );
    text(d, 768.0, 521.0, "60 HZ / AIGE", 1.0, CYAN);
}

fn select(d: &mut DebugDraw, g: &Game) {
    backdrop(d, g.arena, g.frame);
    d.rect(0.0, 0.0, 960.0, 540.0, [0.015, 0.025, 0.055, 0.88]);
    text(d, 40.0, 27.0, "NEON CLASH", 2.0, CYAN);
    text(d, 681.0, 32.0, mode_label(g.mode), 1.5, MUTED);
    center(
        d,
        75.0,
        if g.select_slot == 0 {
            "CHOOSE YOUR FIGHTER"
        } else {
            "CHOOSE YOUR RIVAL"
        },
        3.0,
        WHITE,
    );
    let pick = g.selected[g.select_slot];
    let c = &ROSTER[pick];
    panel(d, 38.0, 123.0, 309.0, 349.0, c.color);
    text(
        d,
        62.0,
        143.0,
        if g.select_slot == 0 {
            "PLAYER 01"
        } else {
            "PLAYER 02 / RIVAL"
        },
        1.0,
        c.color,
    );
    let f = Fighter::new(pick, 0);
    ellipse(d, 192.0, 384.0, 100.0, 19.0, alpha(c.color, 0.08));
    fighter(d, &f, g.frame, 1.4, 186.0, 382.0);
    text(d, 62.0, 397.0, c.name, 3.0, WHITE);
    text(d, 62.0, 430.0, c.title, 1.5, c.color);
    for (i, character) in ROSTER.iter().enumerate() {
        let x = 376.0 + (i % 3) as f32 * 180.0;
        let y = 129.0 + (i / 3) as f32 * 133.0;
        let active = i == pick;
        d.rect(
            x,
            y,
            166.0,
            119.0,
            if active {
                shade(character.color, 0.2)
            } else {
                [0.055, 0.07, 0.11, 1.0]
            },
        );
        d.rect(
            x,
            y,
            166.0,
            if active { 3.0 } else { 1.0 },
            if active {
                character.color
            } else {
                shade(MUTED, 0.3)
            },
        );
        let f = Fighter::new(i, 0);
        fighter(d, &f, g.frame + i as u64 * 8, 0.46, x + 83.0, y + 90.0);
        d.rect(x, y + 94.0, 166.0, 25.0, alpha(INK, 0.95));
        text(
            d,
            x + 11.0,
            y + 100.0,
            character.name,
            1.5,
            if active { WHITE } else { MUTED },
        );
        if g.select_slot == 1 && i == g.selected[0] {
            text(d, x + 132.0, y + 9.0, "P1", 1.0, CYAN);
        }
        if active {
            text(
                d,
                x + 134.0,
                y + 9.0,
                if g.select_slot == 0 { "01" } else { "02" },
                1.0,
                character.color,
            );
        }
    }
    text(d, 380.0, 407.0, c.style, 1.5, c.color);
    text(
        d,
        380.0,
        435.0,
        &format!("U  {}    S+U  LAUNCHER", c.special),
        1.0,
        MUTED,
    );
    text(
        d,
        380.0,
        456.0,
        &format!("SPEED {}   POWER {}", c.speed / 10, c.power),
        1.0,
        MUTED,
    );
    d.rect(0.0, 488.0, 960.0, 52.0, INK);
    text(
        d,
        40.0,
        499.0,
        &format!("Q/E  ARENA: {}", ARENAS[g.arena]),
        1.0,
        CYAN,
    );
    text(
        d,
        449.0,
        499.0,
        &format!("Z/X  CPU: {}", ["EASY", "NORMAL", "HARD"][g.difficulty]),
        1.0,
        GOLD,
    );
    text(
        d,
        40.0,
        522.0,
        "ARROWS / WASD  SELECT     ENTER  LOCK IN     ESC  BACK",
        1.0,
        MUTED,
    );
    text(d, 798.0, 520.0, "F  MOVES", 1.0, WHITE);
}

fn mode_label(mode: Mode) -> &'static str {
    match mode {
        Mode::Arcade => "ARCADE",
        Mode::Versus => "LOCAL VERSUS",
        Mode::Training => "TRAINING",
        Mode::Demo => "CPU EXHIBITION",
    }
}

fn battle(d: &mut DebugDraw, g: &Game) {
    backdrop(d, g.arena, g.frame);
    let shake = if g.shake > 0 {
        ((g.frame % 3) as i32 - 1) as f32 * (g.shake.min(8) as f32 / 2.0)
    } else {
        0.0
    };
    for f in &g.fighters {
        let x = f.x as f32 / FP as f32;
        ellipse(
            d,
            x,
            FLOOR as f32 + 4.0,
            38.0 - f.y as f32 / FP as f32 / 12.0,
            7.0,
            [0.0, 0.0, 0.0, 0.5],
        );
        fighter(
            d,
            f,
            g.frame,
            1.0,
            x + shake,
            FLOOR as f32 - f.y as f32 / FP as f32,
        );
    }
    for p in &g.projectiles {
        let c = ROSTER[g.fighters[p.owner].character].color;
        let x = p.x as f32 / FP as f32;
        let y = FLOOR as f32 - p.y as f32 / FP as f32;
        ellipse(d, x, y, 24.0, 16.0, alpha(c, 0.13));
        polygon(
            d,
            &[
                (x - p.vx.signum() as f32 * 42.0, y),
                (x, y - 9.0),
                (x + p.vx.signum() as f32 * 13.0, y),
                (x, y + 9.0),
            ],
            c,
        );
        ellipse(d, x, y, 9.0, 7.0, WHITE);
    }
    for e in &g.effects {
        let x = e.x as f32 / FP as f32;
        let y = FLOOR as f32 - e.y as f32 / FP as f32;
        let c = if e.kind == 1 { CYAN } else { GOLD };
        let extent = (24 - e.life).max(1) as f32 * if e.kind == 2 { 3.5 } else { 2.0 };
        for (dx, dy) in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (0.7, 0.7),
            (-0.7, -0.7),
            (-0.7, 0.7),
            (0.7, -0.7),
        ] {
            limb(
                d,
                (x + dx * extent * 0.5, y + dy * extent * 0.5),
                (x + dx * extent, y + dy * extent),
                if e.kind == 2 { 5.0 } else { 3.0 },
                c,
            );
        }
        if e.life > 9 {
            ellipse(d, x, y, 9.0, 9.0, WHITE);
        }
    }
    hud(d, g);
    if g.phase == Phase::Intro {
        d.rect(0.0, 192.0, 960.0, 98.0, [0.015, 0.025, 0.045, 0.8]);
        if g.phase_timer > 32 {
            center(d, 211.0, &format!("ROUND {}", g.round), 5.0, WHITE);
        } else {
            center(d, 211.0, "FIGHT", 6.0, GOLD);
        }
        center(d, 276.0, "ENTER / SPACE  SKIP", 1.0, MUTED);
    }
    if g.phase == Phase::RoundOver {
        let label = if g.remaining <= 0 {
            "TIME UP"
        } else if g.winner == -1 {
            "DOUBLE K.O."
        } else {
            "K.O."
        };
        center(d, 176.0, label, 7.0, GOLD);
        center(
            d,
            247.0,
            &if g.winner < 0 {
                "DRAW ROUND".to_owned()
            } else {
                format!(
                    "{} WINS",
                    ROSTER[g.fighters[g.winner as usize].character].name
                )
            },
            3.0,
            WHITE,
        );
    }
    if g.phase == Phase::MatchOver {
        d.rect(0.0, 0.0, 960.0, 540.0, [0.01, 0.025, 0.06, 0.77]);
        panel(d, 195.0, 138.0, 570.0, 252.0, GOLD);
        let champion = g.mode == Mode::Arcade && g.wins[0] >= 2 && g.arcade_stage == 4;
        center(
            d,
            159.0,
            if champion {
                "SIGNAL CHAMPION"
            } else {
                "MATCH COMPLETE"
            },
            3.0,
            GOLD,
        );
        let winner = if g.wins[0] >= 2 { 0 } else { 1 };
        center(
            d,
            208.0,
            &format!("{} WINS", ROSTER[g.fighters[winner].character].name),
            4.0,
            WHITE,
        );
        center(
            d,
            264.0,
            &format!("{} - {}", g.wins[0], g.wins[1]),
            3.0,
            CYAN,
        );
        let next = if g.mode == Mode::Arcade && winner == 0 && !champion {
            "ENTER  NEXT RIVAL"
        } else {
            "ENTER  REMATCH"
        };
        center(d, 324.0, next, 1.5, WHITE);
        center(
            d,
            356.0,
            "R  CHARACTER SELECT    ESC  MAIN MENU",
            1.0,
            MUTED,
        );
    }
}

fn hud(d: &mut DebugDraw, g: &Game) {
    d.rect(0.0, 0.0, 960.0, 108.0, [0.015, 0.025, 0.05, 0.94]);
    text(d, 36.0, 13.0, mode_label(g.mode), 1.0, CYAN);
    text(d, 685.0, 13.0, ARENAS[g.arena], 1.0, MUTED);
    for side in 0..2 {
        let f = &g.fighters[side];
        let c = ROSTER[f.character].color;
        let x = if side == 0 { 36.0 } else { 553.0 };
        text(d, x, 31.0, ROSTER[f.character].name, 2.0, WHITE);
        let hp_width = 371.0 * f.health as f32 / MAX_HEALTH as f32;
        d.rect(x - 2.0, 55.0, 375.0, 22.0, [0.17, 0.22, 0.28, 1.0]);
        d.rect(x, 57.0, 371.0, 18.0, [0.26, 0.09, 0.15, 1.0]);
        d.rect(
            if side == 0 { x } else { x + 371.0 - hp_width },
            57.0,
            hp_width,
            18.0,
            if f.health < 250 {
                [1.0, 0.22, 0.3, 1.0]
            } else {
                c
            },
        );
        d.rect(
            if side == 0 { x } else { x + 371.0 - hp_width },
            57.0,
            hp_width,
            3.0,
            alpha(WHITE, 0.35),
        );
        text(
            d,
            x + 140.0,
            63.0,
            &format!("{} / 1000", f.health),
            1.0,
            INK,
        );
        d.rect(x, 82.0, 120.0, 4.0, [0.1, 0.14, 0.19, 1.0]);
        d.rect(x, 82.0, f.guard as f32 * 1.2, 4.0, shade(CYAN, 0.55));
        for win in 0..2 {
            ellipse(
                d,
                x + 344.0 + win as f32 * 18.0,
                39.0,
                5.0,
                5.0,
                if g.wins[side] as usize > win {
                    GOLD
                } else {
                    [0.22, 0.26, 0.32, 1.0]
                },
            );
        }
        let meter_x = if side == 0 { 36.0 } else { 684.0 };
        d.rect(meter_x, 475.0, 240.0, 12.0, [0.07, 0.12, 0.18, 1.0]);
        d.rect(
            meter_x,
            475.0,
            f.meter as f32 * 2.4,
            12.0,
            if f.meter == 100 { GOLD } else { c },
        );
        for tick in 1..4 {
            d.rect(meter_x + tick as f32 * 60.0, 475.0, 2.0, 12.0, INK);
        }
        text(
            d,
            meter_x,
            456.0,
            if f.meter >= 100 {
                "NEON BREAKER READY"
            } else {
                "SUPER METER"
            },
            1.0,
            if f.meter >= 100 { GOLD } else { MUTED },
        );
        if f.combo_hits >= 2 && f.combo_timer > 0 {
            let cx = if side == 0 { 39.0 } else { 746.0 };
            text(d, cx, 135.0, &format!("{} HITS", f.combo_hits), 2.5, c);
            text(
                d,
                cx,
                167.0,
                &format!("{} DAMAGE", f.combo_damage),
                1.0,
                WHITE,
            );
        }
    }
    panel(d, 436.0, 26.0, 88.0, 68.0, GOLD);
    let timer = if g.mode == Mode::Training {
        "--".to_owned()
    } else {
        format!("{:02}", (g.remaining.max(0) + 59) / 60)
    };
    text(
        d,
        450.0,
        37.0,
        &timer,
        4.0,
        if g.remaining < 600 {
            [1.0, 0.3, 0.3, 1.0]
        } else {
            WHITE
        },
    );
    text(d, 449.0, 77.0, &format!("RND {}", g.round), 1.0, GOLD);
    if g.notice_timer > 0 {
        center(d, 117.0, &g.notification, 1.5, GOLD);
    }
    if g.mode == Mode::Training {
        panel(d, 343.0, 450.0, 274.0, 40.0, CYAN);
        text(
            d,
            353.0,
            460.0,
            &format!("DUMMY: {}", ["IDLE", "GUARD", "CPU"][g.dummy]),
            1.0,
            CYAN,
        );
        text(d, 353.0, 477.0, "B CHANGE / T RESET", 1.0, MUTED);
    } else if g.mode == Mode::Arcade {
        text(
            d,
            387.0,
            471.0,
            &format!("RIVAL {} / 5", g.arcade_stage + 1),
            1.5,
            MUTED,
        );
    }
    d.rect(0.0, 506.0, 960.0, 34.0, INK);
    text(d, 36.0, 518.0, "P1  WASD  J K L U I O", 1.0, CYAN);
    text(d, 352.0, 518.0, "F  MOVES / ESC  PAUSE", 1.0, MUTED);
    text(d, 652.0, 518.0, "P2  ARROWS  1 2 3 4 5 6", 1.0, CYAN);
}

/// Segmented character rigs. A pose is a pure projection of action frames.
fn fighter(d: &mut DebugDraw, f: &Fighter, frame: u64, scale: f32, x: f32, ground: f32) {
    let character = &ROSTER[f.character];
    let color = if f.flash > 0 && f.flash % 3 != 0 {
        WHITE
    } else {
        character.color
    };
    let dir = f.facing as f32;
    let broad = if f.character == 3 { 1.25 } else { 1.0 };
    let breath = ((frame / 5 % 8) as i32 - 4).abs() as f32 * 0.5;
    let crouch = f.crouch || f.action == Action::Sweep;
    let drop = if crouch { 43.0 } else { 0.0 };
    let lean = if f.stun > 0 && !f.blocking {
        -10.0
    } else {
        0.0
    };
    let at = |px: f32, py: f32| (x + px * dir * scale, ground + py * scale);
    let hip = (lean, -62.0 + drop);
    let shoulder = (lean + 2.0, -118.0 + drop + breath);
    let mut front_knee = (20.0, -35.0 + drop * 0.35);
    let mut front_foot = (31.0, -5.0);
    let mut back_knee = (-21.0, -32.0 + drop * 0.35);
    let mut back_foot = (-35.0, -5.0);
    let mut front_elbow = (31.0, -91.0 + drop);
    let mut front_hand = (41.0, -119.0 + drop + breath);
    let mut back_elbow = (-20.0, -92.0 + drop);
    let mut back_hand = (7.0, -114.0 + drop);
    if f.moving && f.grounded() {
        let stride = ((f.walk_tick % 24) - 12).abs() as f32 - 6.0;
        front_knee.0 += stride;
        front_foot.0 += stride * 2.0;
        back_knee.0 -= stride;
        back_foot.0 -= stride * 2.0;
        front_foot.1 -= stride.max(0.0);
        back_foot.1 -= (-stride).max(0.0);
    }
    let m = move_for(f.character, f.action);
    let active_pose = f.action != Action::Idle
        && f.action_tick >= m.startup.saturating_sub(3)
        && f.action_tick < m.startup + m.active + 7;
    if f.y > 0 {
        front_knee = (26.0, -55.0);
        front_foot = (16.0, -27.0);
        back_knee = (-25.0, -43.0);
        back_foot = (-39.0, -16.0);
    }
    if f.blocking {
        front_elbow = (29.0, -101.0 + drop);
        front_hand = (17.0, -137.0 + drop);
        back_elbow = (20.0, -97.0 + drop);
        back_hand = (29.0, -127.0 + drop);
    }
    if active_pose {
        match f.action {
            Action::Light | Action::Chain => {
                front_elbow = (45.0, -111.0);
                front_hand = (81.0, -110.0);
            }
            Action::Heavy | Action::Air => {
                front_knee = (54.0, -79.0);
                front_foot = (118.0, -99.0);
                front_elbow = (9.0, -94.0);
            }
            Action::Sweep => {
                front_knee = (62.0, -19.0);
                front_foot = (119.0, -13.0);
            }
            Action::Special => {
                front_elbow = (44.0, -82.0);
                front_hand = (78.0, -82.0);
                back_hand = (40.0, -82.0);
            }
            Action::Uppercut => {
                front_elbow = (29.0, -139.0);
                front_hand = (36.0, -183.0);
            }
            Action::Throw => {
                front_elbow = (35.0, -94.0);
                front_hand = (59.0, -109.0);
                back_hand = (59.0, -91.0);
            }
            Action::Super => {
                front_elbow = (54.0, -107.0);
                front_hand = (129.0, -105.0);
                back_hand = (70.0, -82.0);
            }
            _ => {}
        }
    }
    if f.knockdown > 0 && f.y == 0 {
        let a = at(-55.0, -15.0);
        let b = at(33.0, -15.0);
        limb(d, a, b, 23.0 * scale, shade(color, 0.5));
        ellipse(
            d,
            b.0 + dir * 15.0 * scale,
            b.1 - 2.0,
            13.0 * scale,
            12.0 * scale,
            color,
        );
        return;
    }
    let skin: Color = match f.character {
        3 => [0.42, 0.47, 0.52, 1.0],
        5 => [0.7, 0.76, 0.83, 1.0],
        _ => [0.69, 0.48, 0.39, 1.0],
    };
    let armor = [0.1, 0.12, 0.17, 1.0];
    let suit = shade(color, 0.24);
    if matches!(f.character, 0 | 1 | 4) {
        let sway = ((frame / 8 % 8) as f32 - 4.0) * 2.0;
        polygon(
            d,
            &[
                at(-12.0, -130.0 + drop),
                at(-40.0 - sway, -48.0 + drop),
                at(-9.0, -73.0 + drop),
            ],
            shade(color, 0.55),
        );
    }
    // Far limbs, then torso, then near limbs preserve a readable silhouette.
    limb(
        d,
        at(hip.0 - 10.0, hip.1),
        at(back_knee.0, back_knee.1),
        20.0 * broad * scale,
        shade(suit, 0.8),
    );
    limb(
        d,
        at(back_knee.0, back_knee.1),
        at(back_foot.0, back_foot.1),
        16.0 * broad * scale,
        armor,
    );
    limb(
        d,
        at(back_foot.0 - 7.0, back_foot.1),
        at(back_foot.0 + 13.0, back_foot.1),
        10.0 * scale,
        shade(color, 0.7),
    );
    limb(
        d,
        at(shoulder.0 - 10.0, shoulder.1),
        at(back_elbow.0, back_elbow.1),
        17.0 * scale,
        shade(skin, 0.65),
    );
    limb(
        d,
        at(back_elbow.0, back_elbow.1),
        at(back_hand.0, back_hand.1),
        15.0 * scale,
        shade(color, 0.5),
    );
    polygon(
        d,
        &[
            at(shoulder.0 - 24.0 * broad, shoulder.1 - 6.0),
            at(shoulder.0 + 23.0 * broad, shoulder.1 - 6.0),
            at(hip.0 + 17.0 * broad, hip.1),
            at(hip.0 - 18.0 * broad, hip.1),
        ],
        suit,
    );
    polygon(
        d,
        &[
            at(shoulder.0 - 20.0 * broad, shoulder.1 + 2.0),
            at(shoulder.0 + 20.0 * broad, shoulder.1 + 2.0),
            at(hip.0 + 12.0, hip.1 - 24.0),
            at(hip.0 - 13.0, hip.1 - 24.0),
        ],
        shade(color, 0.68),
    );
    limb(
        d,
        at(shoulder.0 - 16.0, shoulder.1 + 3.0),
        at(hip.0 + 10.0, hip.1 - 23.0),
        5.0 * scale,
        armor,
    );
    match f.character {
        0 => {
            polygon(
                d,
                &[
                    at(-16.0, hip.1 + 1.0),
                    at(-28.0, hip.1 + 39.0),
                    at(-6.0, hip.1 + 23.0),
                ],
                shade(color, 0.65),
            );
        }
        1 => {
            polygon(
                d,
                &[
                    at(-19.0, shoulder.1 - 9.0),
                    at(0.0, shoulder.1 + 13.0),
                    at(21.0, shoulder.1 - 9.0),
                ],
                armor,
            );
            let point = at(3.0, shoulder.1 + 20.0);
            ellipse(d, point.0, point.1, 4.0 * scale, 7.0 * scale, color);
        }
        2 => {
            polygon(
                d,
                &[
                    at(7.0, shoulder.1 + 7.0),
                    at(-5.0, shoulder.1 + 28.0),
                    at(5.0, shoulder.1 + 28.0),
                    at(-2.0, hip.1 - 12.0),
                    at(18.0, shoulder.1 + 23.0),
                    at(8.0, shoulder.1 + 23.0),
                ],
                WHITE,
            );
        }
        3 => {
            for row in 0..3 {
                limb(
                    d,
                    at(-19.0, shoulder.1 + 9.0 + row as f32 * 13.0),
                    at(21.0, shoulder.1 + 9.0 + row as f32 * 13.0),
                    9.0 * scale,
                    shade(color, 0.45),
                );
            }
        }
        4 => {
            for offset in [-19.0, 19.0] {
                polygon(
                    d,
                    &[
                        at(offset - 8.0, shoulder.1),
                        at(offset, shoulder.1 - 21.0),
                        at(offset + 11.0, shoulder.1 + 3.0),
                    ],
                    shade(color, 0.8),
                );
            }
        }
        5 => {
            limb(
                d,
                at(6.0, shoulder.1 + 5.0),
                at(6.0, hip.1 - 14.0),
                5.0 * scale,
                armor,
            );
            for row in 0..3 {
                let point = at(6.0, shoulder.1 + 12.0 + row as f32 * 12.0);
                ellipse(d, point.0, point.1, 3.0 * scale, 3.0 * scale, WHITE);
            }
        }
        _ => {}
    }
    limb(
        d,
        at(hip.0 - 18.0 * broad, hip.1 - 8.0),
        at(hip.0 + 18.0 * broad, hip.1 - 8.0),
        9.0 * scale,
        armor,
    );
    let belt = at(hip.0 + 6.0, hip.1 - 8.0);
    ellipse(d, belt.0, belt.1, 4.0 * scale, 4.0 * scale, color);
    limb(
        d,
        at(hip.0 + 10.0, hip.1),
        at(front_knee.0, front_knee.1),
        23.0 * broad * scale,
        suit,
    );
    limb(
        d,
        at(front_knee.0, front_knee.1),
        at(front_foot.0, front_foot.1),
        17.0 * broad * scale,
        armor,
    );
    let knee = at(front_knee.0, front_knee.1);
    ellipse(
        d,
        knee.0,
        knee.1,
        10.0 * scale,
        8.0 * scale,
        shade(color, 0.7),
    );
    limb(
        d,
        at(front_foot.0 - 8.0, front_foot.1),
        at(front_foot.0 + 16.0, front_foot.1),
        11.0 * scale,
        shade(color, 0.8),
    );
    limb(
        d,
        at(shoulder.0 + 13.0, shoulder.1),
        at(front_elbow.0, front_elbow.1),
        19.0 * broad * scale,
        skin,
    );
    limb(
        d,
        at(front_elbow.0, front_elbow.1),
        at(front_hand.0, front_hand.1),
        16.0 * broad * scale,
        shade(color, 0.8),
    );
    let hand = at(front_hand.0, front_hand.1);
    ellipse(d, hand.0, hand.1, 10.0 * scale, 9.0 * scale, color);
    let shoulder_point = at(shoulder.0 + 13.0, shoulder.1);
    ellipse(
        d,
        shoulder_point.0,
        shoulder_point.1,
        13.0 * broad * scale,
        10.0 * scale,
        armor,
    );
    limb(
        d,
        at(shoulder.0 + 5.0, shoulder.1 - 2.0),
        at(shoulder.0 + 24.0, shoulder.1 - 2.0),
        3.0 * scale,
        color,
    );
    let head = at(lean + 4.0, -143.0 + drop + breath);
    ellipse(d, head.0, head.1, 15.0 * broad * scale, 19.0 * scale, armor);
    ellipse(
        d,
        head.0 + dir * 3.0 * scale,
        head.1 - 1.0 * scale,
        11.0 * scale,
        14.0 * scale,
        skin,
    );
    if f.character == 3 {
        polygon(
            d,
            &[
                at(lean - 12.0, -164.0 + drop),
                at(lean + 18.0, -164.0 + drop),
                at(lean + 20.0, -145.0 + drop),
                at(lean - 13.0, -144.0 + drop),
            ],
            shade(color, 0.65),
        );
    } else if f.character == 5 {
        polygon(
            d,
            &[
                at(lean - 13.0, -155.0 + drop),
                at(lean - 10.0, -174.0 + drop),
                at(lean + 8.0, -183.0 + drop),
                at(lean + 14.0, -158.0 + drop),
            ],
            color,
        );
    } else {
        limb(
            d,
            at(lean - 10.0, -157.0 + drop + breath),
            at(lean + 16.0, -157.0 + drop + breath),
            9.0 * scale,
            shade(color, 0.6),
        );
    }
    limb(
        d,
        at(lean + 2.0, -145.0 + drop + breath),
        at(lean + 17.0, -145.0 + drop + breath),
        3.0 * scale,
        WHITE,
    );
    limb(
        d,
        at(lean - 6.0, -135.0 + drop + breath),
        at(lean + 17.0, -135.0 + drop + breath),
        10.0 * scale,
        shade(color, 0.85),
    );
    if f.blocking {
        let a = at(57.0, -135.0 + drop);
        let b = at(67.0, -100.0 + drop);
        let c = at(57.0, -65.0 + drop);
        limb(d, a, b, 3.0 * scale, alpha(CYAN, 0.7));
        limb(d, b, c, 3.0 * scale, alpha(CYAN, 0.7));
    }
    if f.action == Action::Super && f.action_tick < 25 {
        ellipse(
            d,
            hand.0,
            hand.1,
            19.0 * scale,
            19.0 * scale,
            alpha(color, 0.28),
        );
        ellipse(d, hand.0, hand.1, 8.0 * scale, 8.0 * scale, WHITE);
    }
}

fn move_list(d: &mut DebugDraw, g: &Game) {
    d.rect(0.0, 0.0, 960.0, 540.0, [0.01, 0.02, 0.04, 0.94]);
    panel(d, 52.0, 36.0, 856.0, 469.0, CYAN);
    center(d, 55.0, "FIGHT MANUAL", 3.0, WHITE);
    text(d, 84.0, 108.0, "PLAYER 01", 2.0, CYAN);
    text(d, 527.0, 108.0, "PLAYER 02", 2.0, GOLD);
    let left = [
        "A / D   MOVE",
        "W       JUMP",
        "S       CROUCH / LOW GUARD",
        "J / K   PUNCH / OVERHEAD KICK",
        "L       HOLD GUARD",
        "U       CHARACTER SPECIAL",
        "S + U   RISING LAUNCHER",
        "S + K   LOW SWEEP",
        "I       THROW / THROW ESCAPE",
        "O       SUPER (100 METER)",
    ];
    let right = [
        "LEFT / RIGHT   MOVE",
        "UP             JUMP",
        "DOWN           CROUCH",
        "1 / 2          PUNCH / KICK",
        "3              HOLD GUARD",
        "4              SPECIAL",
        "DOWN + 4       LAUNCHER",
        "DOWN + 2       SWEEP",
        "5              THROW / TECH",
        "6              SUPER",
    ];
    for i in 0..left.len() {
        text(d, 84.0, 146.0 + i as f32 * 21.0, left[i], 1.0, WHITE);
        text(d, 527.0, 146.0 + i as f32 * 21.0, right[i], 1.0, MUTED);
    }
    d.rect(84.0, 360.0, 790.0, 1.0, shade(CYAN, 0.35));
    text(
        d,
        84.0,
        376.0,
        "COMBO  J > J > K > U : PRESS NEXT ATTACK WHEN THE HIT CONNECTS",
        1.0,
        GOLD,
    );
    text(
        d,
        84.0,
        398.0,
        "MOTION  DOWN, FORWARD, J = SPECIAL / DOUBLE FORWARD = DASH",
        1.0,
        WHITE,
    );
    text(
        d,
        84.0,
        420.0,
        "BLOCK LOW WITH S+L; STAND TO BLOCK KICKS. THROWS BEAT GUARD.",
        1.0,
        MUTED,
    );
    text(
        d,
        84.0,
        442.0,
        "TRAINING  B DUMMY / T RESET     FIRST TO TWO ROUNDS WINS",
        1.0,
        MUTED,
    );
    text(
        d,
        84.0,
        478.0,
        &format!(
            "{}: U = {}",
            ROSTER[g.selected[0]].name, ROSTER[g.selected[0]].special
        ),
        1.0,
        CYAN,
    );
    text(d, 716.0, 478.0, "F  CLOSE", 1.0, WHITE);
}
