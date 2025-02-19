use std::io::{self, IsTerminal};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};

#[derive(Copy, Clone)]
enum Mode {
    Tail,   // Auto-scrolling mode
    Normal, // Manual scrolling and searching
}

impl Mode {
    fn status_text(&self) -> String {
        match self {
            Mode::Tail => "TAIL".to_string(),
            Mode::Normal => "NORMAL".to_string(),
        }
    }
}

struct App {
    lines: Arc<Mutex<Vec<String>>>,
    scroll: usize,
    mode: Mode,
    search_query: String,
    current_match: usize,
    matches: Vec<(usize, usize, usize)>, // (line_index, start, end)
}

impl App {
    fn new() -> Self {
        Self {
            lines: Arc::new(Mutex::new(Vec::new())),
            scroll: 0,
            mode: Mode::Tail,
            search_query: String::new(),
            current_match: 0,
            matches: Vec::new(),
        }
    }

    fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    fn scroll_down(&mut self, amount: usize, max_scroll: usize) {
        self.scroll = (self.scroll + amount).min(max_scroll);
    }

    fn len(&self) -> usize {
        self.lines.lock().unwrap().len()
    }

    fn update_search(&mut self) {
        if self.search_query.is_empty() {
            self.matches.clear();
            return;
        }

        if let Ok(lines) = self.lines.lock() {
            self.matches.clear();
            for (line_idx, line) in lines.iter().enumerate() {
                for (match_idx, _) in line.match_indices(&self.search_query) {
                    self.matches.push((line_idx, match_idx, match_idx + self.search_query.len()));
                }
            }
        }
    }

    fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
            if let Some((line_idx, _, _)) = self.matches.get(self.current_match) {
                self.scroll = *line_idx;
                self.mode = Mode::Normal;
            }
        }
    }

    fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = self.current_match.checked_sub(1).unwrap_or(self.matches.len() - 1);
            if let Some((line_idx, _, _)) = self.matches.get(self.current_match) {
                self.scroll = *line_idx;
                self.mode = Mode::Normal;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Exit if stdin is not a pipe
    if io::stdin().is_terminal() {
        return Ok(());
    }

    let mut app = App::new();
    let lines = app.lines.clone();
    
    // Spawn an async task to read from stdin continuously
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines_stream = reader.lines();
        
        while let Ok(Some(line)) = lines_stream.next_line().await {
            if let Ok(mut lines_vec) = lines.lock() {
                lines_vec.push(line);
            }
        }
    });

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;


    loop {
        terminal.draw(|frame| {
            let size = frame.size();
            // Create a temporary vector of lines while holding the lock
            let items: Vec<ListItem> = app.lines
                .lock()
                .map(|lines| {
                    lines.iter().enumerate().map(|(idx, line)| {
                        let mut spans = Vec::new();
                        let mut last_end = 0;

                        // Get all matches for this line
                        let line_matches: Vec<_> = app.matches.iter()
                            .enumerate()
                            .filter(|(_, (line_idx, _, _))| *line_idx == idx)
                            .collect();

                        for (match_idx, (_, start, end)) in line_matches {
                            // Add non-matching text before this match
                            if last_end < *start {
                                spans.push(ratatui::text::Span::raw(
                                    line[last_end..*start].to_string()
                                ));
                            }

                            // Add the matching text with highlight
                            let style = if match_idx == app.current_match {
                                Style::default().bg(ratatui::style::Color::Yellow)
                                    .fg(ratatui::style::Color::Black)
                            } else {
                                Style::default().bg(ratatui::style::Color::DarkGray)
                                    .fg(ratatui::style::Color::White)
                            };

                            spans.push(ratatui::text::Span::styled(
                                line[*start..*end].to_string(),
                                style,
                            ));
                            last_end = *end;
                        }

                        // Add remaining text after last match
                        if last_end < line.len() {
                            spans.push(ratatui::text::Span::raw(
                                line[last_end..].to_string()
                            ));
                        }

                        // If no matches were found, just show the plain line
                        if spans.is_empty() {
                            spans.push(ratatui::text::Span::raw(line.to_string()));
                        }

                        ListItem::new(ratatui::text::Line::from(spans))
                    }).collect()
                })
                .unwrap_or_default();

            let view_height = size.height as usize;
            let total_lines = items.len();
            let max_scroll = total_lines.saturating_sub(view_height);

            // Auto-scroll to bottom if in tail mode and content doesn't fit
            if matches!(app.mode, Mode::Tail) && total_lines > view_height {
                app.scroll = max_scroll;
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Text Viewer"))
                .style(Style::default())
                .highlight_style(Style::default().bold());

            // Create a layout with main content and status bar
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),     // Main content
                    Constraint::Length(1),  // Status bar
                ].as_ref())
                .split(size);

            // Render main content
            frame.render_stateful_widget(
                list,
                chunks[0],
                &mut ratatui::widgets::ListState::default().with_offset(app.scroll),
            );

            // Render status bar
            let mode_text = format!(" {} ", app.mode.status_text());
            let mode_style = match app.mode {
                Mode::Tail => Style::default().bg(Color::Green).fg(Color::Black),
                Mode::Normal => Style::default().bg(Color::Blue).fg(Color::White),
            };
            
            let status = Line::from(vec![
                ratatui::text::Span::styled(mode_text, mode_style),
                if !app.search_query.is_empty() {
                    ratatui::text::Span::raw(format!(" Search: {} ", app.search_query))
                } else {
                    ratatui::text::Span::raw("")
                },
            ]);

            frame.render_widget(
                Paragraph::new(status)
                    .style(Style::default().bg(Color::DarkGray)),
                chunks[1]
            );
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (app.mode, key.code) {
                    // Quit works in any mode
                    (_, KeyCode::Char('q')) => break,
                    
                    // Esc always returns to tail mode
                    (_, KeyCode::Esc) => app.mode = Mode::Tail,
                    
                    // In tail mode, any movement key enters normal mode
                    (Mode::Tail, KeyCode::Char('/')) => {
                        app.mode = Mode::Normal;
                        app.search_query.clear();
                    },
                    (Mode::Tail, KeyCode::Char('j')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            app.mode = Mode::Normal;
                            app.scroll_down(1, app.len().saturating_sub(view_height));
                        }
                    },
                    (Mode::Tail, KeyCode::Char('k')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            app.mode = Mode::Normal;
                            app.scroll_up(1);
                        }
                    },
                    (Mode::Tail, KeyCode::Char('d')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            app.mode = Mode::Normal;
                            let amount = view_height / 2;
                            app.scroll_down(amount, app.len().saturating_sub(view_height));
                        }
                    },
                    (Mode::Tail, KeyCode::Char('u')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            app.mode = Mode::Normal;
                            let amount = view_height / 2;
                            app.scroll_up(amount);
                        }
                    },
                    
                    // Normal mode commands
                    (Mode::Normal, KeyCode::Char('/')) => {
                        app.search_query.clear();
                    },
                    (Mode::Normal, KeyCode::Char('n')) if !app.matches.is_empty() => app.next_match(),
                    (Mode::Normal, KeyCode::Char('N')) if !app.matches.is_empty() => app.prev_match(),
                    (Mode::Normal, KeyCode::Char('j')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            app.scroll_down(1, app.len().saturating_sub(view_height));
                        }
                    },
                    (Mode::Normal, KeyCode::Char('k')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            app.scroll_up(1);
                        }
                    },
                    (Mode::Normal, KeyCode::Char('d')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            let amount = view_height / 2;
                            app.scroll_down(amount, app.len().saturating_sub(view_height));
                        }
                    },
                    (Mode::Normal, KeyCode::Char('u')) => {
                        let view_height = terminal.size()?.height as usize;
                        if app.len() > view_height {
                            let amount = view_height / 2;
                            app.scroll_up(amount);
                        }
                    },
                    // Handle all characters in normal mode (for search)
                    (Mode::Normal, KeyCode::Char(c)) => {
                        app.search_query.push(c);
                        app.update_search();
                    },
                    (Mode::Normal, KeyCode::Backspace) => {
                        app.search_query.pop();
                        app.update_search();
                    },
                    (Mode::Normal, KeyCode::Enter) => {
                        if !app.matches.is_empty() {
                            if let Some((line_idx, _, _)) = app.matches.get(app.current_match) {
                                app.scroll = *line_idx;
                            }
                        }
                        app.search_query.clear();
                    },
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // Print the text after exiting
    if let Ok(lines) = app.lines.lock() {
        for line in lines.iter() {
            println!("{}", line);
        }
    }

    Ok(())
}
