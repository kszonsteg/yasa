use yasa_core::model::ball::Ball;
use yasa_core::model::enums::Procedure;
use yasa_core::model::game::GameState;
use yasa_core::model::player::Player;
use yasa_core::model::position::Square;
use yasa_core::model::team::Team;

pub const HOME_PLAYER_ID: &str = "home_player_id";
pub const HOME_TEAM_ID: &str = "home_team_id";
pub const AWAY_PLAYER_ID: &str = "away_player_id";
pub const AWAY_TEAM_ID: &str = "away_team_id";

pub fn game_state_setup(
    home_x: i32,
    home_y: i32,
    away_x: i32,
    away_y: i32,
    ball_x: i32,
    ball_y: i32,
) -> GameState {
    let mut home_team = Team::new(HOME_TEAM_ID.to_string());

    let home_player = Player {
        player_id: HOME_PLAYER_ID.to_string(),
        position: Some(Square {
            x: home_x,
            y: home_y,
        }),
        ma: 4,
        ag: 3,
        ..Default::default()
    };

    home_team
        .players_by_id
        .insert(home_player.player_id.clone(), home_player);

    let mut away_team = Team::new(AWAY_TEAM_ID.to_string());

    let away_player = Player {
        player_id: AWAY_PLAYER_ID.to_string(),
        position: Some(Square {
            x: away_x,
            y: away_y,
        }),
        ma: 4,
        ..Default::default()
    };

    away_team
        .players_by_id
        .insert(away_player.player_id.clone(), away_player);

    let ball = Ball::new(
        Some(Square {
            x: ball_x,
            y: ball_y,
        }),
        false,
    );

    GameState {
        home_team: Some(home_team),
        away_team: Some(away_team),
        current_team_id: Some(HOME_TEAM_ID.to_string()),
        procedure: Some(Procedure::Turn),
        balls: vec![ball],
        ..Default::default()
    }
}
