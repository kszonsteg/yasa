use crate::model::enums::ActionType;
use crate::model::position::Square;
use crate::pathfinding::Path;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Action {
    action_type: ActionType,
    player: Option<String>,
    position: Option<Square>,
    #[serde(skip)]
    path: Option<Path>,
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        self.action_type == other.action_type
            && self.player == other.player
            && self.position == other.position
    }
}

impl Eq for Action {}

impl Hash for Action {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.action_type.hash(state);
        self.player.hash(state);
        self.position.hash(state);
    }
}

impl Action {
    pub fn new(action_type: ActionType, player: Option<String>, position: Option<Square>) -> Self {
        Action {
            action_type,
            player,
            position,
            path: None,
        }
    }

    pub fn new_with_path(
        action_type: ActionType,
        player: Option<String>,
        _position: Option<Square>,
        path: Path,
    ) -> Self {
        Action {
            action_type,
            player,
            position: Some(path.target),
            path: Some(path),
        }
    }

    pub fn action_type(&self) -> ActionType {
        self.action_type
    }

    pub fn player(&self) -> &Option<String> {
        &self.player
    }

    pub fn position(&self) -> &Option<Square> {
        &self.position
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref()
    }

    /// Get the probability of successfully completing this action.
    /// For Move actions with paths, returns the path probability.
    /// For other actions, returns 1.0.
    pub fn success_probability(&self) -> f64 {
        self.path.as_ref().map(|p| p.prob).unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_new() {
        let action = Action::new(ActionType::Move, None, Some(Square::new(5, 5)));
        assert_eq!(action.action_type(), ActionType::Move);
        assert!(action.path().is_none());
        assert_eq!(action.success_probability(), 1.0);
    }

    #[test]
    fn test_action_new_with_path() {
        let mut path = Path::new(Square::new(8, 5));
        path.squares = vec![Square::new(6, 5), Square::new(7, 5), Square::new(8, 5)];
        path.prob = 0.75;
        path.moves_used = 3;

        let action = Action::new_with_path(ActionType::Move, None, None, path);

        assert_eq!(action.action_type(), ActionType::Move);
        assert_eq!(action.position(), &Some(Square::new(8, 5)));
        assert!(action.path().is_some());
        assert_eq!(action.success_probability(), 0.75);
    }

    #[test]
    fn test_action_equality_ignores_path() {
        let mut path = Path::new(Square::new(8, 5));
        path.prob = 0.5;

        let action1 = Action::new(ActionType::Move, None, Some(Square::new(8, 5)));
        let action2 = Action::new_with_path(ActionType::Move, None, Some(Square::new(8, 5)), path);

        // Actions should be equal even with different path metadata
        assert_eq!(action1, action2);
    }

    #[test]
    fn test_action_hash_ignores_path() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut path = Path::new(Square::new(8, 5));
        path.prob = 0.5;

        let action1 = Action::new(ActionType::Move, None, Some(Square::new(8, 5)));
        let action2 = Action::new_with_path(ActionType::Move, None, Some(Square::new(8, 5)), path);

        let mut hasher1 = DefaultHasher::new();
        action1.hash(&mut hasher1);

        let mut hasher2 = DefaultHasher::new();
        action2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }
}
