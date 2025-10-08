use super::enums::{PlayerRole, Skill};
use super::position::Square;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub up: bool,
    pub used: bool,
    pub moves: u8,
    pub stunned: bool,
    pub knocked_out: bool,
    pub squares_moved: Vec<Square>,
    pub has_blocked: bool,
}

impl Default for PlayerState {
    fn default() -> Self {
        PlayerState {
            up: true,
            used: false,
            moves: 0,
            stunned: false,
            knocked_out: false,
            squares_moved: Vec::new(),
            has_blocked: false,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Player {
    pub player_id: String,
    pub role: PlayerRole,
    pub skills: Vec<Skill>,
    pub ma: u8,
    pub st: u8,
    pub ag: u8,
    pub av: u8,
    pub state: PlayerState,
    pub position: Option<Square>,
}

impl Default for Player {
    fn default() -> Self {
        Player {
            player_id: Uuid::new_v4().to_string(),
            role: PlayerRole::Blitzer,
            skills: vec![Skill::Block],
            ma: 7,
            st: 3,
            ag: 3,
            av: 8,
            position: None,
            state: PlayerState::default(),
        }
    }
}

impl Player {
    pub fn get_ma(&self) -> u8 {
        let ma = self.ma;
        ma.clamp(1, 10)
    }

    pub fn get_st(&self) -> u8 {
        let st = self.st;
        st.clamp(1, 10)
    }

    pub fn get_ag(&self) -> u8 {
        let ag = self.ag;
        ag.clamp(1, 10)
    }

    pub fn get_av(&self) -> u8 {
        let av = self.av;
        av.clamp(1, 10)
    }
}
