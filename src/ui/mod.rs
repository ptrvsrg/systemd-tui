use crate::app::{App, FocusBlock};
use crate::ui::units_list::units_table;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, TableState, Wrap};

pub mod units_list;

pub fn draw(frame: &mut Frame<'_>, app: &App, table_state: &mut TableState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(5),
        ])
        .split(frame.area());

    let title = Paragraph::new(format!(
        " systemd-tui  conn:{}  units:{}  state:{}  filter:{}{} ",
        app.connection_label(),
        app.units.len(),
        app.state_filter_label(),
        if app.name_filter().is_empty() {
            "-"
        } else {
            app.name_filter()
        },
        if app.filter_input_mode() {
            " [input]"
        } else {
            ""
        },
    ))
    .style(
        Style::default()
            .fg(ratatui::style::Color::Black)
            .bg(app.config.colors.header_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, chunks[0]);

    let table = units_table(app, table_state, app.focus_block() == FocusBlock::Units);
    frame.render_stateful_widget(table, chunks[1], table_state);

    let detail_lines = app.details_lines();
    let detail_height = chunks[2].height.saturating_sub(2) as usize;
    let detail_max_scroll = detail_lines.len().saturating_sub(detail_height) as u16;
    let detail_scroll = app.details_scroll().min(detail_max_scroll);
    let detail_title = if app.focus_block() == FocusBlock::Details {
        format!("Details * {detail_scroll}/{detail_max_scroll}")
    } else {
        format!("Details {detail_scroll}/{detail_max_scroll}")
    };
    let detail_widget = Paragraph::new(detail_lines.join("\n"))
        .block(styled_block(
            detail_title,
            app.focus_block() == FocusBlock::Details,
        ))
        .scroll((detail_scroll, 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(detail_widget, chunks[2]);

    let status_lines = app.status_lines();
    let status_height = chunks[3].height.saturating_sub(2) as usize;
    let status_max_scroll = status_lines.len().saturating_sub(status_height) as u16;
    let status_scroll = app.status_scroll().min(status_max_scroll);
    let status_title = if app.focus_block() == FocusBlock::Status {
        format!("Status * {status_scroll}/{status_max_scroll}")
    } else {
        format!("Status {status_scroll}/{status_max_scroll}")
    };
    let footer = Paragraph::new(status_lines.join("\n"))
        .block(styled_block(
            status_title,
            app.focus_block() == FocusBlock::Status,
        ))
        .scroll((status_scroll, 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(footer, chunks[3]);

    if app.show_help() {
        draw_help_popup(frame, app);
    }
}

fn draw_help_popup(frame: &mut Frame<'_>, app: &App) {
    let area = centered_rect(72, 70, frame.area());
    frame.render_widget(Clear, area);

    let lines = [
        "systemd-tui keymap",
        "",
        "Navigation:",
        "  Tab / Shift+Tab Focus next/prev block",
        "  ↑/k, ↓/j      Move selection",
        "  g / G          First / last unit",
        "",
        "Unit actions (selected unit):",
        "  s              Start",
        "  t              Stop",
        "  R              Restart",
        "  L              Reload",
        "  r              Refresh now",
        "",
        "Filtering:",
        "  /              Name filter input mode",
        "  F2             Cycle state filter (all/active/inactive/failed)",
        "",
        "General:",
        "  h / F1         Toggle this help",
        "  q              Quit (when help closed)",
        "",
        "Scroll (focused block or help):",
        "  ↑/↓ or j/k     Scroll 1 line",
        "  PgUp/PgDn      Scroll 8 lines",
        "  g / G          Top / bottom",
        "",
        "Close help: Esc, h, F1, q",
    ];
    let text = lines.join("\n");
    let visible_height = area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(visible_height) as u16;
    let scroll = app.help_scroll().min(max_scroll);

    let popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!("Help  scroll:{scroll}/{max_scroll}"))
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(popup, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn styled_block(title: String, focused: bool) -> Block<'static> {
    let block = Block::default().title(title).borders(Borders::ALL);
    if focused {
        block.border_style(Style::default().fg(ratatui::style::Color::Cyan))
    } else {
        block
    }
}
