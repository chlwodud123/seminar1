use crate::physics::{
    MatchWorld, PlayerInput, Side, BALL_GRAVITY, BALL_GRAVITY_HALF, BALL_RADIUS, BLOBBY_GROUND_Y,
    BLOBBY_SPEED, FIELD_WIDTH, GROUND_PLANE_Y, NET_RADIUS, NET_TOP_NORMAL_FACTOR,
    NET_TOP_TANGENT_FACTOR, NET_TOP_Y, NET_X,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BotDifficulty {
    Weak,
    Medium,
    Strong,
}

impl BotDifficulty {
    pub fn label(self) -> &'static str {
        match self {
            BotDifficulty::Weak => "Weak",
            BotDifficulty::Medium => "Medium",
            BotDifficulty::Strong => "Strong",
        }
    }
}

pub struct BotController {
    rng_state: u64,
    serve_offset: f32,
    jump_delay: u32,
    cached_noise: f32,
    last_ball_vx_sign: i32,
}

impl BotController {
    pub fn new(seed: u64) -> Self {
        Self {
            rng_state: seed.max(1),
            serve_offset: 0.0,
            jump_delay: 0,
            cached_noise: 0.0,
            last_ball_vx_sign: 0,
        }
    }

    fn next_rand_01(&mut self) -> f32 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        ((x >> 11) as f32) / ((1u64 << 53) as f32)
    }

    pub fn decide(&mut self, world: &MatchWorld, side: Side, difficulty: BotDifficulty) -> PlayerInput {
        let blob = &world.blobs[side.index()];

        // Refresh slight randomness whenever ball changes horizontal direction
        let vx_sign = if world.ball.vx > 0.2 {
            1
        } else if world.ball.vx < -0.2 {
            -1
        } else {
            0
        };
        if vx_sign != self.last_ball_vx_sign {
            self.last_ball_vx_sign = vx_sign;
            let jitter_amp = match difficulty {
                BotDifficulty::Weak => 34.0,
                BotDifficulty::Medium => 14.0,
                BotDifficulty::Strong => 4.0,
            };
            self.cached_noise = (self.next_rand_01() - 0.5) * 2.0 * jitter_amp;
        }

        // Convert to normalized left-side coordinates so bot logic is completely symmetric
        let to_local_x = |x: f32| match side {
            Side::Left => x,
            Side::Right => FIELD_WIDTH - x,
        };
        let to_local_vx = |vx: f32| match side {
            Side::Left => vx,
            Side::Right => -vx,
        };

        let my_x = to_local_x(blob.x);
        let my_y = blob.y;
        let bx = to_local_x(world.ball.x);
        let by = world.ball.y;
        let bvx = to_local_vx(world.ball.vx);
        let bvy = world.ball.vy;

        // 1. Serving / between rallies
        if !world.is_game_running {
            if world.serving_side == side && world.is_ball_valid {
                if self.serve_offset == 0.0 {
                    // Aim slightly behind ball so it arcs cleanly over the net
                    let spread = match difficulty {
                        BotDifficulty::Weak => 16.0,
                        BotDifficulty::Medium => 12.0,
                        BotDifficulty::Strong => 8.0,
                    };
                    self.serve_offset = -12.0 - self.next_rand_01() * spread;
                    self.jump_delay = 14 + (self.next_rand_01() * 14.0) as u32;
                }
                let target_x = bx + self.serve_offset;
                let mut inp = move_toward_local(my_x, target_x, side);
                if !blob.is_grounded() {
                    // Hold jump while rising to reach full jump height and strike the serve cleanly
                    inp.jump = blob.vy < 0.0;
                } else if (my_x - target_x).abs() <= BLOBBY_SPEED {
                    if self.jump_delay > 0 {
                        self.jump_delay -= 1;
                    } else {
                        inp.jump = true;
                    }
                }
                return inp;
            } else {
                self.serve_offset = 0.0;
                // Wait in ready stance near center of own half
                return move_toward_local(my_x, 185.0, side);
            }
        }
        self.serve_offset = 0.0;

        // 2. Active rally: simulate ball trajectory forward to find landing & spike points
        let high_hit_y = 290.0;
        let low_hit_y = BLOBBY_GROUND_Y - 52.0; // ~403.5 (head contact standing)

        let high_pred = simulate_ball_to_y(bx, by, bvx, bvy, high_hit_y, 180);
        let low_pred = simulate_ball_to_y(bx, by, bvx, bvy, low_hit_y, 240);

        let mut want_jump = false;
        let target_x;

        // Decide whether to attempt an attacking jump-spike (inspired by scripts/reduced.lua)
        let allow_spike = match difficulty {
            BotDifficulty::Weak => false,
            BotDifficulty::Medium => world.touches[side.index()] < 2,
            BotDifficulty::Strong => world.touches[side.index()] < 3,
        };

        if allow_spike {
            if let Some((hx, h_vx, h_t)) = high_pred {
                let spike_x = (hx - 24.0 - h_vx.min(0.0) * 1.5).clamp(48.0, NET_X - 42.0);
                let travel_t = (my_x - spike_x).abs() / BLOBBY_SPEED;
                if hx < NET_X - 18.0 && hx > 70.0 && h_t > travel_t + 10.0 && h_t < 65.0 {
                    target_x = spike_x + self.cached_noise * 0.4;
                    let jump_window = match difficulty {
                        BotDifficulty::Strong => 26.0,
                        _ => 22.0,
                    };
                    if h_t <= jump_window && (my_x - spike_x).abs() < 34.0 {
                        want_jump = true;
                    }
                    let mut inp = move_toward_local(my_x, target_x, side);
                    inp.jump = want_jump || (!blob.is_grounded() && by < my_y - 30.0);
                    return inp;
                }
            }
        }

        // Otherwise play defensively / set up the ball
        if let Some((lx, _l_vx, l_t)) = low_pred {
            if lx < NET_X - 8.0 {
                // Position slightly behind ball to push it toward opponent's side
                let push_offset = match difficulty {
                    BotDifficulty::Weak => -6.0,
                    BotDifficulty::Medium => -14.0,
                    BotDifficulty::Strong => {
                        if world.touches[side.index()] >= 2 {
                            -22.0 // send it over on 3rd touch!
                        } else {
                            -14.0
                        }
                    }
                };
                target_x = (lx + push_offset + self.cached_noise).clamp(38.0, NET_X - 42.0);

                // Emergency save jump if ball is close and dropping fast
                if l_t < 18.0 && (my_x - lx).abs() < 55.0 && bx < NET_X {
                    if difficulty != BotDifficulty::Weak && (lx > 315.0 || lx < 80.0 || l_t < 12.0) {
                        want_jump = true;
                    }
                }
            } else {
                // Ball is heading to opponent's court; recover to defensive center
                target_x = 180.0 + self.cached_noise * 0.5;
            }
        } else {
            target_x = 180.0;
        }

        let mut inp = move_toward_local(my_x, target_x, side);
        inp.jump = want_jump || (!blob.is_grounded() && blob.vy < 0.0 && by < my_y);
        inp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::{Ruleset, Side};

    #[test]
    fn ai_serves_and_rallies_across_net() {
        let mut world = MatchWorld::new(Ruleset::Default, 15);
        let mut left_bot = BotController::new(0xBEEF_1234);
        let mut right_bot = BotController::new(0xCAFE_5678);
        let mut crossed_net = false;
        let mut ever_running = false;
        for _ in 0..600 {
            let l = left_bot.decide(&world, Side::Left, BotDifficulty::Medium);
            let r = right_bot.decide(&world, Side::Right, BotDifficulty::Medium);
            world.step(l, r);
            if world.is_game_running {
                ever_running = true;
            }
            if world.ball.x > NET_X + 40.0 {
                crossed_net = true;
            }
        }
        assert!(ever_running, "AI should serve and start the rally");
        assert!(crossed_net, "Ball should cross over the net to the right side");
    }
}

