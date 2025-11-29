use ratatui::style::Color;

pub fn color_to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (128, 128, 128),
    }
}

pub fn breathing_color(base: Color, frame_count: usize, is_active: bool) -> Color {
    let (r, g, b) = color_to_rgb(base);
    if is_active {
        let t = frame_count as f32 * 0.1;
        let factor = 0.85 + 0.15 * t.sin();
        Color::Rgb(
            (r as f32 * factor) as u8,
            (g as f32 * factor) as u8,
            (b as f32 * factor) as u8,
        )
    } else {
        Color::Rgb(r, g, b)
    }
}

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
    pub user_message_foreground: Color,
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
    pub fn all_names() -> Vec<&'static str> {
        vec![
            "gemini",
            "goose",
            "light",
            "dark",
            "midnight",
            "nord",
            "dracula",
            "matrix",
            "tokyonight",
            "solarized",
            "retrowave",
        ]
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "goose" => Theme::goose(),
            "light" => Theme::light(),
            "dark" => Theme::dark(),
            "midnight" => Theme::midnight(),
            "nord" => Theme::nord(),
            "dracula" => Theme::dracula(),
            "matrix" => Theme::matrix(),
            "tokyonight" | "tokyo" => Theme::tokyonight(),
            "solarized" | "solar" => Theme::solarized(),
            "retrowave" | "retro" | "synthwave" => Theme::retrowave(),
            _ => Theme::gemini(),
        }
    }

    pub fn gemini() -> Self {
        Self {
            name: "Gemini".to_string(),
            base: BaseColors {
                background: Color::Reset,
                foreground: Color::Rgb(205, 214, 244),
                cursor: Color::Rgb(166, 173, 200),
                selection: Color::Rgb(49, 50, 68),
                border: Color::Rgb(108, 112, 134),
                border_active: Color::Rgb(137, 180, 250),
                user_message_foreground: Color::Rgb(165, 173, 192),
            },
            status: StatusColors {
                info: Color::Rgb(137, 180, 250),
                success: Color::Rgb(166, 227, 161),
                warning: Color::Rgb(249, 226, 175),
                error: Color::Rgb(243, 139, 168),
                thinking: Color::Rgb(203, 166, 247),
            },
        }
    }

    pub fn goose() -> Self {
        Self {
            name: "Goose".to_string(),
            base: BaseColors {
                background: Color::Rgb(31, 28, 26),
                foreground: Color::Rgb(230, 225, 220),
                cursor: Color::Rgb(212, 163, 115),
                selection: Color::Rgb(60, 56, 54),
                border: Color::Rgb(80, 70, 60),
                border_active: Color::Rgb(212, 163, 115),
                user_message_foreground: Color::Rgb(180, 175, 170),
            },
            status: StatusColors {
                info: Color::Rgb(138, 173, 189),
                success: Color::Rgb(169, 182, 101),
                warning: Color::Rgb(234, 178, 102),
                error: Color::Rgb(204, 102, 102),
                thinking: Color::Rgb(212, 163, 115),
            },
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".to_string(),
            base: BaseColors {
                background: Color::Rgb(255, 255, 255),
                foreground: Color::Rgb(36, 41, 46),
                cursor: Color::Rgb(3, 102, 214),
                selection: Color::Rgb(225, 228, 232),
                border: Color::Rgb(225, 228, 232),
                border_active: Color::Rgb(3, 102, 214),
                user_message_foreground: Color::Rgb(88, 96, 105),
            },
            status: StatusColors {
                info: Color::Rgb(3, 102, 214),
                success: Color::Rgb(34, 134, 58),
                warning: Color::Rgb(227, 98, 9),
                error: Color::Rgb(215, 58, 73),
                thinking: Color::Rgb(111, 66, 193),
            },
        }
    }

    pub fn dark() -> Self {
        Self {
            name: "Dark".to_string(),
            base: BaseColors {
                background: Color::Rgb(40, 44, 52),
                foreground: Color::Rgb(171, 178, 191),
                cursor: Color::Rgb(97, 175, 239),
                selection: Color::Rgb(62, 68, 81),
                border: Color::Rgb(62, 68, 81),
                border_active: Color::Rgb(97, 175, 239),
                user_message_foreground: Color::Rgb(130, 137, 151),
            },
            status: StatusColors {
                info: Color::Rgb(97, 175, 239),
                success: Color::Rgb(152, 195, 121),
                warning: Color::Rgb(229, 192, 123),
                error: Color::Rgb(224, 108, 117),
                thinking: Color::Rgb(198, 120, 221),
            },
        }
    }

    pub fn midnight() -> Self {
        Self {
            name: "Midnight".to_string(),
            base: BaseColors {
                background: Color::Rgb(30, 30, 46),
                foreground: Color::Rgb(205, 214, 244),
                cursor: Color::Rgb(245, 224, 220),
                selection: Color::Rgb(49, 50, 68),
                border: Color::Rgb(108, 112, 134),
                border_active: Color::Rgb(137, 180, 250),
                user_message_foreground: Color::Rgb(165, 173, 192),
            },
            status: StatusColors {
                info: Color::Rgb(137, 180, 250),
                success: Color::Rgb(166, 227, 161),
                warning: Color::Rgb(249, 226, 175),
                error: Color::Rgb(243, 139, 168),
                thinking: Color::Rgb(203, 166, 247),
            },
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "Nord".to_string(),
            base: BaseColors {
                background: Color::Rgb(46, 52, 64),
                foreground: Color::Rgb(216, 222, 233),
                cursor: Color::Rgb(136, 192, 208),
                selection: Color::Rgb(67, 76, 94),
                border: Color::Rgb(67, 76, 94),
                border_active: Color::Rgb(136, 192, 208),
                user_message_foreground: Color::Rgb(176, 182, 196),
            },
            status: StatusColors {
                info: Color::Rgb(129, 161, 193),
                success: Color::Rgb(163, 190, 140),
                warning: Color::Rgb(235, 203, 139),
                error: Color::Rgb(191, 97, 106),
                thinking: Color::Rgb(180, 142, 173),
            },
        }
    }

    pub fn dracula() -> Self {
        Self {
            name: "Dracula".to_string(),
            base: BaseColors {
                background: Color::Rgb(40, 42, 54),
                foreground: Color::Rgb(248, 248, 242),
                cursor: Color::Rgb(255, 121, 198),
                selection: Color::Rgb(68, 71, 90),
                border: Color::Rgb(68, 71, 90),
                border_active: Color::Rgb(189, 147, 249),
                user_message_foreground: Color::Rgb(189, 189, 189),
            },
            status: StatusColors {
                info: Color::Rgb(139, 233, 253),
                success: Color::Rgb(80, 250, 123),
                warning: Color::Rgb(255, 184, 108),
                error: Color::Rgb(255, 85, 85),
                thinking: Color::Rgb(189, 147, 249),
            },
        }
    }

    pub fn matrix() -> Self {
        Self {
            name: "Matrix".to_string(),
            base: BaseColors {
                background: Color::Rgb(0, 10, 0),
                foreground: Color::Rgb(0, 255, 65),
                cursor: Color::Rgb(0, 255, 65),
                selection: Color::Rgb(0, 60, 0),
                border: Color::Rgb(0, 80, 0),
                border_active: Color::Rgb(0, 255, 65),
                user_message_foreground: Color::Rgb(0, 180, 45),
            },
            status: StatusColors {
                info: Color::Rgb(0, 200, 80),
                success: Color::Rgb(0, 255, 65),
                warning: Color::Rgb(180, 255, 0),
                error: Color::Rgb(255, 50, 50),
                thinking: Color::Rgb(0, 255, 130),
            },
        }
    }

    pub fn tokyonight() -> Self {
        Self {
            name: "TokyoNight".to_string(),
            base: BaseColors {
                background: Color::Rgb(26, 27, 38),
                foreground: Color::Rgb(192, 202, 245),
                cursor: Color::Rgb(125, 207, 255),
                selection: Color::Rgb(43, 48, 74),
                border: Color::Rgb(59, 66, 97),
                border_active: Color::Rgb(125, 207, 255),
                user_message_foreground: Color::Rgb(148, 156, 187),
            },
            status: StatusColors {
                info: Color::Rgb(125, 207, 255),
                success: Color::Rgb(158, 206, 106),
                warning: Color::Rgb(224, 175, 104),
                error: Color::Rgb(247, 118, 142),
                thinking: Color::Rgb(187, 154, 247),
            },
        }
    }

    pub fn solarized() -> Self {
        Self {
            name: "Solarized".to_string(),
            base: BaseColors {
                background: Color::Rgb(0, 43, 54),
                foreground: Color::Rgb(131, 148, 150),
                cursor: Color::Rgb(38, 139, 210),
                selection: Color::Rgb(7, 54, 66),
                border: Color::Rgb(88, 110, 117),
                border_active: Color::Rgb(38, 139, 210),
                user_message_foreground: Color::Rgb(101, 123, 131),
            },
            status: StatusColors {
                info: Color::Rgb(38, 139, 210),
                success: Color::Rgb(133, 153, 0),
                warning: Color::Rgb(181, 137, 0),
                error: Color::Rgb(220, 50, 47),
                thinking: Color::Rgb(108, 113, 196),
            },
        }
    }

    pub fn retrowave() -> Self {
        Self {
            name: "Retrowave".to_string(),
            base: BaseColors {
                background: Color::Rgb(22, 22, 30),
                foreground: Color::Rgb(224, 222, 244),
                cursor: Color::Rgb(255, 121, 198),
                selection: Color::Rgb(52, 42, 72),
                border: Color::Rgb(65, 55, 85),
                border_active: Color::Rgb(255, 121, 198),
                user_message_foreground: Color::Rgb(180, 175, 200),
            },
            status: StatusColors {
                info: Color::Rgb(125, 207, 255),
                success: Color::Rgb(115, 218, 202),
                warning: Color::Rgb(255, 158, 100),
                error: Color::Rgb(247, 118, 142),
                thinking: Color::Rgb(187, 154, 247),
            },
        }
    }
}
