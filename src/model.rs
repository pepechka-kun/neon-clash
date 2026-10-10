use aige::Resource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const FP: i32 = 100;
pub const FLOOR: i32 = 425;
pub const MAX_HEALTH: i32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Phase {
    Title,
    Select,
    Intro,
    Fight,
    RoundOver,
    MatchOver,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Mode {
    Arcade,
    Versus,
    Training,
    Demo,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Action {
    #[default]
    Idle,
    Light,
    Chain,
    Heavy,
    Sweep,
    Air,
    Special,
    Uppercut,
    Throw,
    Super,
    Dash,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Height {
    Mid,
    Low,
    Overhead,
    Throw,
}

pub struct Character {
    pub name: &'static str,
    pub title: &'static str,
    pub style: &'static str,
    pub special: &'static str,
    pub color: [f32; 4],
    pub speed: i32,
    pub power: i32,
}

pub const ROSTER: [Character; 6] = [
    Character {
        name: "CINDER",
        title: "THE EXILE",
        style: "FIRE / BALANCED",
        special: "EMBER SHOT",
        color: [1.0, 0.34, 0.16, 1.0],
        speed: 390,
        power: 100,
    },
    Character {
        name: "VESPER",
        title: "NIGHT ASSASSIN",
        style: "SHADOW / RUSH",
        special: "SHADOW STEP",
        color: [0.71, 0.39, 1.0, 1.0],
        speed: 480,
        power: 90,
    },
    Character {
        name: "KAIRO",
        title: "STORM DISCIPLE",
        style: "VOLT / ZONER",
        special: "ARC BOLT",
        color: [0.18, 0.85, 1.0, 1.0],
        speed: 410,
        power: 95,
    },
    Character {
        name: "ROOK",
        title: "IRON SENTINEL",
        style: "STEEL / GRAPPLER",
        special: "IRON CHARGE",
        color: [1.0, 0.72, 0.2, 1.0],
        speed: 310,
        power: 120,
    },
    Character {
        name: "NYX",
        title: "FROST WARDEN",
        style: "ICE / CONTROL",
        special: "FROST SHARD",
        color: [0.35, 1.0, 0.78, 1.0],
        speed: 360,
        power: 100,
    },
    Character {
        name: "ECHO",
        title: "SYNTH REBEL",
        style: "PULSE / MIXUP",
        special: "PULSE DRIVE",
        color: [1.0, 0.3, 0.65, 1.0],
        speed: 440,
        power: 95,
    },
];
pub const ARENAS: [&str; 3] = ["RAIN DISTRICT", "RED SANCTUM", "REACTOR ZERO"];

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Fighter {
    pub character: usize,
    pub x: i32,
    /// Height above the ground, positive upwards; all physics is integer fixed point.
    pub y: i32,
    pub vy: i32,
    pub facing: i32,
    pub health: i32,
    pub meter: i32,
    pub guard: i32,
    pub crouch: bool,
    pub blocking: bool,
    pub blockstun: bool,
    pub moving: bool,
    pub walk_tick: i32,
    pub stun: i32,
    pub knockdown: i32,
    pub invulnerable: i32,
    pub action: Action,
    pub action_tick: i32,
    pub action_id: u64,
    pub connected: bool,
    pub emitted: bool,
    pub buffered: Action,
    pub buffer_timer: i32,
    pub combo_hits: i32,
    pub combo_damage: i32,
    pub combo_timer: i32,
    pub down_tick: u64,
    pub forward_tick: u64,
    pub last_forward: u64,
    pub was_forward: bool,
    pub flash: i32,
    pub ai_wait: i32,
    pub last_damage: i32,
}
impl Fighter {
    pub fn new(character: usize, side: usize) -> Self {
        Self {
            character,
            x: (if side == 0 { 320 } else { 640 }) * FP,
            y: 0,
            vy: 0,
            facing: if side == 0 { 1 } else { -1 },
            health: MAX_HEALTH,
            meter: 0,
            guard: 100,
            crouch: false,
            blocking: false,
            blockstun: false,
            moving: false,
            walk_tick: 0,
            stun: 0,
            knockdown: 0,
            invulnerable: 0,
            action: Action::Idle,
            action_tick: 0,
            action_id: 0,
            connected: false,
            emitted: false,
            buffered: Action::Idle,
            buffer_timer: 0,
            combo_hits: 0,
            combo_damage: 0,
            combo_timer: 0,
            down_tick: 0,
            forward_tick: 0,
            last_forward: 0,
            was_forward: false,
            flash: 0,
            ai_wait: 0,
            last_damage: 0,
        }
    }
    pub fn free(&self) -> bool {
        self.stun == 0 && self.knockdown == 0
    }
    pub fn grounded(&self) -> bool {
        self.y == 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Projectile {
    pub owner: usize,
    pub x: i32,
    pub y: i32,
    pub vx: i32,
    pub life: i32,
    pub super_shot: bool,
    pub action_id: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Effect {
    pub x: i32,
    pub y: i32,
    pub life: i32,
    pub kind: u8,
    pub side: usize,
}
#[derive(Clone, Debug, Resource, Serialize, Deserialize, JsonSchema)]
#[resource(name = "game")]
pub struct Game {
    pub phase: Phase,
    pub mode: Mode,
    pub menu: usize,
    pub selected: [usize; 2],
    pub select_slot: usize,
    pub arena: usize,
    pub difficulty: usize,
    pub dummy: usize,
    pub arcade_stage: usize,
    pub fighters: [Fighter; 2],
    pub wins: [u8; 2],
    pub round: u8,
    pub remaining: i32,
    pub phase_timer: i32,
    pub winner: i32,
    pub frame: u64,
    pub fight_tick: u64,
    pub hitstop: i32,
    pub shake: i32,
    pub paused: bool,
    pub help: bool,
    pub projectiles: Vec<Projectile>,
    pub effects: Vec<Effect>,
    pub notification: String,
    pub notice_timer: i32,
    pub rng: u64,
}
impl Default for Game {
    fn default() -> Self {
        Self {
            phase: Phase::Title,
            mode: Mode::Arcade,
            menu: 0,
            selected: [0, 1],
            select_slot: 0,
            arena: 0,
            difficulty: 1,
            dummy: 0,
            arcade_stage: 0,
            fighters: [Fighter::new(0, 0), Fighter::new(1, 1)],
            wins: [0, 0],
            round: 1,
            remaining: 60 * 60,
            phase_timer: 0,
            winner: -1,
            frame: 0,
            fight_tick: 0,
            hitstop: 0,
            shake: 0,
            paused: false,
            help: false,
            projectiles: Vec::new(),
            effects: Vec::new(),
            notification: String::new(),
            notice_timer: 0,
            rng: 0xC1A5_2026,
        }
    }
}
impl Game {
    pub fn random(&mut self) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 32) as u32
    }
    pub fn start_match(&mut self) {
        if self.mode == Mode::Arcade {
            self.selected[1] = (self.selected[0] + 1 + self.arcade_stage) % 6;
        }
        self.wins = [0, 0];
        self.round = 1;
        self.winner = -1;
        self.reset_round();
    }
    pub fn reset_round(&mut self) {
        self.fighters = [
            Fighter::new(self.selected[0], 0),
            Fighter::new(self.selected[1], 1),
        ];
        if self.mode == Mode::Training {
            self.fighters[0].meter = 100;
        }
        self.remaining = 60 * 60;
        self.phase = Phase::Intro;
        self.phase_timer = 90;
        self.projectiles.clear();
        self.effects.clear();
        self.hitstop = 0;
        self.shake = 0;
        self.notification.clear();
        self.notice_timer = 0;
    }
    pub fn notice(&mut self, s: &str) {
        self.notification = s.to_owned();
        self.notice_timer = 75;
    }
    pub fn finish_round(&mut self) {
        self.winner = match self.fighters[0].health.cmp(&self.fighters[1].health) {
            std::cmp::Ordering::Greater => 0,
            std::cmp::Ordering::Less => 1,
            std::cmp::Ordering::Equal => -1,
        };
        if self.winner >= 0 {
            self.wins[self.winner as usize] += 1;
        }
        // Draws award neither side; replay the round instead of ending a tied match.
        self.phase = Phase::RoundOver;
        self.phase_timer = 150;
        self.projectiles.clear();
    }
}
