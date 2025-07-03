use crate::session::{output, Session};

impl Session {
    pub fn handle_theme_toggle(&self) {
        let current = output::get_theme();
        let new_theme = match current {
            output::Theme::Light => {
                println!("Switching to Dark theme");
                output::Theme::Dark
            }
            output::Theme::Dark => {
                println!("Switching to Ansi theme");
                output::Theme::Ansi
            }
            output::Theme::Ansi => {
                println!("Switching to Light theme");
                output::Theme::Light
            }
        };
        output::set_theme(new_theme);
    }
}
