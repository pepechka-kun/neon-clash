//! Fixed-point combat. Rendering never feeds back into this simulation.
use crate::model::*;
use aige::{Input, Key};

#[derive(Clone, Copy, Debug)]
pub struct Move {
    pub startup: i32,
    pub active: i32,
    pub recovery: i32,
    pub reach: i32,
    pub damage: i32,
    pub stun: i32,
    pub height: Height,
    pub launch: i32,
    pub knockdown: i32,
}
impl Move {
    pub fn duration(&self) -> i32 {
        self.startup + self.active + self.recovery
    }
    pub fn is_active(&self, tick: i32) -> bool {
        tick >= self.startup && tick < self.startup + self.active
    }
}
pub fn move_data(action: Action) -> Move {
    let (startup, active, recovery, reach, damage, stun, height, launch, knockdown) = match action {
        Action::Light => (5, 3, 10, 88, 55, 23, Height::Mid, 0, 0),
        Action::Chain => (4, 4, 12, 101, 65, 26, Height::Mid, 0, 0),
        Action::Heavy => (11, 5, 19, 131, 110, 32, Height::Overhead, 0, 0),
        Action::Sweep => (10, 5, 23, 132, 85, 18, Height::Low, 0, 40),
        Action::Air => (6, 8, 13, 117, 85, 28, Height::Overhead, 0, 0),
        Action::Special => (12, 8, 21, 110, 130, 30, Height::Mid, 0, 15),
        Action::Uppercut => (8, 6, 25, 100, 120, 34, Height::Mid, 1400, 18),
        Action::Throw => (7, 2, 25, 66, 140, 25, Height::Throw, 500, 30),
        Action::Super => (16, 9, 29, 185, 270, 40, Height::Mid, 900, 45),
        Action::Dash => (0, 0, 12, 0, 0, 0, Height::Mid, 0, 0),
        Action::Idle => (0, 0, 0, 0, 0, 0, Height::Mid, 0, 0),
    };
    Move {
        startup,
        active,
        recovery,
        reach,
        damage,
        stun,
        height,
        launch,
        knockdown,
    }
}

/// Character-specific frame data gives every special its own role.
pub fn move_for(character: usize, action: Action) -> Move {
    let mut m = move_data(action);
    if action == Action::Special {
        match character {
            0 => {
                m.damage = 140;
            }
            1 => {
                m.startup = 7;
                m.active = 7;
                m.recovery = 17;
                m.damage = 100;
            }
            2 => {
                m.startup = 9;
                m.recovery = 19;
                m.damage = 100;
                m.stun = 25;
            }
            3 => {
                m.startup = 16;
                m.active = 10;
                m.recovery = 26;
                m.damage = 180;
                m.knockdown = 35;
            }
            4 => {
                m.startup = 17;
                m.recovery = 26;
                m.damage = 90;
                m.stun = 48;
            }
            5 => {
                m.startup = 11;
                m.recovery = 19;
                m.reach = 145;
                m.damage = 115;
                m.height = Height::Overhead;
            }
            _ => {}
        }
    }
    if action == Action::Throw && character == 3 {
        m.damage = 180;
    }
    m
}

#[derive(Clone, Copy, Default)]
struct Control {
    axis: i32,
    down: bool,
    jump: bool,
    guard: bool,
    attack: Action,
    forward_edge: bool,
}

fn human(input: &Input, side: usize, f: &Fighter, clock: u64) -> Control {
    let keys = if side == 0 {
        [
            Key::A,
            Key::D,
            Key::W,
            Key::S,
            Key::J,
            Key::K,
            Key::L,
            Key::U,
            Key::I,
            Key::O,
        ]
    } else {
        [
            Key::Left,
            Key::Right,
            Key::Up,
            Key::Down,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
        ]
    };
    let axis = i32::from(input.is_pressed(keys[1])) - i32::from(input.is_pressed(keys[0]));
    let down = input.is_pressed(keys[3]);
    let forward = axis == f.facing;
    let attack = if input.just_pressed(keys[9]) {
        Action::Super
    } else if input.just_pressed(keys[8]) {
        Action::Throw
    } else if input.just_pressed(keys[7]) {
        if down {
            Action::Uppercut
        } else {
            Action::Special
        }
    } else if input.just_pressed(keys[5]) {
        if !f.grounded() {
            Action::Air
        } else if down {
            Action::Sweep
        } else {
            Action::Heavy
        }
    } else if input.just_pressed(keys[4]) {
        if !f.grounded() {
            Action::Air
        } else if forward
            && f.down_tick > 0
            && clock.saturating_sub(f.down_tick) <= 24
            && ((f.forward_tick >= f.down_tick && clock.saturating_sub(f.forward_tick) <= 16)
                || !f.was_forward)
        {
            Action::Special
        } else {
            Action::Light
        }
    } else {
        Action::Idle
    };
    Control {
        axis,
        down,
        jump: input.just_pressed(keys[2]),
        guard: input.is_pressed(keys[6]),
        attack,
        forward_edge: forward && !f.was_forward,
    }
}

