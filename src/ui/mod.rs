use crate::app::{App, FocusBlock, LayoutMode};
use crate::ui::units_list::units_table;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, TableState, Wrap};

pub mod units_list;

pub fn draw(frame: &mut Frame<'_>, app: &mut App, table_state: &mut TableState) {
    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(5)])
        .split(frame.area());

    let title = Paragraph::new(format!(
        " systemd-tui  conn:{}  units:{}  state:{}  filter:{}{}  focus:{}  layout:{} \n updated:{}  status:{} ",
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
        app.focus_block().label(),
        app.layout_mode().label(),
        app.last_updated_at(),
        app.status(),
    ))
    .style(
        Style::default()
            .fg(ratatui::style::Color::Black)
            .bg(app.config.colors.header_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(title, root_chunks[0]);

    let content_chunks = match app.layout_mode() {
        LayoutMode::Horizontal => Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
            .split(root_chunks[1]),
        LayoutMode::Vertical => Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(root_chunks[1]),
    };

    let table = units_table(app, table_state, app.focus_block() == FocusBlock::Units);
    frame.render_stateful_widget(table, content_chunks[0], table_state);

    draw_right_panel(frame, app, content_chunks[1]);

    if app.show_help() {
        draw_help_popup(frame, app);
    }
}

fn draw_right_panel(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Min(8)])
        .split(area);

    let detail_lines = app.details_lines();
    let detail_height = panel_chunks[0].height.saturating_sub(2) as usize;
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
    frame.render_widget(detail_widget, panel_chunks[0]);

    let logs_height = panel_chunks[1].height.saturating_sub(2) as usize;
    let logs_max_scroll = app.logs_lines().len().saturating_sub(logs_height) as u16;
    app.update_logs_max_scroll_hint(logs_max_scroll);
    let logs_scroll = app.effective_logs_scroll(logs_max_scroll);
    let logs_title = if app.focus_block() == FocusBlock::Logs {
        format!(
            "Logs * {logs_scroll}/{logs_max_scroll}{}",
            if app.logs_follow() { " [follow]" } else { "" }
        )
    } else {
        format!(
            "Logs {logs_scroll}/{logs_max_scroll}{}",
            if app.logs_follow() { " [follow]" } else { "" }
        )
    };
    let logs_widget = Paragraph::new(app.logs_lines().join("\n"))
        .block(styled_block(
            logs_title,
            app.focus_block() == FocusBlock::Logs,
        ))
        .scroll((logs_scroll, 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(logs_widget, panel_chunks[1]);
}

fn draw_help_popup(frame: &mut Frame<'_>, app: &App) {
    let area = centered_rect(72, 70, frame.area());
    frame.render_widget(Clear, area);

    let lines = [
        "systemd-tui keymap",
        "",
        "Navigation:",
        "  Tab / Shift+Tab Focus next/prev block",
        "  ↑/k, ↓/j      Scroll focused block",
        "  g / G          Top / bottom of focused block",
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
        "  F3             Toggle horizontal / vertical layout",
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
