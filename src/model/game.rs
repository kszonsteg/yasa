use crate::model::constants::{ARENA_HEIGHT, ARENA_WIDTH};
use crate::model::player::Player;
use crate::model::position::Square;

use super::action::Action;
use super::ball::Ball;
use super::enums::{ActionType, Procedure, WeatherType};
use super::team::{Dugout, Team};

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct TurnState {
    pub blitz: bool,
    pub quick_snap: bool,
    pub blitz_available: bool,
    pub pass_available: bool,
    pub foul_available: bool,
    pub handoff_available: bool,
}

impl Default for TurnState {
    fn default() -> Self {
        TurnState {
            blitz: false,
            quick_snap: false,
            blitz_available: true,
            pass_available: true,
            foul_available: true,
            handoff_available: true,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct GameState {
    // basic game info
    pub half: u8,
    pub round: u8,
    pub game_over: bool,
    pub weather: WeatherType,
    #[serde(default)]
    pub balls: Vec<Ball>,
    // team info
    pub home_team: Option<Team>,
    pub home_dugout: Option<Dugout>,
    pub away_team: Option<Team>,
    pub away_dugout: Option<Dugout>,
    pub kicking_first_half: Option<String>,
    pub receiving_first_half: Option<String>,
    pub kicking_this_drive: Option<String>,
    pub receiving_this_drive: Option<String>,
    pub coin_toss_winner: Option<String>,
    // turn state
    pub turn_state: Option<TurnState>,
    // procedure
    #[serde(default)]
    pub procedure: Option<Procedure>,
    pub current_team_id: Option<String>,
    pub active_player_id: Option<String>,
    pub rolls: Vec<ActionType>,
    pub chain_push: Option<bool>, // Indicates if the last push was part of a chain
    pub attacker: Option<String>, // Player ID of the player who pushed
    pub defender: Option<String>, // Player ID of the player who was pushed
    pub position: Option<Vec<i32>>, // Position of the player which is blocked
    #[serde(default)]
    pub available_actions: Vec<Action>,
}

impl Default for GameState {
    fn default() -> Self {
        GameState {
            half: 1,
            round: 0,
            game_over: false,
            weather: WeatherType::default(),
            home_team: None,
            away_team: None,
            kicking_first_half: None,
            receiving_first_half: None,
            kicking_this_drive: None,
            receiving_this_drive: None,
            current_team_id: None,
            active_player_id: None,
            balls: Vec::new(),
            home_dugout: None,
            away_dugout: None,
            available_actions: Vec::new(),
            procedure: None,
            turn_state: None,
            coin_toss_winner: None,
            rolls: Vec::new(),
            chain_push: None,
            attacker: None,
            defender: None,
            position: None,
        }
    }
}

impl GameState {
    pub fn from_json(state: &str) -> Result<GameState, serde_json::Error> {
        let game_state: GameState = serde_json::from_str(state)?;
        Ok(game_state)
    }

    pub fn is_home_team(&self, team_id: &String) -> bool {
        if let Some(home_team) = &self.home_team {
            home_team.team_id == *team_id
        } else {
            false
        }
    }

    pub fn is_team_side(&self, position: &Square, team_id: &String) -> bool {
        if self.is_home_team(team_id) {
            position.x >= ARENA_WIDTH / 2
        } else {
            position.x < ARENA_WIDTH / 2
        }
    }

    pub fn get_adjacent_opponents(
        &self,
        team_id: &String,
        player_postion: &Square,
    ) -> Result<Vec<&Player>, String> {
        let mut opponents = vec![];
        if let Some(opp_team) = if self.is_home_team(team_id) {
            &self.away_team
        } else {
            &self.home_team
        } {
            for player in opp_team.players_by_id.values() {
                if let Some(opp_position) = player.position {
                    if opp_position.distance(player_postion) == 1 {
                        opponents.push(player);
                    }
                } else {
                    return Err("Missing player position".to_string());
                }
            }
        } else {
            return Err("Missing opp team".to_string());
        }

        Ok(opponents)
    }

    pub fn get_ball_position(&self) -> Result<Square, String> {
        self.balls
            .first()
            .and_then(|ball| ball.position)
            .ok_or("Missing ball on field".to_string())
    }

    pub fn get_player(&self, player_id: &String) -> Result<&Player, String> {
        if let Some(home_team) = &self.home_team {
            for player in home_team.players_by_id.values() {
                if player.player_id == *player_id {
                    return Ok(player);
                }
            }
        }

        if let Some(away_team) = &self.away_team {
            for player in away_team.players_by_id.values() {
                if player.player_id == *player_id {
                    return Ok(player);
                }
            }
        }

        Err(format!("No player with id {player_id:?}"))
    }

    pub fn get_player_at(&self, position: &Square) -> Result<&Player, String> {
        if let Some(home_team) = &self.home_team {
            for player in home_team.players_by_id.values() {
                if let Some(player_pos) = &player.position {
                    if player_pos.x == position.x && player_pos.y == position.y {
                        return Ok(player);
                    }
                }
            }
        }

        if let Some(away_team) = &self.away_team {
            for player in away_team.players_by_id.values() {
                if let Some(player_pos) = &player.position {
                    if player_pos.x == position.x && player_pos.y == position.y {
                        return Ok(player);
                    }
                }
            }
        }

        Err(format!("No player at position {position:?}"))
    }

    pub fn get_active_player(&self) -> Result<&Player, String> {
        let active_player_id = self
            .active_player_id
            .as_ref()
            .ok_or("Missing active player.".to_string())?;

        self.get_player(active_player_id)
    }

    pub fn get_player_team_id(&self, player_id: &String) -> Result<&String, String> {
        if let Some(home_team) = &self.home_team {
            for player in home_team.players_by_id.values() {
                if player.player_id == *player_id {
                    return Ok(&home_team.team_id);
                }
            }
        }

        if let Some(away_team) = &self.away_team {
            for player in away_team.players_by_id.values() {
                if player.player_id == *player_id {
                    return Ok(&away_team.team_id);
                }
            }
        }

        Err(format!("No player with id {player_id:?}"))
    }

    pub fn get_receiving_team_side_positions(&self) -> Vec<Square> {
        let mut positions = vec![];
        if let Some(receiving_team_id) = &self.receiving_this_drive {
            let (x_start, x_end) = if self.is_home_team(receiving_team_id) {
                (ARENA_WIDTH / 2, ARENA_WIDTH - 2)
            } else {
                (1, ARENA_WIDTH / 2 - 1)
            };
            for y in 1..ARENA_HEIGHT - 1 {
                for x in x_start..=x_end {
                    positions.push(Square::new(x, y))
                }
            }
        };
        positions
    }

    /// Get the number of tackle zones at a specific position for a team
    pub fn get_team_tackle_zones_at(&self, team_id: &String, position: &Square) -> usize {
        let mut tackle_zones = 0;

        // Get the opposing team
        let opp_team = if self.is_home_team(team_id) {
            &self.away_team
        } else {
            &self.home_team
        };

        if let Some(team) = opp_team {
            for opponent in team.players_by_id.values() {
                if let Some(opp_pos) = &opponent.position {
                    // Check if the opponent is adjacent (within tackle zone range)
                    if position.distance(opp_pos) == 1
                        && opponent.state.up
                        && !opponent.state.stunned
                    {
                        tackle_zones += 1;
                    }
                }
            }
        }

        tackle_zones
    }
}
