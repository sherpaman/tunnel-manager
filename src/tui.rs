use crate::manager::TunnelManager;
use crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use std::io;

pub struct TunnelInfo {
    pub name: String,
    pub active: bool,
    pub remote_host: String,
    pub remote_port: u16,
    pub local_port: u16,
}

pub fn run_tui(mut tunnels: Vec<TunnelInfo>) -> io::Result<()> {
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use ratatui::Terminal;
    use ratatui::backend::CrosstermBackend;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut selected = 0;
    let mut quit = false;
    let mut status_msg = String::new();
    let manager = TunnelManager::new();
    let mut scroll_offset = 0;
    while !quit {
        terminal.draw(|f| {
            let size = f.size();
            // Add a top row for key bindings
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)])
                .split(size);
            let keybinds = Paragraph::new("o: open   c: close   q: quit").block(Block::default());
            f.render_widget(keybinds, main_chunks[0]);

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(main_chunks[1]);
            // Left: tunnel list (scrollable)
            let tunnel_items: Vec<ListItem> = tunnels
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let style = if t.active {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let marker = if t.active { "●" } else { "○" };
                    let mut text = format!("{} {}", marker, t.name);
                    if i == selected {
                        text = format!("> {}", text);
                    }
                    ListItem::new(text).style(style)
                })
                .collect();
            // Calculate visible window for scrolling
            let list_height = chunks[0].height.saturating_sub(2) as usize; // minus borders
            let total = tunnel_items.len();
            if selected < scroll_offset {
                scroll_offset = selected;
            } else if selected >= scroll_offset + list_height {
                scroll_offset = selected + 1 - list_height;
            }
            let end = (scroll_offset + list_height).min(total);
            let visible_items = tunnel_items[scroll_offset..end].to_vec();
            let tunnels_list = List::new(visible_items)
                .block(Block::default().borders(Borders::ALL).title("Tunnels"));
            f.render_widget(tunnels_list, chunks[0]);
            // Right: vertical split
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[1]);
            // Top right: tunnel details
            let t = &tunnels[selected];
            let details = format!(
                "Status: {}\nRemote: {}:{}\nLocal: {}",
                if t.active { "ACTIVE" } else { "INACTIVE" },
                t.remote_host,
                t.remote_port,
                t.local_port
            );
            let details_widget = Paragraph::new(details)
                .block(Block::default().borders(Borders::ALL).title("Details"));
            f.render_widget(details_widget, right_chunks[0]);
            // Bottom right: status/placeholder
            let placeholder = Paragraph::new(if status_msg.is_empty() {
                "[Future features here]"
            } else {
                &status_msg
            })
            .block(Block::default().borders(Borders::ALL).title("Placeholder"));
            f.render_widget(placeholder, right_chunks[1]);
        })?;
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => quit = true,
                    KeyCode::Down => {
                        if selected + 1 < tunnels.len() {
                            selected += 1;
                        }
                    }
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                    }
                    KeyCode::Char('o') => {
                        let name = tunnels[selected].name.clone();
                        let was_active = tunnels[selected].active;
                        if was_active {
                            status_msg = format!("Tunnel '{}' already active", name);
                        } else {
                            match manager.open_tunnel(&name) {
                                Ok(_) => {
                                    tunnels[selected].active = true;
                                    status_msg = format!("Opened tunnel '{}'", name);
                                }
                                Err(e) => status_msg = format!("Failed to open '{}': {}", name, e),
                            }
                        }
                    }
                    KeyCode::Char('c') => {
                        let name = tunnels[selected].name.clone();
                        let was_active = tunnels[selected].active;
                        if !was_active {
                            status_msg = format!("Tunnel '{}' not active", name);
                        } else {
                            match manager.close_tunnel(&name) {
                                Ok(_) => {
                                    tunnels[selected].active = false;
                                    status_msg = format!("Closed tunnel '{}'", name);
                                }
                                Err(e) => status_msg = format!("Failed to close '{}': {}", name, e),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    disable_raw_mode()?;
    terminal.clear()?;
    Ok(())
}
