use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

pub const GOOSE_LOGO: &str = r#"
██████\   ██████\   ██████\   ███████\  ██████
██  __██\ ██  __██\ ██  __██\ ██  _____|██  __██
██ /  ██ |██ /  ██ |██ /  ██ |\██████\  ████████ |
██ |  ██ |██ |  ██ |██ |  ██ | \____██\ ██   ____|
\███████ |\██████  |\██████  |███████  |\███████\
 \____██ | \______/  \______/ \_______/  \_______|
██\   ██ |
\██████  |
 \______/
"#;

fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (start, end) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8;
            let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8;
            let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8;
            Color::Rgb(r, g, b)
        }
        _ => start,
    }
}

pub fn render_logo_with_gradient(start_color: Color, end_color: Color) -> Vec<Line<'static>> {
    let lines: Vec<&str> = GOOSE_LOGO.lines().collect();
    let total_lines = lines.len() as f32;

    lines
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            let t = i as f32 / total_lines.max(1.0);
            let color = lerp_color(start_color, end_color, t);
            Line::from(Span::styled(line.to_string(), Style::default().fg(color)))
        })
        .collect()
}
