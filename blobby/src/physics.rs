#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn opponent(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ruleset {
    Default,
    Classic,
}

impl Ruleset {
    pub fn label(self) -> &'static str {
        match self {
            Ruleset::Default => "Default (Rally Point)",
            Ruleset::Classic => "Classic (Side-Out)",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameSpeed {
    VerySlow,
    Slow,
    Default,
    Fast,
    VeryFast,
}

impl GameSpeed {
    pub fn label(self) -> &'static str {
        match self {
            GameSpeed::VerySlow => "Very Slow (40%)",
            GameSpeed::Slow => "Slow (75%)",
            GameSpeed::Default => "Default (100%)",
            GameSpeed::Fast => "Fast (125%)",
            GameSpeed::VeryFast => "Very Fast (150%)",
        }
    }

    pub fn multiplier(self) -> f64 {
        match self {
            GameSpeed::VerySlow => 0.40,
            GameSpeed::Slow => 0.75,
            GameSpeed::Default => 1.00,
            GameSpeed::Fast => 1.25,
            GameSpeed::VeryFast => 1.50,
        }
    }
}

// --- Verified Court & Physics Constants (see MECHANICS.md / mechanics.json) ---
pub const FIELD_WIDTH: f32 = 800.0;
pub const FIELD_HEIGHT: f32 = 600.0;
pub const GROUND_PLANE_Y: f32 = 500.0;

pub const NET_X: f32 = FIELD_WIDTH * 0.5; // 400.0
pub const NET_RADIUS: f32 = 7.0;
pub const NET_TOP_Y: f32 = 284.0;

pub const BLOBBY_HEIGHT: f32 = 89.0;
pub const BLOBBY_UPPER_SPHERE: f32 = 19.0;
pub const BLOBBY_UPPER_RADIUS: f32 = 25.0;
pub const BLOBBY_LOWER_SPHERE: f32 = 13.0;
pub const BLOBBY_LOWER_RADIUS: f32 = 33.0;

pub const BLOBBY_GROUND_Y: f32 = GROUND_PLANE_Y - BLOBBY_HEIGHT * 0.5; // 455.5
pub const BLOBBY_MAX_JUMP_HEIGHT: f32 = BLOBBY_GROUND_Y - 206.375; // 249.125
pub const BLOBBY_JUMP_VEL: f32 = -15.1;
pub const BLOBBY_GRAVITY: f32 =
    (BLOBBY_JUMP_VEL * BLOBBY_JUMP_VEL) / BLOBBY_MAX_JUMP_HEIGHT; // 0.91524335
pub const BLOBBY_JUMP_BUFFER: f32 = BLOBBY_GRAVITY * 0.5; // 0.45762168
pub const BLOBBY_SPEED: f32 = 4.5;

pub const BALL_RADIUS: f32 = 31.5;
pub const BALL_GRAVITY: f32 = 0.287;
pub const BALL_GRAVITY_HALF: f32 = 0.1435; // 0x00489748 (0.5 * BALL_GRAVITY)
// sqrt(0.75 * 800.0 * 0.287) = sqrt(172.2) ≈ 13.1225 (data 0x004A074C)
pub const BALL_COLLISION_VELOCITY: f32 = 13.1224998;
pub const STANDARD_BALL_Y: f32 = 269.0 + BALL_RADIUS; // 300.5 (0x004129DA)
pub const STANDARD_BALL_ANGULAR_VEL: f32 = 0.1; // 0x00412A12
pub const BALL_ROT_SPEED_DIV: f32 = 6.0; // 0x00489790
pub const BALL_ROT_WRAP: f32 = 6.25; // 0x00489794

pub const BASE_TICK_HZ: f64 = 75.0;
pub const MAX_TOUCHES: u32 = 3;

// --- Disassembled Constants (blobby.exe PhysicWorld 0x00412C50 & GameLogic 0x00411A50) ---
pub const LEFT_START_X: f32 = 200.0; // 0x00412A38
pub const RIGHT_START_X: f32 = 600.0; // 0x00412A4D
pub const SQUISH_COOLDOWN_TICKS: u32 = 11; // 0x0B at 0x00411A73
pub const GROUND_DAMPING: f32 = 0.95; // 0x00489768
pub const NET_TOP_TANGENT_FACTOR: f64 = 0.9; // 0x00489778
pub const NET_TOP_NORMAL_FACTOR: f64 = 0.7; // 0x00489770
pub const ROUND_RESET_TICKS: u32 = 105; // estimated

#[derive(Clone, Copy, Default, Debug)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

#[derive(Clone, Debug)]
pub struct Blob {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub anim_phase: f32,
    pub squish_timer: u32,
    pub squash: f32, // visual squash/stretch feedback
}

impl Blob {
    pub fn new(start_x: f32) -> Self {
        Self {
            x: start_x,
            y: BLOBBY_GROUND_Y,
            vx: 0.0,
            vy: 0.0,
            anim_phase: 0.0,
            squish_timer: 0,
            squash: 0.0,
        }
    }