fn cpu(g: &mut Game, side: usize) -> Control {
    let me = &g.fighters[side];
    let other = &g.fighters[1 - side];
    let distance = (me.x - other.x).abs() / FP;
    let facing = if other.x >= me.x { 1 } else { -1 };
    let danger = other.action != Action::Idle && other.action != Action::Dash && distance < 190;
    let projectile_danger = g
        .projectiles
        .iter()
        .any(|p| p.owner != side && (p.x - me.x).abs() < 170 * FP);
    let low = other.action == Action::Sweep;
    let wait = me.ai_wait;
    let meter = me.meter;
    let can_jump = me.grounded() && me.action == Action::Idle;
    let other_airborne = other.y > 0;
    let random = g.random();
    // Defensive decisions persist over short reaction intervals; no perfect input reading.
    let block = (danger || projectile_danger) && random % 100 < [28, 60, 84][g.difficulty];
    let mut c = Control {
        guard: block,
        down: block && low,
        ..Default::default()
    };
    if distance > 108 && !block {
        c.axis = facing;
    }
    if distance < 65 && random % 100 < 15 {
        c.axis = -facing;
    }
    if wait <= 0 && !block {
        c.attack = if meter >= 100 && distance < 180 {
            Action::Super
        } else if random.is_multiple_of(3)
            && distance
                > if matches!(g.fighters[side].character, 0 | 2 | 4) {
                    150
                } else {
                    110
                }
        {
            Action::Special
        } else if distance < 128 {
            match random % 10 {
                0 if distance < 65 => Action::Throw,
                1 | 2 => Action::Sweep,
                3 | 4 => Action::Heavy,
                5 if other_airborne => Action::Uppercut,
                6 => Action::Special,
                _ => Action::Light,
            }
        } else {
            Action::Idle
        };
        c.jump = can_jump && random.is_multiple_of(29);
        g.fighters[side].ai_wait = [28, 16, 8][g.difficulty] + (random % 9) as i32;
    }
    c
}

