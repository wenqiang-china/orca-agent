use crate::app::{App, ChatMessage, InputMode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

/// Render the entire UI
pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(1),    // Chat area
            Constraint::Length(3), // Input area
            Constraint::Length(1), // Help bar
        ])
        .split(f.area());

    render_status_bar(f, app, chunks[0]);
    render_chat(f, app, chunks[1]);
    render_input(f, app, chunks[2]);
    render_help_bar(f, app, chunks[3]);
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status_style = Style::default().fg(Color::White).bg(Color::DarkGray);

    let left = format!(" Orca | {} ", app.model_name);
    let right = format!(
        " ${:.4} | {} iter | {} ",
        app.cost_usd, app.iterations, app.status
    );

    let padding = " ".repeat((area.width as usize).saturating_sub(left.len() + right.len()));

    let line = Line::from(vec![
        Span::styled(left, status_style),
        Span::styled(padding, status_style),
        Span::styled(right, status_style),
    ]);

    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

fn render_chat(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Chat ");

    let inner = block.inner(area);
    f.render_widget(block, area);

    let messages = app.visible_messages();
    let mut lines: Vec<Line> = Vec::new();

    for msg in &messages {
        match msg {
            ChatMessage::User(text) => {
                lines.push(Line::from(vec![Span::styled(
                    "You: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]));
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
            }
            ChatMessage::Assistant(text) => {
                lines.push(Line::from(vec![Span::styled(
                    "Orca: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::White),
                    )));
                }
                lines.push(Line::from(""));
            }
            ChatMessage::Tool(name, args) => {
                lines.push(Line::from(vec![
                    Span::styled("  [tool] ", Style::default().fg(Color::Yellow)),
                    Span::styled(name.clone(), Style::default().fg(Color::Green)),
                    Span::styled(
                        format!("({})", truncate(args, 60)),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            ChatMessage::ToolResult(content, is_error) => {
                let color = if *is_error {
                    Color::Red
                } else {
                    Color::DarkGray
                };
                let prefix = if *is_error {
                    "  [error] "
                } else {
                    "  [result] "
                };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, truncate(content, 200)),
                    Style::default().fg(color),
                )));
            }
            ChatMessage::System(text) => {
                lines.push(Line::from(Span::styled(
                    format!("  [system] {}", text),
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )));
            }
            ChatMessage::Error(text) => {
                lines.push(Line::from(Span::styled(
                    format!("  [error] {}", text),
                    Style::default().fg(Color::Red),
                )));
            }
        }
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });

    f.render_widget(paragraph, inner);

    // Render scrollbar
    if !app.messages.is_empty() {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        let mut scrollbar_state = ScrollbarState::new(app.messages.len())
            .position(app.messages.len().saturating_sub(app.scroll_offset));
        f.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.input_mode == InputMode::Editing {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Input ");

    let input_text = if app.input.is_empty() && app.input_mode == InputMode::Editing {
        Text::from(Span::styled(
            "Type a message...",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        Text::from(Span::styled(&app.input, Style::default().fg(Color::White)))
    };

    let paragraph = Paragraph::new(input_text).block(block);
    f.render_widget(paragraph, area);

    // Show cursor
    if app.input_mode == InputMode::Editing {
        let cursor_x = area.x + 1 + app.cursor_position as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let help = match app.input_mode {
        InputMode::Editing => {
            if app.is_processing {
                " Processing... (Ctrl+C to cancel) "
            } else {
                " Enter: send | Ctrl+C: quit | ↑/↓: scroll "
            }
        }
        InputMode::Normal => " i: edit | ↑/↓: scroll | q: quit | Ctrl+C: quit ",
    };

    let line = Line::from(Span::styled(help, Style::default().fg(Color::DarkGray)));
    let paragraph = Paragraph::new(line);
    f.render_widget(paragraph, area);
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
