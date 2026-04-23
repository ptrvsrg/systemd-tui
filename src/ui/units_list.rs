use crate::app::App;
use ratatui::layout::Constraint;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};

pub fn units_table(app: &App, state: &mut TableState, focused: bool) -> Table<'static> {
    let rows: Vec<Row<'static>> = app
        .visible_units()
        .map(|unit| {
            Row::new(vec![
                Cell::from(unit.name.clone()),
                Cell::from(unit.active_state.as_str().to_string()),
                Cell::from(unit.sub_state.clone()),
                Cell::from(format!("{:?}", unit.load_state).to_lowercase()),
                Cell::from(unit.description.clone()),
                Cell::from(unit.active_glyph().to_string()),
            ])
            .style(Style::default().fg(unit.status_color(&app.config.colors)))
        })
        .collect();

    state.select(app.selected_index());

    Table::new(
        rows,
        [
            Constraint::Length(32),
            Constraint::Length(12),
            Constraint::Length(14),
            Constraint::Length(12),
            Constraint::Min(10),
            Constraint::Length(2),
        ],
    )
    .header(
        Row::new(vec!["Unit", "Active", "Sub", "Load", "Description", "S"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .block({
        let title = if focused {
            format!("Units * ({})", app.visible_count())
        } else {
            format!("Units ({})", app.visible_count())
        };
        let block = Block::default().title(title).borders(Borders::ALL);
        if focused {
            block.border_style(Style::default().fg(ratatui::style::Color::Cyan))
        } else {
            block
        }
    })
    .highlight_style(
        Style::default()
            .add_modifier(Modifier::REVERSED)
            .add_modifier(Modifier::BOLD),
    )
}