fn move_toward_local(my_x: f32, target_x: f32, side: Side) -> PlayerInput {
    let mut inp = PlayerInput::default();
    let deadzone = BLOBBY_SPEED * 0.6;
    let move_left_local = my_x > target_x + deadzone;
    let move_right_local = my_x < target_x - deadzone;
    match side {
        Side::Left => {
            inp.left = move_left_local;
            inp.right = move_right_local;
        }
        Side::Right => {
            inp.left = move_right_local;
            inp.right = move_left_local;
        }
    }
    inp
}

/// Simulates ball forward in local (left-side) coordinates until descending through `target_y`.
/// Returns `(x, vx, ticks)` when it crosses `target_y` going downward.
fn simulate_ball_to_y(
    mut x: f32,
    mut y: f32,
    mut vx: f32,
    mut vy: f32,
    target_y: f32,
    max_ticks: u32,
) -> Option<(f32, f32, f32)> {
    let min_net_dist = BALL_RADIUS + NET_RADIUS;
    for t in 1..=max_ticks {
        x += vx;
        y += vy + BALL_GRAVITY_HALF;
        vy += BALL_GRAVITY;

        // Walls (exact elastic 1.0 from 0x00412D03..0x00412DB8)
        if x - BALL_RADIUS <= 0.0 && vx < 0.0 {
            x = BALL_RADIUS;
            vx = -vx;
        } else if x + BALL_RADIUS >= FIELD_WIDTH && vx > 0.0 {
            x = FIELD_WIDTH - BALL_RADIUS;
            vx = -vx;
        }

        // Net shaft (exact elastic 1.0) & top sphere (anisotropic 0.9 / 0.7 from 0x00412E46)
        if y > NET_TOP_Y && (x - NET_X).abs() < min_net_dist {
            vx = -vx;
            x = if x > NET_X {
                NET_X + min_net_dist
            } else {
                NET_X - min_net_dist
            };
        } else {
            let dx = NET_X - x;
            let dy = NET_TOP_Y - y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < min_net_dist && d > 1e-6 {
                let nx = dx / d;
                let ny = dy / d;
                let v_par = vx * nx + vy * ny;
                let v_perp_sq = (vx * vx + vy * vy - v_par * v_par).max(0.0);
                let new_speed = ((v_perp_sq as f64) * NET_TOP_TANGENT_FACTOR
                    + ((v_par * v_par) as f64) * NET_TOP_NORMAL_FACTOR)
                    .max(0.0)
                    .sqrt() as f32;
                let rx = vx - 2.0 * v_par * nx;
                let ry = vy - 2.0 * v_par * ny;
                let rl = (rx * rx + ry * ry).sqrt();
                if rl > 1e-6 {
                    vx = (rx / rl) * new_speed;
                    vy = (ry / rl) * new_speed;
                }
                x = NET_X - nx * min_net_dist;
                y = NET_TOP_Y - ny * min_net_dist;
            }
        }

        if vy > 0.0 && y >= target_y {
            return Some((x, vx, t as f32));
        }
        if y + BALL_RADIUS >= GROUND_PLANE_Y {
            return Some((x, vx, t as f32));
        }
    }
    None
}