/// Advances a single combat tick, including simultaneous hit resolution.
pub fn tick(g: &mut Game, input: &Input) {
    if g.notice_timer > 0 {
        g.notice_timer -= 1;
    }
    if g.shake > 0 {
        g.shake -= 1;
    }
    for e in &mut g.effects {
        e.life -= 1;
    }
    g.effects.retain(|e| e.life > 0);

    // Buffer attacks during hit freeze; clocks and physics stay frozen.
    let mut controls = [Control::default(); 2];
    for (side, control) in controls.iter_mut().enumerate() {
        *control = if (side == 0 && g.mode == Mode::Demo)
            || (side == 1
                && (matches!(g.mode, Mode::Arcade | Mode::Demo)
                    || (g.mode == Mode::Training && g.dummy == 2)))
        {
            if g.hitstop == 0 {
                cpu(g, side)
            } else {
                Control::default()
            }
        } else if side == 1 && g.mode == Mode::Training {
            Control {
                guard: g.dummy == 1,
                down: g.dummy == 1 && g.fighters[0].action == Action::Sweep,
                ..Default::default()
            }
        } else {
            human(input, side, &g.fighters[side], g.fight_tick)
        };
        if control.attack != Action::Idle {
            g.fighters[side].buffered = control.attack;
            g.fighters[side].buffer_timer = 10;
        }
    }
    if g.hitstop > 0 {
        g.hitstop -= 1;
        return;
    }
    g.fight_tick += 1;
    if g.mode != Mode::Training {
        g.remaining -= 1;
    }

    for (side, c) in controls.iter().enumerate() {
        let other_x = g.fighters[1 - side].x;
        let f = &mut g.fighters[side];
        f.flash = (f.flash - 1).max(0);
        f.invulnerable = (f.invulnerable - 1).max(0);
        f.ai_wait = (f.ai_wait - 1).max(0);
        f.stun = (f.stun - 1).max(0);
        if f.stun == 0 {
            f.blockstun = false;
        }
        f.moving = false;
        if f.grounded() {
            f.knockdown = (f.knockdown - 1).max(0);
        }
        if f.combo_timer > 0 {
            f.combo_timer -= 1;
        }
        // A combo ends when the victim recovers, not on an arbitrary input sequence.
        if f.combo_timer == 0 {
            f.combo_hits = 0;
            f.combo_damage = 0;
        }
        if f.action == Action::Idle && f.free() {
            f.facing = if other_x >= f.x { 1 } else { -1 };
        }
        if c.down {
            f.down_tick = g.fight_tick;
        }
        if c.forward_edge {
            f.forward_tick = g.fight_tick;
            if f.last_forward > 0
                && g.fight_tick.saturating_sub(f.last_forward) <= 15
                && f.free()
                && f.action == Action::Idle
                && !c.down
            {
                begin(f, Action::Dash);
            }
            f.last_forward = g.fight_tick;
        }
        f.was_forward = c.axis == f.facing;
        f.crouch = c.down && f.grounded() && f.free() && f.action == Action::Idle;
        f.blocking =
            c.guard && (f.free() || f.blockstun) && f.grounded() && f.action == Action::Idle;
        if f.blocking {
            f.crouch = c.down;
        }
        if !f.blocking && g.fight_tick.is_multiple_of(6) {
            f.guard = (f.guard + 1).min(100);
        }
        if f.action != Action::Idle {
            f.action_tick += 1;
            if f.action_tick >= move_for(f.character, f.action).duration() {
                f.action = Action::Idle;
                f.action_tick = 0;
            }
        }
        let mut wanted = f.buffered;
        if wanted == Action::Light && f.action == Action::Light && f.connected {
            wanted = Action::Chain;
        }
        let cancel = f.connected
            && f.action_tick >= move_for(f.character, f.action).startup
            && can_cancel(f.action, wanted);
        if f.buffer_timer > 0 && f.free() && (f.action == Action::Idle || cancel) {
            if (wanted != Action::Super || f.meter >= 100)
                && (wanted != Action::Throw || f.grounded())
            {
                begin(f, wanted);
            }
            f.buffer_timer = 0;
            f.buffered = Action::Idle;
        } else {
            f.buffer_timer = (f.buffer_timer - 1).max(0);
        }
        if f.free() && f.action == Action::Idle {
            if c.jump && f.grounded() {
                f.vy = 1250;
                f.y = 1;
            }
            if !f.blocking && !f.crouch {
                let speed = ROSTER[f.character].speed;
                f.x += c.axis * if f.grounded() { speed } else { speed * 3 / 4 };
                f.moving = c.axis != 0;
                if f.moving {
                    f.walk_tick += 1;
                }
            }
        }
        if f.action == Action::Dash && f.free() {
            f.x += f.facing * 1100;
        }
        if f.action == Action::Special
            && matches!(f.character, 1 | 3 | 5)
            && (4..move_for(f.character, f.action).startup + 8).contains(&f.action_tick)
            && f.free()
        {
            f.x += f.facing
                * match f.character {
                    1 => 1300,
                    3 => 850,
                    _ => 780,
                };
        }
        if f.y > 0 || f.vy > 0 {
            f.y += f.vy;
            f.vy -= 65;
            if f.y <= 0 {
                f.y = 0;
                f.vy = 0;
            }
        }
        f.x = f.x.clamp(45 * FP, 915 * FP);
    }
    pushboxes(g);
    emit_projectiles(g);

    let mut hits = Vec::new();
    for side in 0..2 {
        let f = &g.fighters[side];
        let target = &g.fighters[1 - side];
        let m = move_for(f.character, f.action);
        let projectile_special = f.action == Action::Special && matches!(f.character, 0 | 2 | 4);
        let delta = (target.x - f.x) * f.facing;
        let vertical = (f.y - target.y).abs();
        let vertical_reach = if f.action == Action::Uppercut {
            170 * FP
        } else {
            90 * FP
        };
        let throwable = m.height != Height::Throw
            || (target.grounded() && target.knockdown == 0 && target.stun == 0);
        if !projectile_special
            && !f.connected
            && m.is_active(f.action_tick)
            && delta >= -15 * FP
            && delta <= m.reach * FP
            && vertical <= vertical_reach
            && target.invulnerable == 0
            && target.knockdown == 0
            && throwable
            && (m.height != Height::Low || target.grounded())
        {
            hits.push((side, f.action, false));
        }
    }
    // Both attacks are captured before either is applied, allowing genuine trades.
    for (side, action, projectile) in hits {
        resolve_hit(g, side, action, projectile);
    }
    update_projectiles(g);

    // Extend the displayed combo only for the actual victim's stun/airborne state.
    for side in 0..2 {
        if g.fighters[1 - side].stun == 0
            && g.fighters[1 - side].y == 0
            && g.fighters[1 - side].knockdown == 0
            && g.fighters[side].combo_timer > 35
        {
            g.fighters[side].combo_timer = 35;
        }
    }
    if g.mode == Mode::Training {
        for f in &mut g.fighters {
            f.health = f.health.max(1);
        }
        if g.fight_tick.is_multiple_of(15) {
            g.fighters[0].meter = (g.fighters[0].meter + 2).min(100);
        }
    } else if g.remaining <= 0 || g.fighters.iter().any(|f| f.health <= 0) {
        g.finish_round();
    }
}

