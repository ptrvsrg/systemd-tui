use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::cursor;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};

pub type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<TuiTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(cursor::Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    Ok(())
}
