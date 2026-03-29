//! Color palette matching the original TypeScript TUI.

use iocraft::prelude::Color;

/// Soft red — user prompt arrow, errors, spinner.
pub const CRANBERRY: Color = Color::Rgb {
    r: 248,
    g: 113,
    b: 113,
};

/// Cyan-green — "ready" status, tool running indicator.
pub const TEAL: Color = Color::Rgb {
    r: 45,
    g: 212,
    b: 191,
};

/// Warm yellow — permission dialog, queued messages, history nav.
pub const GOLD: Color = Color::Rgb {
    r: 251,
    g: 191,
    b: 36,
};

/// Off-white — primary text, user messages.
pub const TEXT_PRIMARY: Color = Color::Rgb {
    r: 224,
    g: 224,
    b: 224,
};

/// Light grey — secondary text.
pub const TEXT_SECONDARY: Color = Color::Rgb {
    r: 170,
    g: 170,
    b: 170,
};

/// Dim grey — hints, timestamps, scroll indicators.
pub const TEXT_DIM: Color = Color::Rgb {
    r: 102,
    g: 102,
    b: 102,
};

/// Dark rule line color.
pub const RULE: Color = Color::Rgb {
    r: 64,
    g: 64,
    b: 64,
};
