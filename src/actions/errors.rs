use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("no unit selected")]
    NoSelection,
}
