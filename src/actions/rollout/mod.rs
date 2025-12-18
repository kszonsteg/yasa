pub mod model;

use crate::model::constants::ARENA_WIDTH;
use crate::model::enums::Procedure;
use crate::model::game::GameState;
use crate::model::position::Square;

pub fn gfi_rollout(game_state: &GameState) -> Result<Vec<model::RolloutOutcome>, String> {
    let mut outcomes = vec![];

    // Successful GFI
    let mut state = game_state.clone();

    let position_vec = state
        .position
        .as_ref()
        .ok_or("Missing position in GFI rollout")?;
    let position = Square {
        x: position_vec[0],
        y: position_vec[1],
    };
    let active_player = state.get_active_player_mut()?;
    active_player.state.moves += 1;
    active_player.position = Some(position);

    if let Ok(ball_position) = state.get_ball_position() {
        if ball_position == position {
            state.balls[0].is_carried = true;
        }
    }

    if state.is_active_player_carrying_ball() {
        state.balls[0].position = Some(position);

        let is_home = state.is_home_team(
            state
                .current_team_id
                .as_ref()
                .ok_or("GFI rollout cannot check if home team")?,
        );
        let is_touchdown = if is_home {
            position.x == 1
        } else {
            position.x == ARENA_WIDTH - 1
        };

        if is_touchdown {
            state.procedure = Some(Procedure::Touchdown);

            let team = if is_home {
                state.home_team.as_mut()
            } else {
                state.away_team.as_mut()
            };

            team.ok_or("Missing team for touchdown")?.score += 1;
        }
    }
    outcomes.push(model::RolloutOutcome::new(5.0 / 6.0, state));

    // Unsuccessful GFI
    let mut state = game_state.clone();

    let position_vec = state
        .position
        .as_ref()
        .ok_or("Missing position in GFI rollout")?;
    let position = Square {
        x: position_vec[0],
        y: position_vec[1],
    };
    let active_player = state.get_active_player_mut()?;
    active_player.position = Some(position);
    active_player.state.up = false;
    state.procedure = Some(Procedure::Turnover);
    // TODO: ball fumble, injuries etc.
    outcomes.push(model::RolloutOutcome::new(1.0 / 6.0, state));

    Ok(outcomes)
}
