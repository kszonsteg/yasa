use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;

pub fn reroll_discovery(game_state: &mut GameState) -> Result<(), String> {
    game_state.available_actions = vec![
        Action::new(ActionType::UseReroll, None, None),
        Action::new(ActionType::DontUseReroll, None, None),
    ];
    Ok(())
}

pub fn ejection_discovery(game_state: &mut GameState) -> Result<(), String> {
    let current_team_id = game_state
        .current_team_id
        .as_ref()
        .ok_or("Ejection without current team id")?;

    let team = if game_state.is_home_team(current_team_id) {
        game_state
            .home_team
            .as_ref()
            .ok_or("No home team in ejection")?
    } else {
        game_state
            .away_team
            .as_ref()
            .ok_or("No away team in ejection")?
    };

    if team.bribes > 0 {
        game_state.available_actions = vec![
            Action::new(ActionType::UseBribe, None, None),
            Action::new(ActionType::DontUseBribe, None, None),
        ]
    } else {
        game_state.available_actions = vec![Action::new(ActionType::DontUseBribe, None, None)]
    }

    Ok(())
}
