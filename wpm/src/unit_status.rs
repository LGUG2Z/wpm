use crate::unit::ServiceKind;
use std::fmt::Display;
use std::fmt::Formatter;
use tabled::Tabled;

#[derive(Tabled)]
pub struct UnitStatus {
    pub name: String,
    pub kind: ServiceKind,
    pub state: UnitState,
    pub pid: DisplayedOption<u32>,
    pub timestamp: DisplayedOption<String>,
}

#[derive(Tabled)]
pub enum UnitState {
    Running,
    Stopped,
    Completed,
    Failed,
    Terminated,
}

impl Display for UnitState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitState::Running => write!(f, "Running"),
            UnitState::Stopped => write!(f, "Stopped"),
            UnitState::Completed => write!(f, "Completed"),
            UnitState::Failed => write!(f, "Failed"),
            UnitState::Terminated => write!(f, "Terminated"),
        }
    }
}

pub struct DisplayedOption<T>(pub Option<T>);

impl<T: Display> Display for DisplayedOption<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            None => write!(f, ""),
            Some(inner) => write!(f, "{inner}"),
        }
    }
}
