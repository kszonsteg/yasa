use super::ValuePolicyTrait;
use crate::model::constants::ARENA_WIDTH;
use crate::model::enums::Procedure;
use crate::model::game::GameState;
use crate::model::player::Player;
use crate::model::position::Square;

pub struct HeuristicValuePolicy;

impl HeuristicValuePolicy {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }

    /// Evaluate the ball carrier's position relative to the target endzone.
    /// Returns a score in the range [0.0, 1.0] where:
    /// - 1.0 = carrier is at the target endzone
    /// - 0.0 = carrier is at the opposite endzone
    fn evaluate_carrier_endzone_distance(
        &self,
        carrier_position: &Square,
        target_x: i32,
        _max_endzone_distance: f64,
    ) -> f64 {
        let endzone_distance = (carrier_position.x - target_x).abs() as f64;
        0.985 - 0.03 * endzone_distance
    }

    /// Evaluate how well players are supporting the ball carrier.
    /// Returns a score in the range [0.0, 0.1] per player where:
    /// - 0.1 = player is at optimal support distance (close to carrier)
    /// - 0.0 = player is far from the carrier
    fn evaluate_player_support(
        &self,
        player_position: &Square,
        carrier_position: &Square,
        max_field_distance: f64,
    ) -> f64 {
        let distance_to_carrier = player_position.distance(carrier_position) as f64;
        // Closer support is better, but not too close (3-5 squares ideal for support)
        if distance_to_carrier <= 5.0 {
            0.1 * (1.0 - distance_to_carrier / 5.0)
        } else {
            0.05 * (1.0 - distance_to_carrier / max_field_distance)
        }
    }

    /// Evaluate the team's offensive position when they have the ball.
    /// Returns a score in the range [0.0, 1.0] where:
    /// - Higher scores indicate better offensive positioning
    /// - Includes carrier's proximity to endzone and team support
    fn evaluate_offensive_position(
        &self,
        current_team_players: &[&Player],
        carrier: &Player,
        carrier_position: &Square,
        target_x: i32,
        max_field_distance: f64,
        max_endzone_distance: f64,
    ) -> Result<f64, String> {
        let mut carrier_score = 0.0;
        let mut support_score = 0.0;

        for player in current_team_players {
            let player_pos = player
                .position
                .as_ref()
                .ok_or("Evaluation: Player has no position")?;

            if player.player_id == carrier.player_id {
                // Ball carrier - high score for being close to endzone
                carrier_score = self.evaluate_carrier_endzone_distance(
                    carrier_position,
                    target_x,
                    max_endzone_distance,
                );
            } else {
                // Other players - score for protecting/supporting the carrier
                support_score +=
                    self.evaluate_player_support(player_pos, carrier_position, max_field_distance);
            }
        }

        // Final score is dominated by the carrier's position
        // Support score is averaged and added as a small bonus (max 0.01)
        let avg_support = if current_team_players.len() > 1 {
            (support_score / (current_team_players.len() - 1) as f64) * 0.01
        } else {
            0.0
        };

        Ok(carrier_score + avg_support)
    }

    /// Evaluate the team's defensive position when the enemy has the ball.
    /// Returns a score in the range [-1.0, 0.0] where:
    /// - Lower scores indicate enemy is closer to our endzone
    /// - Higher scores (closer to 0) indicate better defensive positioning
    fn evaluate_defensive_position(
        &self,
        current_team_players: &[&Player],
        carrier_position: &Square,
        target_x: i32,
        max_field_distance: f64,
    ) -> Result<f64, String> {
        let mut team_score = 0.0;

        // Our endzone is at the opposite side of our target endzone
        let our_endzone_x = if target_x == 1 { ARENA_WIDTH - 1 } else { 1 };

        let enemy_distance_to_our_endzone = (carrier_position.x - our_endzone_x).abs() as f64;
        // Enemy closeness to our endzone is bad for us (negative score)
        let base_defensive_score = -(0.99 - 0.03 * enemy_distance_to_our_endzone);

        for player in current_team_players {
            let player_pos = player
                .position
                .as_ref()
                .ok_or("Evaluation: Player has no position")?;

            let distance_to_carrier = player_pos.distance(carrier_position) as f64;
            // Being close to an enemy carrier is good for defense
            team_score += 0.4 * (1.0 - distance_to_carrier / max_field_distance);
        }

        // Normalize and add base defensive score
        let avg_defense = team_score / current_team_players.len() as f64;
        Ok(base_defensive_score + avg_defense * 0.1)
    }

    /// Evaluate the team's position when the ball is on the ground.
    /// Returns a score in the range [0.0, 0.3] where:
    /// - Higher scores indicate players are closer to the loose ball
    /// - 0.0 = base score when the ball is on ground
    fn evaluate_loose_ball_position(
        &self,
        current_team_players: &[&Player],
        ball_position: &Square,
        max_field_distance: f64,
    ) -> Result<f64, String> {
        let mut team_score = 0.0;

        for player in current_team_players {
            let player_pos = player
                .position
                .as_ref()
                .ok_or("Evaluation: Player has no position")?;

            let distance_to_ball = player_pos.distance(ball_position) as f64;
            // Being close to the ball is good
            team_score += 0.3 * (1.0 - distance_to_ball / max_field_distance);
        }

        // Normalize by number of players
        Ok(team_score / current_team_players.len() as f64)
    }

    /// Evaluate a game state from the perspective of the current team.
    /// Combines multiple evaluation aspects into a final score.
    /// Returns a score in the range [-1.0, 1.0] where:
    /// - 1.0 = definitely winning
    /// - 0.0 = neutral/even
    /// - -1.0 = definitely losing
    pub fn evaluate(&self, state: &GameState) -> Result<f64, String> {
        if state.procedure == Some(Procedure::Touchdown) {
            return Ok(1.0);
        }

        let ball_position = state.get_ball_position()?;
        let ball_carrier = state.get_ball_carrier();

        // Get the current team to determine which endzone to target
        let current_team = state.get_current_team().ok_or_else(|| {
            format!(
                "Evaluation: No current team - current_team_id: {:?}, home_team exists: {}, away_team exists: {}, home_team_id: {:?}, away_team_id: {:?}",
                state.current_team_id,
                state.home_team.is_some(),
                state.away_team.is_some(),
                state.home_team.as_ref().map(|t| &t.team_id),
                state.away_team.as_ref().map(|t| &t.team_id)
            )
        })?;

        let is_home_team = state
            .home_team
            .as_ref()
            .map(|t| t.team_id == current_team.team_id)
            .unwrap_or(false);

        // Determine target endzone
        let target_x = if is_home_team {
            1 // Home team targets left endzone
        } else {
            ARENA_WIDTH - 1 // Away team targets right endzone
        };

        // Get all players from the current team on the pitch
        let current_team_players = state.get_players_on_pitch(&current_team.team_id, true);

        if current_team_players.is_empty() {
            return Err("Evaluation: No players on pitch for current team".to_string());
        }

        let max_field_distance = (ARENA_WIDTH + 17) as f64; // Rough max distance on field
        let max_endzone_distance = ARENA_WIDTH as f64;

        let total_score = if ball_carrier.is_ok() {
            let carrier = ball_carrier?;
            // Someone carries a Ball
            let carrier_position = carrier
                .position
                .as_ref()
                .ok_or("Evaluation: Carrier has no position")?;
            let carrier_on_our_team =
                *state.get_player_team_id(&carrier.player_id)? == current_team.team_id;

            if carrier_on_our_team {
                // Our team has the ball-evaluate offensive position
                self.evaluate_offensive_position(
                    &current_team_players,
                    carrier,
                    carrier_position,
                    target_x,
                    max_field_distance,
                    max_endzone_distance,
                )?
            } else {
                // Enemy has the ball - evaluated defensive position
                self.evaluate_defensive_position(
                    &current_team_players,
                    carrier_position,
                    target_x,
                    max_field_distance,
                )?
            }
        } else {
            // Ball is on the ground - evaluate loose ball positioning
            self.evaluate_loose_ball_position(
                &current_team_players,
                &ball_position,
                max_field_distance,
            )?
        };

        // Apply unused player penalty to get the final score
        let final_score = total_score;

        Ok(final_score.clamp(-1.0, 1.0))
    }
}

impl ValuePolicyTrait for HeuristicValuePolicy {
    fn evaluate(&self, state: &GameState) -> Result<f64, String> {
        // Call inherent method explicitly to avoid name resolution ambiguity
        HeuristicValuePolicy::evaluate(self, state)
    }

    fn name(&self) -> &'static str {
        "heuristic"
    }
}
