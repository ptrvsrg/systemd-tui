use crate::actions::manager::{UnitAction, execute};
use crate::config::Config;
use crate::tui::TuiTerminal;
use crate::ui;
use crate::units::{ManagerSignal, SystemdManager, SystemdUnit};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::widgets::TableState;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub struct App {
    manager: SystemdManager,
    signal_rx: mpsc::UnboundedReceiver<ManagerSignal>,
    pub config: Config,
    pub units: Vec<SystemdUnit>,
    filtered_indices: Vec<usize>,
    selected: usize,
    name_filter: String,
    filter_input_mode: bool,
    state_filter: StateFilter,
    last_refresh: Instant,
    status: String,
    show_help: bool,
    help_scroll: u16,
    focus_block: FocusBlock,
    details_scroll: u16,
    status_scroll: u16,
    should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateFilter {
    All,
    Active,
    Inactive,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusBlock {
    Units,
    Details,
    Status,
}

impl FocusBlock {
    fn next(self) -> Self {
        match self {
            Self::Units => Self::Details,
            Self::Details => Self::Status,
            Self::Status => Self::Units,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Units => Self::Status,
            Self::Details => Self::Units,
            Self::Status => Self::Details,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Units => "units",
            Self::Details => "details",
            Self::Status => "status",
        }
    }
}

impl StateFilter {
    fn cycle(self) -> Self {
        match self {
            Self::All => Self::Active,
            Self::Active => Self::Inactive,
            Self::Inactive => Self::Failed,
            Self::Failed => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Failed => "failed",
        }
    }
}

impl App {
    pub async fn new(config: Config) -> Result<Self> {
        let manager = SystemdManager::connect_with_config(&config.connection).await?;
        let signal_rx = manager.subscribe_unit_signals().await?;
        let mut app = Self {
            manager,
            signal_rx,
            config,
            units: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            name_filter: String::new(),
            filter_input_mode: false,
            state_filter: StateFilter::All,
            last_refresh: Instant::now() - Duration::from_secs(10),
            status: "Ready".to_string(),
            show_help: false,
            help_scroll: 0,
            focus_block: FocusBlock::Units,
            details_scroll: 0,
            status_scroll: 0,
            should_quit: false,
        };
        app.refresh_units().await?;
        Ok(app)
    }

    pub async fn run(&mut self, terminal: &mut TuiTerminal) -> Result<()> {
        let mut table_state = TableState::default();
        loop {
            let mut signal_updates = 0usize;
            while let Ok(_signal) = self.signal_rx.try_recv() {
                signal_updates += 1;
            }
            if signal_updates > 0 {
                if let Err(err) = self.refresh_units().await {
                    self.status = format!("signal refresh error: {err}");
                } else {
                    self.status = format!("dbus events: {signal_updates} unit changes");
                }
            }

            if self.last_refresh.elapsed() >= self.config.refresh_interval
                && let Err(err) = self.refresh_units().await
            {
                self.status = format!("refresh error: {err}");
            }

            terminal.draw(|frame| ui::draw(frame, self, &mut table_state))?;

            if event::poll(Duration::from_millis(100))? {
                let ev = event::read()?;
                if let Event::Key(key) = ev
                    && key.kind == KeyEventKind::Press
                {
                    self.on_key(key.code).await?;
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    pub fn selected_index(&self) -> Option<usize> {
        if self.filtered_indices.is_empty() {
            None
        } else {
            Some(self.selected.min(self.filtered_indices.len() - 1))
        }
    }

    pub fn selected_unit(&self) -> Option<&SystemdUnit> {
        self.selected_index()
            .and_then(|visible_idx| self.filtered_indices.get(visible_idx).copied())
            .and_then(|unit_idx| self.units.get(unit_idx))
    }

    pub fn visible_units(&self) -> impl Iterator<Item = &SystemdUnit> {
        self.filtered_indices
            .iter()
            .filter_map(|idx| self.units.get(*idx))
    }

    pub fn visible_count(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn connection_label(&self) -> &str {
        self.manager.connection_label()
    }

    pub fn state_filter_label(&self) -> &'static str {
        self.state_filter.label()
    }

    pub fn name_filter(&self) -> &str {
        &self.name_filter
    }

    pub fn filter_input_mode(&self) -> bool {
        self.filter_input_mode
    }

    pub fn show_help(&self) -> bool {
        self.show_help
    }

    pub fn help_scroll(&self) -> u16 {
        self.help_scroll
    }

    pub fn focus_block(&self) -> FocusBlock {
        self.focus_block
    }

    pub fn details_scroll(&self) -> u16 {
        self.details_scroll
    }

    pub fn status_scroll(&self) -> u16 {
        self.status_scroll
    }

    pub fn details_lines(&self) -> Vec<String> {
        if let Some(unit) = self.selected_unit() {
            vec![
                format!("Name: {}", unit.name),
                format!("Description: {}", unit.description),
                format!("Active: {}", unit.active_state.as_str()),
                format!("Sub: {}", unit.sub_state),
                format!("Load: {:?}", unit.load_state),
                format!(
                    "Follows: {}",
                    if unit.follows.is_empty() {
                        "-"
                    } else {
                        &unit.follows
                    }
                ),
                format!("Path: {}", unit.path),
            ]
        } else {
            vec!["No unit selected".to_string()]
        }
    }

    pub fn status_lines(&self) -> Vec<String> {
        vec![
            format!("Status: {}", self.status),
            format!(
                "Focus: {} (Tab/Shift+Tab to switch)",
                self.focus_block.label()
            ),
            format!(
                "Filters: state={}, name='{}'",
                self.state_filter.label(),
                self.name_filter
            ),
            format!(
                "Keys: ↑/↓ or j/k scroll focused block, PgUp/PgDn faster, g/G top-bottom, / filter, F2 state, s/t/R/L actions"
            ),
        ]
    }

    async fn refresh_units(&mut self) -> Result<()> {
        self.units = self.manager.list_units().await?;
        self.units.sort_by(|a, b| a.name.cmp(&b.name));
        self.rebuild_filtered_indices();
        self.last_refresh = Instant::now();
        self.status = format!("updated at {}", chrono::Local::now().format("%H:%M:%S"));
        Ok(())
    }

    async fn on_key(&mut self, key: KeyCode) -> Result<()> {
        if self.show_help {
            return self.on_help_key(key);
        }

        if self.filter_input_mode {
            return self.on_filter_input_key(key);
        }

        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('h') | KeyCode::F(1) => {
                self.show_help = true;
                self.help_scroll = 0;
            }
            KeyCode::Tab => self.focus_block = self.focus_block.next(),
            KeyCode::BackTab => self.focus_block = self.focus_block.prev(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_focused_down(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_focused_up(1),
            KeyCode::PageDown => self.scroll_focused_down(8),
            KeyCode::PageUp => self.scroll_focused_up(8),
            KeyCode::Char('g') => self.scroll_focused_top(),
            KeyCode::Char('G') => {
                self.scroll_focused_bottom();
            }
            KeyCode::Char('r') => {
                self.refresh_units().await?;
            }
            KeyCode::Char('/') => {
                self.filter_input_mode = true;
                self.status = "name filter mode: type and press Enter/Esc".to_string();
            }
            KeyCode::F(2) => {
                self.state_filter = self.state_filter.cycle();
                self.rebuild_filtered_indices();
                self.status = format!("state filter: {}", self.state_filter.label());
            }
            KeyCode::Char('s') => {
                let selected = self.selected_unit().cloned();
                if let Some(unit_name) = selected.as_ref().map(|u| u.name.clone()) {
                    execute(&self.manager, selected.as_ref(), UnitAction::Start).await?;
                    self.status = format!("start requested for {unit_name}");
                    self.refresh_units().await?;
                }
            }
            KeyCode::Char('t') => {
                let selected = self.selected_unit().cloned();
                if let Some(unit_name) = selected.as_ref().map(|u| u.name.clone()) {
                    execute(&self.manager, selected.as_ref(), UnitAction::Stop).await?;
                    self.status = format!("stop requested for {unit_name}");
                    self.refresh_units().await?;
                }
            }
            KeyCode::Char('R') => {
                let selected = self.selected_unit().cloned();
                if let Some(unit_name) = selected.as_ref().map(|u| u.name.clone()) {
                    execute(&self.manager, selected.as_ref(), UnitAction::Restart).await?;
                    self.status = format!("restart requested for {unit_name}");
                    self.refresh_units().await?;
                }
            }
            KeyCode::Char('L') => {
                let selected = self.selected_unit().cloned();
                if let Some(unit_name) = selected.as_ref().map(|u| u.name.clone()) {
                    execute(&self.manager, selected.as_ref(), UnitAction::Reload).await?;
                    self.status = format!("reload requested for {unit_name}");
                    self.refresh_units().await?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_help_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('h') | KeyCode::Char('q') => {
                self.show_help = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.help_scroll = self.help_scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.help_scroll = self.help_scroll.saturating_add(1);
            }
            KeyCode::PageUp => {
                self.help_scroll = self.help_scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                self.help_scroll = self.help_scroll.saturating_add(8);
            }
            KeyCode::Char('g') => {
                self.help_scroll = 0;
            }
            KeyCode::Char('G') => {
                self.help_scroll = u16::MAX;
            }
            _ => {}
        }
        Ok(())
    }

    fn on_filter_input_key(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Enter => {
                self.filter_input_mode = false;
                self.status = format!("name filter: '{}'", self.name_filter);
            }
            KeyCode::Backspace => {
                self.name_filter.pop();
                self.rebuild_filtered_indices();
            }
            KeyCode::Char(c) => {
                self.name_filter.push(c);
                self.rebuild_filtered_indices();
            }
            _ => {}
        }
        Ok(())
    }

    fn rebuild_filtered_indices(&mut self) {
        let needle = self.name_filter.to_lowercase();
        self.filtered_indices = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| {
                self.matches_state_filter(unit)
                    && (needle.is_empty()
                        || unit.name.to_lowercase().contains(&needle)
                        || unit.description.to_lowercase().contains(&needle))
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    fn matches_state_filter(&self, unit: &SystemdUnit) -> bool {
        match self.state_filter {
            StateFilter::All => true,
            StateFilter::Active => unit.active_state.as_str() == "active",
            StateFilter::Inactive => unit.active_state.as_str() == "inactive",
            StateFilter::Failed => unit.active_state.as_str() == "failed",
        }
    }

    fn scroll_focused_down(&mut self, amount: usize) {
        match self.focus_block {
            FocusBlock::Units => {
                if self.filtered_indices.is_empty() {
                    self.selected = 0;
                    return;
                }
                self.selected = (self.selected + amount).min(self.filtered_indices.len() - 1);
            }
            FocusBlock::Details => {
                self.details_scroll = self.details_scroll.saturating_add(amount as u16);
            }
            FocusBlock::Status => {
                self.status_scroll = self.status_scroll.saturating_add(amount as u16);
            }
        }
    }

    fn scroll_focused_up(&mut self, amount: usize) {
        match self.focus_block {
            FocusBlock::Units => {
                self.selected = self.selected.saturating_sub(amount);
            }
            FocusBlock::Details => {
                self.details_scroll = self.details_scroll.saturating_sub(amount as u16);
            }
            FocusBlock::Status => {
                self.status_scroll = self.status_scroll.saturating_sub(amount as u16);
            }
        }
    }

    fn scroll_focused_top(&mut self) {
        match self.focus_block {
            FocusBlock::Units => self.selected = 0,
            FocusBlock::Details => self.details_scroll = 0,
            FocusBlock::Status => self.status_scroll = 0,
        }
    }

    fn scroll_focused_bottom(&mut self) {
        match self.focus_block {
            FocusBlock::Units => {
                if !self.filtered_indices.is_empty() {
                    self.selected = self.filtered_indices.len() - 1;
                }
            }
            FocusBlock::Details => self.details_scroll = u16::MAX,
            FocusBlock::Status => self.status_scroll = u16::MAX,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FocusBlock, StateFilter};

    #[test]
    fn state_filter_cycle_roundtrip() {
        let mut state = StateFilter::All;
        state = state.cycle();
        assert_eq!(state, StateFilter::Active);
        state = state.cycle();
        assert_eq!(state, StateFilter::Inactive);
        state = state.cycle();
        assert_eq!(state, StateFilter::Failed);
        state = state.cycle();
        assert_eq!(state, StateFilter::All);
    }

    #[test]
    fn focus_block_next_prev_roundtrip() {
        let units = FocusBlock::Units;
        let details = units.next();
        let status = details.next();
        assert_eq!(details, FocusBlock::Details);
        assert_eq!(status, FocusBlock::Status);
        assert_eq!(status.next(), FocusBlock::Units);

        assert_eq!(FocusBlock::Units.prev(), FocusBlock::Status);
        assert_eq!(FocusBlock::Status.prev(), FocusBlock::Details);
        assert_eq!(FocusBlock::Details.prev(), FocusBlock::Units);
    }
}