fn begin(f: &mut Fighter, action: Action) {
    f.action_id += 1;
    f.action = action;
    f.action_tick = 0;
    f.connected = false;
    f.emitted = false;
    f.blocking = false;
    f.crouch = action == Action::Sweep;
    if action == Action::Super {
        f.meter -= 100;
        f.invulnerable = 18;
    }
    if action == Action::Uppercut {
        f.invulnerable = 6;
    }
}
fn can_cancel(from: Action, to: Action) -> bool {
    matches!(
        (from, to),
        (
            Action::Light,
            Action::Chain | Action::Heavy | Action::Special | Action::Uppercut | Action::Super
        ) | (
            Action::Chain,
            Action::Heavy | Action::Special | Action::Uppercut | Action::Super
        ) | (
            Action::Heavy | Action::Air,
            Action::Special | Action::Uppercut | Action::Super
        ) | (Action::Special | Action::Uppercut, Action::Super)
    )
}

fn pushboxes(g: &mut Game) {
    let (left, right) = g.fighters.split_at_mut(1);
    let a = &mut left[0];
    let b = &mut right[0];
    if (a.y - b.y).abs() > 105 * FP {
        return;
    }
    let distance = (a.x - b.x).abs();
    if distance < 50 * FP {
        let sign = if a.x <= b.x { 1 } else { -1 };
        let push = (50 * FP - distance + 1) / 2;
        a.x = (a.x - sign * push).clamp(45 * FP, 915 * FP);
        b.x = (b.x + sign * push).clamp(45 * FP, 915 * FP);
        let missing = 50 * FP - (a.x - b.x).abs();
        if missing > 0 {
            if a.x == 45 * FP || a.x == 915 * FP {
                b.x += sign * missing;
            } else {
                a.x -= sign * missing;
            }
        }
    }
}

fn emit_projectiles(g: &mut Game) {
    for side in 0..2 {
        let f = &mut g.fighters[side];
        if f.action == Action::Special
            && matches!(f.character, 0 | 2 | 4)
            && f.action_tick >= move_for(f.character, f.action).startup
            && !f.emitted
        {
            f.emitted = true;
            g.projectiles.push(Projectile {
                owner: side,
                x: f.x + f.facing * 65 * FP,
                y: f.y + 80 * FP,
                vx: f.facing
                    * match f.character {
                        2 => 1400,
                        4 => 650,
                        _ => 850,
                    },
                life: 100,
                super_shot: false,
                action_id: f.action_id,
            });
        }
    }
}

fn update_projectiles(g: &mut Game) {
    let mut projectiles = std::mem::take(&mut g.projectiles);
    // Opposing shots cancel before either can damage a fighter.
    for a in 0..projectiles.len() {
        for b in (a + 1)..projectiles.len() {
            if projectiles[a].owner != projectiles[b].owner
                && (projectiles[a].x - projectiles[b].x).abs() < 38 * FP
                && (projectiles[a].y - projectiles[b].y).abs() < 30 * FP
            {
                projectiles[a].life = 0;
                projectiles[b].life = 0;
            }
        }
    }
    for p in &mut projectiles {
        if p.life <= 0 {
            continue;
        }
        p.x += p.vx;
        p.life -= 1;
        let target = &g.fighters[1 - p.owner];
        let hurt_height = if target.crouch { 60 } else { 145 };
        if (p.x - target.x).abs() <= 38 * FP
            && p.y >= target.y + 15 * FP
            && p.y <= target.y + hurt_height * FP
            && target.invulnerable == 0
            && target.knockdown == 0
        {
            let owner = &mut g.fighters[p.owner];
            if owner.action == Action::Special && owner.action_id == p.action_id {
                owner.connected = true;
            }
            resolve_hit(g, p.owner, Action::Special, true);
            p.life = 0;
        }
    }
    projectiles.retain(|p| p.life > 0 && p.x > -30 * FP && p.x < 990 * FP);
    g.projectiles = projectiles;
}

