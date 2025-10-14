use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
pub enum PlayerRole {
    Blitzer,
    Catcher,
    Lineman,
    Thrower,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Skill {
    Block,
    Catch,
    Dodge,
    Pass,
    SureHands,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
pub enum Procedure {
    Armor,
    BlitzAction,
    Block,
    BlockAction,
    Bounce,
    Casualty,
    Catch,
    CoinTossFlip,
    CoinTossKickReceive,
    Dodge,
    Ejection,
    EndGame,
    EndPlayerTurn,
    EndTurn,
    FollowUp,
    Foul,
    FoulAction,
    GFI,
    Half,
    Handoff,
    HandoffAction,
    HighKick,
    Injury,
    Intercept,
    Interception,
    Kickoff,
    KickoffTable,
    KnockDown,
    KnockOut,
    Move,
    MoveAction,
    PassAction,
    PassAttempt,
    Pickup,
    PlaceBall,
    Push,
    Reroll,
    Setup,
    StandUp,
    StartGame,
    Touchback,
    Touchdown,
    Turn,
    Turnover,
    WeatherTable,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionType {
    Block,
    Continue,
    DontUseApothecary,
    DontUseBribe,
    DontUseReroll,
    EndPlayerTurn,
    EndSetup,
    EndTurn,
    FollowUp,
    Foul,
    Handoff,
    Heads,
    Kick,
    Move,
    Pass,
    PlaceBall,
    PlacePlayer,
    Push,
    Receive,
    SelectAttackerDown,
    SelectBothDown,
    SelectDefenderDown,
    SelectDefenderStumbles,
    SelectFirstRoll,
    SelectNone,
    SelectPlayer,
    SelectPush,
    SelectSecondRoll,
    SetupFormationLine,
    SetupFormationSpread,
    SetupFormationWedge,
    SetupFormationZone,
    StandUp,
    StartBlitz,
    StartBlock,
    StartFoul,
    #[default]
    StartGame,
    StartHandoff,
    StartMove,
    StartPass,
    Tails,
    UseBribe,
    UseReroll,
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WeatherType {
    Blizzard,
    #[default]
    Nice,
    PouringRain,
    SwelteringHeat,
    VerySunny,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Hash)]
pub enum PassDistance {
    QuickPass,
    ShortPass,
    LongPass,
    LongBomb,
    HailMary,
}
