use super::action::Action;
use super::ball::Ball;
use super::block::BlockContext;
use super::constants::{ARENA_HEIGHT, ARENA_WIDTH, PASS_MATRIX};
use super::enums::{ActionType, PassDistance, Procedure, WeatherType};
use super::player::Player;
use super::position::Square;
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
    #[serde(default)]
    pub parent_procedure: Option<Procedure>,
    pub current_team_id: Option<String>,
    pub active_player_id: Option<String>,
    pub rolls: Vec<ActionType>,
    #[serde(default)]
    pub block_context: Option<BlockContext>, // Context for block procedures including chain pushes
    pub position: Option<Square>, // Position for non-block procedures (dodge, GFI, interception)
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
            parent_procedure: None,
            turn_state: Some(TurnState::default()),
            coin_toss_winner: None,
            rolls: Vec::new(),
            position: None,
            block_context: None,
        }
    }
}

impl GameState {
    pub fn from_json(state: &str) -> Result<GameState, serde_json::Error> {
        let game_state: GameState = serde_json::from_str(state)?;
        Ok(game_state)
    }

    pub fn get_current_team(&self) -> Option<&Team> {
        if let Some(current_team_id) = &self.current_team_id {
            if let Some(home_team) = &self.home_team {
                if &home_team.team_id == current_team_id {
                    return Some(home_team);
                }
            }
            if let Some(away_team) = &self.away_team {
                if &away_team.team_id == current_team_id {
                    return Some(away_team);
                }
            }
        }
        None
    }

    pub fn is_ball_carried(&self) -> bool {
        if !self.balls.is_empty() {
            let ball = &self.balls[0];
            ball.is_carried
        } else {
            false
        }
    }

    pub fn is_active_player_carrying_ball(&self) -> bool {
        if self.is_ball_carried() {
            let player = self.get_active_player();
            match player {
                Ok(player) => {
                    let ball = &self.balls[0];
                    player.position == ball.position
                }
                Err(_) => false,
            }
        } else {
            false
        }
    }

    pub fn is_home_team(&self, team_id: &String) -> bool {
        if let Some(home_team) = &self.home_team {
            home_team.team_id == *team_id
        } else {
            false
        }
    }

    pub fn is_current_team_home(&self) -> bool {
        self.current_team_id
            .as_ref()
            .map(|id| self.is_home_team(id))
            .unwrap_or(false)
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

    pub fn get_adjacent_teammates(
        &self,
        team_id: &String,
        player_postion: &Square,
    ) -> Result<Vec<&Player>, String> {
        let mut teammates = vec![];
        if let Some(team) = if self.is_home_team(team_id) {
            &self.home_team
        } else {
            &self.away_team
        } {
            for player in team.players_by_id.values() {
                if let Some(opp_position) = player.position {
                    if opp_position.distance(player_postion) == 1 {
                        teammates.push(player);
                    }
                } else {
                    return Err("Missing player position".to_string());
                }
            }
        } else {
            return Err("Missing opp team".to_string());
        }

        Ok(teammates)
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

    pub fn get_player_mut(&mut self, player_id: &String) -> Result<&mut Player, String> {
        if let Some(home_team) = &mut self.home_team {
            for player in home_team.players_by_id.values_mut() {
                if player.player_id == *player_id {
                    return Ok(player);
                }
            }
        }

        if let Some(away_team) = &mut self.away_team {
            for player in away_team.players_by_id.values_mut() {
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

    pub fn get_active_player_mut(&mut self) -> Result<&mut Player, String> {
        let active_player_id = self
            .active_player_id
            .as_ref()
            .ok_or("Missing active player.".to_string())?
            .clone();

        self.get_player_mut(&active_player_id)
    }

    pub fn get_ball_carrier(&self) -> Result<&Player, String> {
        let ball_position = self.get_ball_position()?;
        self.get_player_at(&ball_position)
    }

    pub fn get_players_on_pitch(&self, team_id: &str, up_only: bool) -> Vec<&Player> {
        let mut players = Vec::new();

        let team = if self
            .home_team
            .as_ref()
            .map(|team| team.team_id == *team_id)
            .unwrap_or(false)
        {
            &self.home_team
        } else {
            &self.away_team
        };

        if let Some(team) = team {
            for player in team.players_by_id.values() {
                // Player must have a position (be on pitch) and optionally be standing
                if player.position.is_some() && (!up_only || player.state.up) {
                    players.push(player);
                }
            }
        }

        players
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

    pub fn get_pass_distance(
        &self,
        from_position: &Square,
        to_position: &Square,
    ) -> Result<PassDistance, String> {
        let distance_x = (from_position.x - to_position.x).unsigned_abs() as usize;
        let distance_y = (from_position.y - to_position.y).unsigned_abs() as usize;

        let distance = PASS_MATRIX[distance_y][distance_x];

        match distance {
            1 => Ok(PassDistance::QuickPass),
            2 => Ok(PassDistance::ShortPass),
            3 => Ok(PassDistance::LongPass),
            4 => Ok(PassDistance::LongBomb),
            5 => Ok(PassDistance::HailMary),
            _ => Err("Wrong Pass distance".to_string()),
        }
    }

    pub fn get_pass_distances_at(
        &self,
        position: &Square,
    ) -> Result<(Vec<Square>, Vec<PassDistance>), String> {
        let mut squares = Vec::new();
        let mut distances = Vec::new();

        let distances_allowed: Vec<PassDistance> = if self.weather == WeatherType::Blizzard {
            vec![PassDistance::QuickPass, PassDistance::ShortPass]
        } else {
            vec![
                PassDistance::QuickPass,
                PassDistance::ShortPass,
                PassDistance::LongPass,
                PassDistance::LongBomb,
            ]
        };

        for y in position.y.saturating_sub(13)..=position.y.saturating_add(13) {
            if y <= 0 || y >= ARENA_HEIGHT - 1 {
                continue;
            }

            for x in position.x.saturating_sub(13)..=position.x.saturating_add(13) {
                if x <= 0 || x >= ARENA_WIDTH - 1 {
                    continue;
                }

                let to_position = Square::new(x, y);
                if position == &to_position {
                    continue;
                }
                let distance = self.get_pass_distance(position, &to_position)?;
                if distances_allowed.contains(&distance) {
                    squares.push(to_position);
                    distances.push(distance);
                }
            }
        }

        Ok((squares, distances))
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
