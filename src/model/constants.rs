pub const ARENA_WIDTH: i32 = 28;
pub const ARENA_HEIGHT: i32 = 17;

// Blood Bowl Agility Table
// Index by agility value (1-6), returns target number needed on d6
// AG 0 is unused but included for direct indexing
pub const AGILITY_TABLE: [u8; 7] = [
    6, // AG 0 (unused, for indexing)
    6, // AG 1: need 6+
    5, // AG 2: need 5+
    4, // AG 3: need 4+
    3, // AG 4: need 3+
    2, // AG 5: need 2+
    1, // AG 6+: need 1+ (auto-success before modifiers)
];

// GFI (Go For It) target numbers on d6
pub const GFI_TARGET_NORMAL: u8 = 2; // 2+ on d6
pub const GFI_TARGET_BLIZZARD: u8 = 3; // 3+ on d6 in blizzard

// Maximum GFI attempts per turn
pub const MAX_GFI: u8 = 2;

pub const PASS_MATRIX: [[u8; 14]; 14] = [
    [0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4],
    [1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4],
    [1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5],
    [1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5],
    [2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5],
    [2, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5],
    [2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5],
    [3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5],
    [3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5],
    [3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5, 5],
    [3, 3, 3, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5],
    [4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5],
    [4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5],
    [4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5],
];