    pub fn is_grounded(&self) -> bool {
        self.y >= BLOBBY_GROUND_Y - 0.01
    }
}

#[derive(Clone, Debug)]
pub struct Ball {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub rotation: f32,
    pub angular_vel: f32,
}

impl Ball {
    pub fn at_serve(server: Side) -> Self {
        let x = match server {
            Side::Left => LEFT_START_X,
            Side::Right => RIGHT_START_X,
        };
        Self {
            x,
            y: STANDARD_BALL_Y,
            vx: 0.0,
            vy: 0.0,
            rotation: 0.0,
            angular_vel: STANDARD_BALL_ANGULAR_VEL,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PointBanner {
    pub text: String,
    pub timer: u32,
    pub side: Side,
}

#[derive(Clone, Debug)]
pub struct MatchWorld {
    pub blobs: [Blob; 2],
    pub ball: Ball,
    pub scores: [u32; 2],
    pub touches: [u32; 2],
    pub serving_side: Side,
    pub is_ball_valid: bool,
    pub is_game_running: bool,
    pub reset_countdown: u32,
    pub ruleset: Ruleset,
    pub score_to_win: u32,
    pub classic_first_round: bool,
    pub classic_last_server: Side,
    pub winner: Option<Side>,
    pub banner: Option<PointBanner>,
    pub tick_count: u64,
}

impl MatchWorld {
    pub fn new(ruleset: Ruleset, score_to_win: u32) -> Self {
        let mut world = Self {
            blobs: [Blob::new(LEFT_START_X), Blob::new(RIGHT_START_X)],
            ball: Ball::at_serve(Side::Left),
            scores: [0, 0],
            touches: [0, 0],
            serving_side: Side::Left,
            is_ball_valid: true,
            is_game_running: false,
            reset_countdown: 0,
            ruleset,
            score_to_win,
            classic_first_round: true,
            classic_last_server: Side::Left,
            winner: None,
            banner: None,
            tick_count: 0,
        };
        world.reset_rally(Side::Left);
        world
    }

    pub fn reset_match(&mut self, ruleset: Ruleset, score_to_win: u32) {
        self.ruleset = ruleset;
        self.score_to_win = score_to_win;
        self.scores = [0, 0];
        self.winner = None;
        self.classic_first_round = true;
        self.classic_last_server = Side::Left;
        self.banner = None;
        self.reset_rally(Side::Left);
    }

    pub fn reset_rally(&mut self, server: Side) {
        self.serving_side = server;
        self.blobs[0] = Blob::new(LEFT_START_X);
        self.blobs[1] = Blob::new(RIGHT_START_X);
        self.ball = Ball::at_serve(server);
        self.touches = [0, 0];
        self.is_ball_valid = true;
        self.is_game_running = false;
        self.reset_countdown = 0;
    }

    pub fn step(&mut self, left_in: PlayerInput, right_in: PlayerInput) {
        self.tick_count = self.tick_count.wrapping_add(1);

        if let Some(ref mut b) = self.banner {
            if b.timer > 0 {
                b.timer -= 1;
            }
            if b.timer == 0 {
                self.banner = None;
            }
        }

        if self.winner.is_some() {
            // Keep blobs gently idling on victory
            for blob in &mut self.blobs {
                blob.squash *= 0.85;
            }
            return;
        }

        // Update both blobs (PhysicWorld::handleBlob 0x00413090)
        self.step_blob(Side::Left, left_in);
        self.step_blob(Side::Right, right_in);

        // Update ball position with exact Verlet midpoint integration (PhysicWorld::step 0x004135F9)
        let ball_moving = self.is_game_running || !self.is_ball_valid;
        if ball_moving {
            self.ball.x += self.ball.vx;
            self.ball.y += self.ball.vy + BALL_GRAVITY_HALF;
            self.ball.vy += BALL_GRAVITY;
        }

        // Check ball collisions with blobs (PhysicWorld::playerBallCollision 0x00413260)
        self.handle_blob_ball_collision(Side::Left);
        self.handle_blob_ball_collision(Side::Right);

        // Walls, Net & Ground collisions (PhysicWorld::handleBallWorldCollisions 0x00412C50)
        self.handle_ball_world_collisions();

        // Exact ball angular rotation (PhysicWorld::step 0x004136C8..0x00413796)
        if !ball_moving {
            self.ball.rotation -= self.ball.angular_vel;
        } else {
            let speed = (self.ball.vx * self.ball.vx + self.ball.vy * self.ball.vy).sqrt();
            let delta = (speed / BALL_ROT_SPEED_DIV) * self.ball.angular_vel;
            if self.ball.vx > 0.0 {
                self.ball.rotation += delta;
            } else {
                self.ball.rotation -= delta;
            }
        }
        if self.ball.rotation <= 0.0 {
            self.ball.rotation += BALL_ROT_WRAP;
        } else if self.ball.rotation >= BALL_ROT_WRAP {
            self.ball.rotation -= BALL_ROT_WRAP;
        }

        // Reset countdown after a mistake
        if !self.is_ball_valid {
            if self.reset_countdown > 0 {
                self.reset_countdown -= 1;
                if self.reset_countdown == 0 && self.winner.is_none() {
                    let next_server = self.serving_side;
                    self.reset_rally(next_server);
                }
            }
        }
    }

    fn step_blob(&mut self, side: Side, input: PlayerInput) {
        let blob = &mut self.blobs[side.index()];
        let was_grounded = blob.is_grounded();

        // Horizontal movement (0x00413120..0x004131AB)
        let mut dir = 0.0f32;
        if input.left {
            dir -= 1.0;
        }
        if input.right {
            dir += 1.0;
        }
        blob.vx = dir * BLOBBY_SPEED;
        blob.x += blob.vx;

        // Jump initiation & jump-hold gravity buffer (0x0041309F..0x00413117)
        let grav = if input.jump {
            if was_grounded {
                blob.vy = BLOBBY_JUMP_VEL;
                blob.squash = -0.18;
            }
            BLOBBY_JUMP_BUFFER
        } else {
            BLOBBY_GRAVITY
        };

        // Exact Verlet vertical integration (0x004131AF..0x004131CF)
        blob.y += blob.vy + 0.5 * grav;
        blob.vy += grav;

        // Ground clamp (0x004131C7..0x00413208)
        if blob.y > BLOBBY_GROUND_Y {
            if !was_grounded && blob.vy > 3.0 {
                blob.squash = 0.22;
            }
            blob.y = BLOBBY_GROUND_Y;
            blob.vy = 0.0;
        }

        // Exact horizontal court & net clamp (0x00413658..0x004136C8)
        let (min_x, max_x) = match side {
            Side::Left => (0.0, NET_X - NET_RADIUS - BLOBBY_LOWER_RADIUS), // [0.0, 360.0]
            Side::Right => (NET_X + NET_RADIUS + BLOBBY_LOWER_RADIUS, FIELD_WIDTH), // [440.0, 800.0]
        };
        blob.x = blob.x.clamp(min_x, max_x);

        // Animation phase & timers
        if dir.abs() > 0.0 && blob.is_grounded() {
            blob.anim_phase += 0.22;
        } else {
            blob.anim_phase *= 0.92;
        }
        blob.squash *= 0.84;
        if blob.squish_timer > 0 {
            blob.squish_timer -= 1;
        }
    }

    fn handle_blob_ball_collision(&mut self, side: Side) {
        let idx = side.index();
        let blob_x = self.blobs[idx].x;
        let blob_y = self.blobs[idx].y;

        // Exact blobby.exe order (0x00413260): lower sphere first (dist^2 < 4160.25), then upper sphere (dist^2 < 3192.25)
        let lower_y = blob_y + BLOBBY_LOWER_SPHERE;
        let upper_y = blob_y - BLOBBY_UPPER_SPHERE;
        let dx = self.ball.x - blob_x;
        let dy_lower = self.ball.y - lower_y;
        let dy_upper = self.ball.y - upper_y;

        let sphere_y = if dx * dx + dy_lower * dy_lower
            < (BALL_RADIUS + BLOBBY_LOWER_RADIUS) * (BALL_RADIUS + BLOBBY_LOWER_RADIUS)
        {
            Some(lower_y)
        } else if dx * dx + dy_upper * dy_upper
            < (BALL_RADIUS + BLOBBY_UPPER_RADIUS) * (BALL_RADIUS + BLOBBY_UPPER_RADIUS)
        {
            Some(upper_y)
        } else {
            None
        };

        if let Some(sy) = sphere_y {
            let nx = self.ball.x - blob_x;
            let ny = self.ball.y - sy;
            let len = (nx * nx + ny * ny).sqrt();
            if len > 1e-6 {
                self.ball.vx = (nx / len) * BALL_COLLISION_VELOCITY;
                self.ball.vy = (ny / len) * BALL_COLLISION_VELOCITY;
            }
            // Immediate one-step separation (0x00413430..0x0041344A)
            self.ball.x += self.ball.vx;
            self.ball.y += self.ball.vy;

            self.blobs[idx].squash = 0.16;
            if self.blobs[idx].squish_timer == 0 {
                // GameLogic::onBallHitsPlayer (0x00411A50..0x00411AB0)
                self.blobs[idx].squish_timer = SQUISH_COOLDOWN_TICKS;
                self.blobs[side.opponent().index()].squish_timer = 0;
                if self.is_ball_valid {
                    if !self.is_game_running {
                        self.is_game_running = true;
                    }
                    self.touches[idx] += 1;
                    self.touches[side.opponent().index()] = 0;

                    if self.touches[idx] > MAX_TOUCHES {
                        self.commit_mistake(side, true);
                    }
                }
            }
        }
    }

    fn handle_ball_world_collisions(&mut self) {
        // 1. Ground collision (0x00412C75..0x00412CE8)
        if self.ball.y + BALL_RADIUS > GROUND_PLANE_Y {
            self.ball.vx *= GROUND_DAMPING;
            self.ball.vy = -self.ball.vy * GROUND_DAMPING;
            self.ball.y = GROUND_PLANE_Y - BALL_RADIUS;

            if self.is_ball_valid {
                let mistake_side = if self.ball.x < NET_X {
                    Side::Left
                } else {
                    Side::Right
                };
                self.commit_mistake(mistake_side, false);
            }
        }

        // 2. Left & right wall collisions (0x00412D03..0x00412DB8: exact 1.0 elastic reflection)
        if self.ball.x - BALL_RADIUS <= 0.0 && self.ball.vx < 0.0 {
            self.ball.vx = -self.ball.vx;
            self.ball.x = BALL_RADIUS;
        } else if self.ball.x + BALL_RADIUS >= FIELD_WIDTH && self.ball.vx > 0.0 {
            self.ball.vx = -self.ball.vx;
            self.ball.x = FIELD_WIDTH - BALL_RADIUS;
        }

        // 3. Vertical net post collision (0x00412DBD..0x00412E41: exact 1.0 elastic reflection)
        let min_net_dist = BALL_RADIUS + NET_RADIUS; // 38.5
        if self.ball.y > NET_TOP_Y && (self.ball.x - NET_X).abs() < min_net_dist {
            self.ball.vx = -self.ball.vx;
            if self.ball.x > NET_X {
                self.ball.x = NET_X + min_net_dist;
            } else {
                self.ball.x = NET_X - min_net_dist;
            }
            return;
        }

        // 4. Net top sphere collision (0x00412E46..0x0041306B: anisotropic energy retention 0.9 tangent / 0.7 normal)
        let dx = NET_X - self.ball.x;
        let dy = NET_TOP_Y - self.ball.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist < min_net_dist {
            let (nx, ny) = if dist > 1e-6 {
                (dx / dist, dy / dist)
            } else {
                (dx, dy)
            };
            let v_par = self.ball.vx * nx + self.ball.vy * ny;
            let v_sq = self.ball.vx * self.ball.vx + self.ball.vy * self.ball.vy;
            let v_perp_sq = (v_sq - v_par * v_par).max(0.0);

            let new_speed = ((v_perp_sq as f64) * NET_TOP_TANGENT_FACTOR
                + ((v_par * v_par) as f64) * NET_TOP_NORMAL_FACTOR)
                .max(0.0)
                .sqrt() as f32;

            let rx = self.ball.vx - 2.0 * v_par * nx;
            let ry = self.ball.vy - 2.0 * v_par * ny;
            let r_len = (rx * rx + ry * ry).sqrt();
            if r_len > 1e-6 {
                self.ball.vx = (rx / r_len) * new_speed;
                self.ball.vy = (ry / r_len) * new_speed;
            }
            self.ball.x = NET_X - nx * min_net_dist;
            self.ball.y = NET_TOP_Y - ny * min_net_dist;
        }
    }

    fn commit_mistake(&mut self, mistake_side: Side, four_touches: bool) {
        self.is_ball_valid = false;
        self.is_game_running = false;
        self.reset_countdown = ROUND_RESET_TICKS;

        let scorer = mistake_side.opponent();
        self.serving_side = scorer;

        let points_awarded = match self.ruleset {
            Ruleset::Default => 1,
            Ruleset::Classic => {
                // Classic BV2 rules: only the side that already held serve scores
                if self.classic_first_round {
                    self.classic_last_server = scorer;
                    self.classic_first_round = false;
                    0
                } else if self.classic_last_server != scorer {
                    self.classic_last_server = scorer;
                    0
                } else {
                    1
                }
            }
        };

        self.scores[scorer.index()] += points_awarded;

        let side_label = match scorer {
            Side::Left => "BLUE",
            Side::Right => "RED",
        };
        let msg = if four_touches {
            if points_awarded > 0 {
                format!("4 TOUCHES! {} POINT", side_label)
            } else {
                format!("4 TOUCHES! {} SERVES", side_label)
            }
        } else if points_awarded > 0 {
            format!("{} POINT!", side_label)
        } else {
            format!("SIDE OUT - {} SERVES", side_label)
        };

        self.banner = Some(PointBanner {
            text: msg,
            timer: ROUND_RESET_TICKS,
            side: scorer,
        });

        // Win check: >= score_to_win and lead by >= 2
        let l = self.scores[0];
        let r = self.scores[1];
        if l >= self.score_to_win && l >= r + 2 {
            self.winner = Some(Side::Left);
        } else if r >= self.score_to_win && r >= l + 2 {
            self.winner = Some(Side::Right);
        }
    }
}
