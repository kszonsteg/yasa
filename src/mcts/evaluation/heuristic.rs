use super::ValuePolicyTrait;
use crate::model::constants::ARENA_WIDTH;
use crate::model::game::GameState;
use crate::model::player::Player;

pub struct HeuristicParameters {
    pub ball_carry: f64,
    pub end_zone_distance: f64,
    pub protecting_carrier: f64,
    pub standing_players: f64,
    pub knock_out: f64,
    pub enemy_player_blocked: f64,
}

impl Default for HeuristicParameters {
    fn default() -> Self {
        Self {
            end_zone_distance: 10.0,
            ball_carry: 1.0,
            protecting_carrier: 0.05,
            standing_players: 0.5,
            knock_out: 0.5,
            enemy_player_blocked: 0.2,
        }
    }
}

pub struct HeuristicValuePolicy {
    pub parameters: HeuristicParameters,
}

impl HeuristicValuePolicy {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            parameters: HeuristicParameters::default(),
        })
    }

    /// 1. Reward for getting closer to the end zone with a ball.
    /// 2. Reward for carrying the ball, or penalty if the enemy is carrying.
    /// 3. Reward for getting closer to the ball, if ball is not carried by anyone.
    fn evaluate_ball_state(
        &self,
        state: &GameState,
        current_team_id: &String,
        current_team_players: &[&Player],
        target_x: i32,
    ) -> Result<f64, String> {
        let ball_position = state.get_ball_position()?;
        let ball_carrier = state.get_ball_carrier();

        match ball_carrier {
            Ok(carrier) => {
                let carrier_team_id = state.get_player_team_id(&carrier.player_id)?;
                let is_our_carrier = carrier_team_id == current_team_id;

                if is_our_carrier {
                    let carrier_pos = carrier.position.as_ref().ok_or("Carrier has no position")?;
                    let dist = (carrier_pos.x - target_x).abs() as f64;
                    // Normalized distance score (1.0 at target, 0.0 at other side)
                    let dist_score = 1.0 - (dist / ARENA_WIDTH as f64);

                    // Component 1 & 2 (Positive)
                    Ok(self.parameters.ball_carry
                        + (self.parameters.end_zone_distance * dist_score))
                } else {
                    // Component 2 (Negative - Penalty)
                    Ok(-self.parameters.ball_carry)
                }
            }
            Err(_) => {
                // Component 3: Loose ball
                // Calculate average proximity of our team to the ball
                let mut total_proximity = 0.0;
                for player in current_team_players {
                    if let Some(pos) = &player.position {
                        let dist = pos.distance(&ball_position) as f64;
                        total_proximity += 1.0 - (dist / ARENA_WIDTH as f64);
                    }
                }

                let avg_proximity = if !current_team_players.is_empty() {
                    total_proximity / current_team_players.len() as f64
                } else {
                    0.0
                };

                // Use ball_carry weight as base for this reward as per instructions (or similar magnitude)
                Ok(self.parameters.ball_carry * avg_proximity)
            }
        }
    }

    /// 4. Reward for protecting the ball carrier (or pressuring enemy carrier).
    ///    Both cases require getting teammates close to the carrier.
    fn evaluate_carrier_proximity(
        &self,
        state: &GameState,
        _current_team_id: &String,
        current_team_players: &[&Player],
    ) -> Result<f64, String> {
        if let Ok(carrier) = state.get_ball_carrier() {
            let carrier_pos = carrier
                .position
                .as_ref()
                .ok_or("Carrier missing position")?;

            let mut proximity_score = 0.0;
            for player in current_team_players {
                // Skip the carrier himself (he is distance 0 to himself)
                if player.player_id == carrier.player_id {
                    continue;
                }

                if let Some(pos) = &player.position {
                    let dist = pos.distance(carrier_pos) as f64;
                    // "The closer to the ball, the better."
                    let max_dist = 10.0;
                    if dist < max_dist {
                        proximity_score += 1.0 - (dist / max_dist);
                    }
                }
            }

            Ok(proximity_score * self.parameters.protecting_carrier)
        } else {
            Ok(0.0)
        }
    }

    /// 5. Reward for blocking the enemy ball carrier
    /// 8. Reward for blocking the enemy players
    fn evaluate_blocks(&self, state: &GameState, current_team_id: &String) -> Result<f64, String> {
        let mut score = 0.0;

        // Identify enemy team
        let home_is_current = state.is_home_team(current_team_id);
        let enemy_team = if home_is_current {
            state.away_team.as_ref()
        } else {
            state.home_team.as_ref()
        };

        if let Some(enemy_team) = enemy_team {
            let ball_carrier_id = state.get_ball_carrier().ok().map(|p| p.player_id.clone());

            for enemy in enemy_team.players_by_id.values() {
                if !enemy.state.up || enemy.state.knocked_out || enemy.position.is_none() {
                    continue;
                }
                let enemy_pos = enemy.position.as_ref().unwrap();

                // Check if blocked by at least one of our players
                // "Only one player counts"

                let blockers = state.get_adjacent_opponents(&enemy_team.team_id, enemy_pos)?;

                if !blockers.is_empty() {
                    // Component 8: Reward for blocking enemy
                    score += self.parameters.enemy_player_blocked;

                    // Component 5: Reward for blocking carrier
                    if let Some(carrier_id) = &ball_carrier_id {
                        if &enemy.player_id == carrier_id {
                            score += self.parameters.enemy_player_blocked;
                        }
                    }
                }
            }
        }

        Ok(score)
    }

    /// 6. Penalty for not standing or knocked out players from the team
    /// 7. Reward for not standing or knocked out players from the enemy team
    fn evaluate_team_states(&self, state: &GameState, current_team_id: &String) -> f64 {
        let mut score = 0.0;

        let teams = [
            (&state.home_team, &state.home_dugout),
            (&state.away_team, &state.away_dugout),
        ];

        for (team_opt, _dugout_opt) in teams.iter() {
            if let Some(team) = team_opt {
                let is_us = &team.team_id == current_team_id;

                for player in team.players_by_id.values() {
                    // Knocked out
                    if player.state.knocked_out {
                        if is_us {
                            score -= self.parameters.knock_out;
                        } else {
                            score += self.parameters.knock_out;
                        }
                        continue; // KO implies not standing, but we separate the penalties usually
                    }

                    // Not standing (Down or Stunned) but on pitch
                    if player.position.is_some() && !player.state.up {
                        if is_us {
                            score -= self.parameters.standing_players;
                        } else {
                            score += self.parameters.standing_players;
                        }
                    }
                }
            }
        }

        score
    }

    pub fn evaluate(&self, state: &GameState) -> Result<f64, String> {
        let current_team = state
            .get_current_team()
            .ok_or("Evaluation: No current team!")?;
        let current_team_id = &current_team.team_id;

        let current_team_players = state.get_players_on_pitch(current_team_id, false); // Include down players for some checks if needed, but mostly we filter

        let is_home_team = state.is_home_team(current_team_id);
        let target_x = if is_home_team { 1 } else { ARENA_WIDTH - 1 };

        // 1, 2, 3
        let ball_score =
            self.evaluate_ball_state(state, current_team_id, &current_team_players, target_x)?;

        // 4 (and proximity aspect of 5)
        let protection_score =
            self.evaluate_carrier_proximity(state, current_team_id, &current_team_players)?;

        // 5 (Block contact aspect), 8
        let block_score = self.evaluate_blocks(state, current_team_id)?;

        // 6, 7
        let team_state_score = self.evaluate_team_states(state, current_team_id);

        let total_score = ball_score + protection_score + block_score + team_state_score;

        // Normalize to (-1.0, 1.0) using tanh
        // Scaling factor to prevent saturation too early.
        // Max theoretical score is roughly 20-30.
        // tanh(3.0) is ~0.995. So we want 30 to map to ~3.0.
        // Scaling factor 10.0.
        Ok((total_score / 10.0).tanh())
    }
}

impl ValuePolicyTrait for HeuristicValuePolicy {
    fn evaluate(&self, state: &GameState) -> Result<f64, String> {
        HeuristicValuePolicy::evaluate(self, state)
    }

    fn name(&self) -> &'static str {
        "heuristic"
    }
}
