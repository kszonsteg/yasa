use super::action::Action;
use super::ball::Ball;
use super::enums::{ActionType, Procedure, WeatherType};
use super::team::Team;

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
pub struct Dugout {
    pub team_id: String,
    pub reserves: Vec<String>, // Player IDs
    pub kod: Vec<String>,      // Knocked out players
    pub dungeon: Vec<String>,  // Ejected players
}

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