fn resolve_hit(g: &mut Game, side: usize, action: Action, projectile: bool) {
    let target_side = 1 - side;
    if action == Action::Throw && g.fighters[target_side].invulnerable > 0 {
        return;
    }
    let m = move_for(g.fighters[side].character, action);
    let tech = action == Action::Throw
        && g.fighters[target_side].action == Action::Throw
        && g.fighters[target_side].action_tick <= 10;
    if tech {
        for f in &mut g.fighters {
            f.action = Action::Idle;
            f.stun = 14;
            f.invulnerable = 18;
            f.connected = true;
        }
        g.notice("THROW ESCAPE");
        return;
    }
    if !projectile {
        g.fighters[side].connected = true;
    }
    let target = &g.fighters[target_side];
    let blocked = target.blocking
        && target.grounded()
        && match m.height {
            Height::Mid => true,
            Height::Low => target.crouch,
            Height::Overhead => !target.crouch,
            Height::Throw => false,
        };
    let counter = !blocked
        && target.action != Action::Idle
        && target.action != Action::Dash
        && target.action_tick < move_for(target.character, target.action).startup;
    let victim_still_in_combo = target.stun > 0 || target.y > 0 || target.knockdown > 0;
    let previous_hits = if victim_still_in_combo {
        g.fighters[side].combo_hits
    } else {
        0
    };
    let scale = (100 - previous_hits * 12).max(35);
    let armored = m.height != Height::Throw
        && target.character == 3
        && target.action == Action::Special
        && target.action_tick < move_for(3, Action::Special).startup;
    let damage = m.damage * ROSTER[g.fighters[side].character].power / 100 * scale / 100
        * if counter { 115 } else { 100 }
        / 100
        / if armored { 2 } else { 1 };
    let direction = if g.fighters[target_side].x >= g.fighters[side].x {
        1
    } else {
        -1
    };
    let x = g.fighters[target_side].x;
    let y = g.fighters[target_side].y
        + if m.height == Height::Low {
            30 * FP
        } else {
            90 * FP
        };
    if blocked {
        let target = &mut g.fighters[target_side];
        // Chip cannot kill; each move deals damage at most once.
        target.health = (target.health
            - if matches!(action, Action::Special | Action::Super) {
                damage / 10
            } else {
                0
            })
        .max(1);
        target.guard -= if action == Action::Super { 35 } else { 15 };
        target.stun = (m.stun / 2).max(8);
        target.blockstun = true;
        target.flash = 6;
        target.x = (target.x + direction * 12 * FP).clamp(45 * FP, 915 * FP);
        target.meter = (target.meter + 4).min(100);
        if target.guard <= 0 {
            target.guard = 50;
            target.stun = 65;
            target.blocking = false;
            target.blockstun = false;
            g.notice("GUARD BREAK");
        }
        g.hitstop = 3;
        g.effects.push(Effect {
            x,
            y,
            life: 12,
            kind: 1,
            side,
        });
    } else {
        let airborne = !g.fighters[target_side].grounded();
        let target = &mut g.fighters[target_side];
        target.health = (target.health - damage).max(0);
        target.last_damage = damage;
        target.flash = 10;
        if !armored {
            target.stun = m.stun + if counter { 6 } else { 0 };
            target.blockstun = false;
            target.knockdown = m.knockdown;
            target.action = Action::Idle;
            target.action_tick = 0;
        }
        target.blocking = false;
        target.crouch = false;
        target.buffer_timer = 0;
        target.buffered = Action::Idle;
        target.meter = (target.meter + 8).min(100);
        if m.launch > 0 && !armored {
            target.vy = m.launch;
            target.y = target.y.max(1);
        } else if airborne {
            target.vy = target.vy.max(500);
        }
        target.x = (target.x + direction * if action == Action::Throw { 45 } else { 12 } * FP)
            .clamp(45 * FP, 915 * FP);
        let f = &mut g.fighters[side];
        f.meter = (f.meter + if action == Action::Super { 0 } else { 12 }).min(100);
        if previous_hits == 0 {
            f.combo_damage = 0;
        }
        f.combo_hits = previous_hits + 1;
        f.combo_damage += damage;
        f.combo_timer = 100;
        g.hitstop = if action == Action::Super { 12 } else { 6 };
        g.shake = if action == Action::Super { 18 } else { 7 };
        g.effects.push(Effect {
            x,
            y,
            life: if action == Action::Super { 24 } else { 16 },
            kind: if action == Action::Super { 2 } else { 0 },
            side,
        });
        if armored {
            g.notice("IRON ARMOR");
        } else if g.fighters[side].character == 4 && action == Action::Special {
            g.notice("FROZEN");
        } else if counter {
            g.notice("COUNTER HIT");
        } else if action == Action::Super {
            g.notice("NEON BREAKER");
        }
    }
}
