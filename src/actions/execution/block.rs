use crate::model::action::Action;
use crate::model::block::{BlockContext, PushChainItem};
use crate::model::constants::ARENA_WIDTH;
use crate::model::enums::Procedure;
use crate::model::game::GameState;
use crate::model::position::Square;

pub fn block_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    game_state.procedure = Some(Procedure::BlockRoll);
    if let Some(position) = action.position() {
        game_state.block_context = Some(BlockContext::new(
            game_state
                .active_player_id
                .clone()
                .ok_or("Missing active player in Block".to_string())?,
            game_state.get_player_at(position)?.player_id.clone(),
            *position,
        ));
        Ok(())
    } else {
        Err("No position at block execution.".to_string())
    }
}

pub fn select_attacker_down_execution(game_state: &mut GameState) -> Result<(), String> {
    game_state.procedure = Some(Procedure::Turnover);
    game_state.get_active_player_mut()?.state.knocked_out = true;
    Ok(())
}

pub fn select_both_down_execution(game_state: &mut GameState) -> Result<(), String> {
    let defender_id = game_state
        .block_context
        .as_ref()
        .ok_or("Missing block context in both down execution")?
        .defender
        .clone();

    game_state.procedure = Some(Procedure::Turnover);
    game_state.get_active_player_mut()?.state.knocked_out = true;

    let defender_player = game_state.get_player_mut(&defender_id)?;
    defender_player.state.knocked_out = true;

    Ok(())
}

pub fn select_push_execution(game_state: &mut GameState) -> Result<(), String> {
    game_state.procedure = Some(Procedure::Push);
    if let Some(ref mut block_ctx) = game_state.block_context {
        block_ctx.knock_out = false;
        block_ctx.push_chain.push(PushChainItem::new(
            block_ctx.attacker.clone(),
            block_ctx.defender.clone(),
            None,
        ));
        Ok(())
    } else {
        Err("Missing block context in push execution".to_string())
    }
}

pub fn select_defender_stumbles_execution(game_state: &mut GameState) -> Result<(), String> {
    game_state.procedure = Some(Procedure::Push);

    if let Some(ref mut block_ctx) = game_state.block_context {
        block_ctx.knock_out = true;
        block_ctx.push_chain.push(PushChainItem::new(
            block_ctx.attacker.clone(),
            block_ctx.defender.clone(),
            None,
        ));
        Ok(())
    } else {
        Err("Missing block context in Stumbles execution".to_string())
    }
}

pub fn select_defender_down_execution(game_state: &mut GameState) -> Result<(), String> {
    game_state.procedure = Some(Procedure::Push);

    if let Some(ref mut block_ctx) = game_state.block_context {
        block_ctx.knock_out = true;
        block_ctx.push_chain.push(PushChainItem::new(
            block_ctx.attacker.clone(),
            block_ctx.defender.clone(),
            None,
        ));
        Ok(())
    } else {
        Err("Missing block context in Defender Down execution".to_string())
    }
}

pub fn push_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    let position = action.position().ok_or("No position at push execution.")?;

    update_latest_push_position(game_state, position)?;

    if position.is_out_of_bounds() {
        Err("Position out of bounds in push execution.".to_string())
    } else if let Ok(player) = game_state.get_player_at(&position) {
        add_chained_push(game_state, player.player_id.clone())?;
        Ok(())
    } else {
        execute_push_chain(game_state)?;
        if game_state.procedure != Some(Procedure::Touchdown) {
            game_state.procedure = Some(Procedure::FollowUp);
        }
        Ok(())
    }
}

fn update_latest_push_position(game_state: &mut GameState, position: Square) -> Result<(), String> {
    let block_ctx = game_state
        .block_context
        .as_mut()
        .ok_or("Missing block context")?;
    let latest = block_ctx
        .push_chain
        .last_mut()
        .ok_or("Missing last value")?;
    latest.position = Some(position);
    Ok(())
}

fn add_chained_push(game_state: &mut GameState, player_id: String) -> Result<(), String> {
    let latest_defender = game_state
        .block_context
        .as_ref()
        .ok_or("No Block context in add chained push".to_string())?
        .push_chain
        .last()
        .ok_or("Missing last element in add chained push".to_string())?
        .defender
        .clone();

    game_state
        .block_context
        .as_mut()
        .ok_or("No Block context in add chained push".to_string())?
        .push_chain
        .push(PushChainItem::new(latest_defender, player_id, None));
    Ok(())
}

fn execute_push_chain(game_state: &mut GameState) -> Result<(), String> {
    let (push_items, knock_out, defender_id) = {
        let block_ctx = game_state
            .block_context
            .as_ref()
            .ok_or("Missing block context in push chain execution.")?;
        let items: Vec<_> = block_ctx
            .push_chain
            .iter()
            .map(|item| (item.defender.clone(), item.position))
            .collect();
        (items, block_ctx.knock_out, block_ctx.defender.clone())
    };

    for (defender_id, pos) in push_items.iter().rev() {
        let mut ball_moved = false;
        if let Ok(player) = game_state.get_player(defender_id) {
            if let Some(old_pos) = player.position {
                if let Ok(ball_pos) = game_state.get_ball_position() {
                    if game_state.is_ball_carried() && ball_pos == old_pos {
                        ball_moved = true;
                    }
                }
            }
        }

        let player = game_state.get_player_mut(defender_id)?;
        player.position = *pos;

        if ball_moved {
            if let Some(ball) = game_state.balls.first_mut() {
                ball.position = *pos;
            }
        }
    }

    if knock_out {
        let defender = game_state.get_player_mut(&defender_id)?;
        defender.state.knocked_out = true;
    }

    let td_info = if let Ok(carrier) = game_state.get_ball_carrier() {
        if !carrier.state.knocked_out && !carrier.state.stunned && carrier.state.up {
            if let Some(carrier_pos) = carrier.position {
                let team_id = game_state.get_player_team_id(&carrier.player_id)?.clone();
                let is_home = game_state.is_home_team(&team_id);
                let is_touchdown = if is_home {
                    carrier_pos.x == 1
                } else {
                    carrier_pos.x == ARENA_WIDTH - 1
                };
                if is_touchdown {
                    Some(is_home)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some(is_home) = td_info {
        game_state.procedure = Some(Procedure::Touchdown);
        let team = if is_home {
            game_state.home_team.as_mut()
        } else {
            game_state.away_team.as_mut()
        };
        if let Some(team) = team {
            team.score += 1;
        }
    }

    Ok(())
}

pub fn follow_up_execution(game_state: &mut GameState, action: &Action) -> Result<(), String> {
    let position = action
        .position()
        .ok_or("Missing position in follow up execution.")?;

    let is_blitz = game_state.parent_procedure == Some(Procedure::BlitzAction);

    let active_player = game_state.get_active_player_mut()?;
    active_player.state.has_blocked = true;
    active_player.position = Some(position);

    // follow-up ends the player turn if it is not a blitz
    if is_blitz {
        game_state.procedure = Some(Procedure::BlitzAction);
    } else {
        active_player.state.used = true;
        game_state.procedure = Some(Procedure::Turn);
    }
    game_state.block_context = None;
    Ok(())
}
