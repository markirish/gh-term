use std::{
    sync::mpsc,
    time::Duration,
};

use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

mod auth;
use auth::{AuthBootstrap, GhCliAuthBootstrap};

pub struct App {
    auth: AuthScreenState,
    should_quit: bool,
}

pub enum AuthScreenState {
    Checking,
    Success {
        username: String,
        host: String,
        token_preview: String,
    },
    Error {
        message: String,
    },
}

enum AuthMessage {
    Success {
        username: String,
        host: String,
        token_preview: String,
    },
    Error(String),
}

fn main() -> Result<(), std::io::Error> {
    let mut terminal = init_terminal()?;

    let (tx, rx) = mpsc::channel::<AuthMessage>();

    std::thread::spawn(move || {
        let auth = GhCliAuthBootstrap::new();

        match auth.get_session() {
            Ok(session) => {
                let preview = preview_token(&session.token);
                let _ = tx.send(AuthMessage::Success {
                    username: session.username,
                    host: session.host,
                    token_preview: preview,
                });
            }
            Err(err) => {
                let _ = tx.send(AuthMessage::Error(err.to_string()));
            }
        }
    });

    let mut app = App {
        auth: AuthScreenState::Checking,
        should_quit: false,
    };

    while !app.should_quit {
        while let Ok(msg) = rx.try_recv() {
            match msg {
                AuthMessage::Success {
                    username,
                    host,
                    token_preview,
                } => {
                    app.auth = AuthScreenState::Success {
                        username,
                        host,
                        token_preview,
                    };
                }
                AuthMessage::Error(message) => {
                    app.auth = AuthScreenState::Error { message };
                }
            }
        }

        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    _ => {}
                }
            }
        }
    }

    restore_terminal()?;
    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    let area = centered_rect(frame.area(), 60, 12);

    let (title, body, border_color) = match &app.auth {
        AuthScreenState::Checking => (
            "GitHub Auth",
            Text::from(vec![
                Line::from(""),
                Line::from("Getting `gh` CLI token").alignment(Alignment::Center),
                Line::from(""),
                Line::from("Please wait...").alignment(Alignment::Center),
            ]),
            Color::Yellow,
        ),
        AuthScreenState::Success {
            username,
            host,
            token_preview,
        } => (
            "GitHub Auth",
            Text::from(vec![
                Line::from(""),
                Line::from("Token found!").alignment(Alignment::Center),
                Line::from(""),
                Line::from(format!("User: {username}")).alignment(Alignment::Center),
                Line::from(format!("Host: {host}")).alignment(Alignment::Center),
                Line::from(format!("Token: {token_preview}")).alignment(Alignment::Center),
                Line::from(""),
                Line::from("Press q to quit").alignment(Alignment::Center),
            ]),
            Color::Green,
        ),
        AuthScreenState::Error { message } => (
            "GitHub Auth",
            Text::from(vec![
                Line::from(""),
                Line::from("Authentication failed").alignment(Alignment::Center),
                Line::from(""),
                Line::from(message.as_str()).alignment(Alignment::Center),
                Line::from(""),
                Line::from("Press q to quit").alignment(Alignment::Center),
            ]),
            Color::Red,
        ),
    };

    let block = Block::default()
        .title(Line::from(title).alignment(Alignment::Center))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let paragraph = Paragraph::new(body)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn centered_rect(full: Rect, width: u16, height: u16) -> Rect {
    let x = full.x + full.width.saturating_sub(width) / 2;
    let y = full.y + full.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(full.width), height.min(full.height))
}

fn preview_token(token: &str) -> String {
    if token.len() <= 8 {
        return "********".to_string();
    }

    format!("{}...{}", &token[..4], &token[token.len() - 4..])
}

fn init_terminal() -> Result<DefaultTerminal, std::io::Error> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let terminal = ratatui::init();
    Ok(terminal)
}

fn restore_terminal() -> Result<(), std::io::Error> {
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    ratatui::restore();
    Ok(())
}