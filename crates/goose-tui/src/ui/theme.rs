use ratatui::style::Color;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub name: String,
    pub base: BaseColors,
    pub status: StatusColors,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BaseColors {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,
    pub border: Color,
    pub border_active: Color,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StatusColors {
    pub info: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub thinking: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::gemini()
    }
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "goose" => Theme::goose(),
            "light" => Theme::light(),
            "dark" => Theme::dark(),
            "midnight" => Theme::midnight(),
            "matrix" => Theme::matrix(),
            _ => Theme::gemini(),
        }
    }

    pub fn gemini() -> Self {
        Self {
            name: "Gemini".to_string(),
            base: BaseColors {
                background: Color::Reset,             // User's terminal background
                foreground: Color::Rgb(205, 214, 244), // #CDD6F4
                cursor: Color::Rgb(137, 220, 235),     // #89DCEB (AccentCyan)
                selection: Color::Rgb(49, 50, 68),     // #313244
                border: Color::Rgb(108, 112, 134),     // #6C7086
                border_active: Color::Rgb(137, 180, 250), // #89B4FA
            },
            status: StatusColors {
                info: Color::Rgb(137, 180, 250),     // #89B4FA
                success: Color::Rgb(166, 227, 161),  // #A6E3A1
                warning: Color::Rgb(249, 226, 175),  // #F9E2AF
                error: Color::Rgb(243, 139, 168),    // #F38BA8
                thinking: Color::Rgb(203, 166, 247), // #CBA6F7
            },
        }
    }

    pub fn goose() -> Self {
        Self {
            name: "Goose".to_string(),
            base: BaseColors {
                background: Color::Rgb(31, 28, 26),    // #1F1C1A
                foreground: Color::Rgb(230, 225, 220), // #E6E1DC
                cursor: Color::Rgb(212, 163, 115),     // #D4A373
                selection: Color::Rgb(60, 56, 54),
                border: Color::Rgb(80, 70, 60),
                border_active: Color::Rgb(212, 163, 115), // #D4A373
            },
            status: StatusColors {
                info: Color::Blue,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                thinking: Color::Magenta,
            },
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            base: BaseColors {
                background: Color::White,           // #FFFFFF
                foreground: Color::Rgb(31, 41, 55), // #1F2937
                cursor: Color::Black,
                selection: Color::Rgb(219, 234, 254), // Light Blue
                border: Color::DarkGray,
                border_active: Color::Blue,
            },
            status: StatusColors {
                info: Color::Blue,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                thinking: Color::Magenta,
            },
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            base: BaseColors {
                background: Color::Rgb(30, 30, 30),    // #1E1E1E
                foreground: Color::Rgb(212, 212, 212), // #D4D4D4
                cursor: Color::White,
                selection: Color::Rgb(38, 79, 120),
                border: Color::Gray,
                border_active: Color::Blue,
            },
            status: StatusColors {
                info: Color::Blue,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                thinking: Color::Magenta,
            },
        }
    }

    pub fn midnight() -> Self {
        Self {
            name: "Midnight".to_string(),
            base: BaseColors {
                background: Color::Rgb(30, 30, 46),       // #1E1E2E
                foreground: Color::Rgb(205, 214, 244),    // #CDD6F4
                cursor: Color::Rgb(245, 224, 220),        // #F5E0DC (Rosewater)
                selection: Color::Rgb(49, 50, 68),        // #313244
                border: Color::Rgb(108, 112, 134),        // #6C7086
                border_active: Color::Rgb(137, 180, 250), // #89B4FA
            },
            status: StatusColors {
                info: Color::Rgb(137, 180, 250),     // #89B4FA
                success: Color::Rgb(166, 227, 161),  // #A6E3A1
                warning: Color::Rgb(249, 226, 175),  // #F9E2AF
                error: Color::Rgb(243, 139, 168),    // #F38BA8
                thinking: Color::Rgb(203, 166, 247), // #CBA6F7
            },
        }
    }

    pub fn matrix() -> Self {
        Self {
            name: "Matrix".to_string(),
            base: BaseColors {
                background: Color::Black,
                foreground: Color::Green,
                cursor: Color::Green,
                selection: Color::DarkGray,
                border: Color::DarkGray,
                border_active: Color::Green,
            },
            status: StatusColors {
                info: Color::Green,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                thinking: Color::Green,
            },
        }
    }
}
