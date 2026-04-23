use crate::actions::errors::ActionError;
use crate::units::{SystemdManager, SystemdUnit};
use anyhow::Result;

#[derive(Debug, Clone, Copy)]
pub enum UnitAction {
    Start,
    Stop,
    Restart,
    Reload,
}

pub async fn execute(
    manager: &SystemdManager,
    unit: Option<&SystemdUnit>,
    action: UnitAction,
) -> Result<()> {
    let unit = unit.ok_or(ActionError::NoSelection)?;

    match action {
        UnitAction::Start => manager.start_unit(&unit.name).await,
        UnitAction::Stop => manager.stop_unit(&unit.name).await,
        UnitAction::Restart => manager.restart_unit(&unit.name).await,
        UnitAction::Reload => manager.reload_unit(&unit.name).await,
    }
}
