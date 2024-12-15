use crate::unit::ServiceType;
use crate::unit::WpmUnit;
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tabled::Table;
use tabled::Tabled;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessManagerError {
    #[error("{0} is not a registered unit")]
    UnregisteredUnit(String),
    #[error("{0} is already running")]
    AlreadyRunning(String),
    #[error("{0} is not running")]
    NotRunning(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("{0} did not spawn a process with a handle")]
    NoHandle(String),
}

pub struct ProcessManager {
    units: HashMap<String, WpmUnit>,
    running: Arc<Mutex<HashMap<String, u32>>>,
    completed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
}

impl ProcessManager {
    pub fn unit_directory() -> PathBuf {
        let home = dirs::home_dir().expect("could not find home dir");
        let dir = home.join(".config").join("wpm");

        if !dir.is_dir() {
            std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm");
        }

        dir
    }

    pub fn init() -> Result<Self, ProcessManagerError> {
        let mut pm = ProcessManager {
            units: Default::default(),
            running: Arc::new(Default::default()),
            completed: Arc::new(Default::default()),
        };

        pm.load_units()?;
        Ok(pm)
    }

    pub fn load_units(&mut self) -> Result<(), ProcessManagerError> {
        let read_dir = std::fs::read_dir(Self::unit_directory())?;

        let mut units = vec![];

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("toml")) {
                units.push(path)
            }
        }

        for path in units {
            let mut unit: WpmUnit = toml::from_str(&std::fs::read_to_string(path)?)?;
            for arg in unit.service.arguments.iter_mut().flatten() {
                *arg = arg.replace(
                    "$USERPROFILE",
                    dirs::home_dir()
                        .expect("could not find home dir")
                        .to_str()
                        .unwrap(),
                );
            }

            for (_, value) in unit.service.environment.iter_mut().flatten() {
                *value = value.replace(
                    "$USERPROFILE",
                    dirs::home_dir()
                        .expect("could not find home dir")
                        .to_str()
                        .unwrap(),
                );
            }

            self.register(unit);
        }

        Ok(())
    }

    pub fn register(&mut self, wpm_unit: WpmUnit) {
        let name = wpm_unit.unit.name.clone();
        self.units.insert(wpm_unit.unit.name.clone(), wpm_unit);
        tracing::info!("registered unit: {name}");
    }

    pub fn start(&mut self, unit_name: &str) -> Result<(), ProcessManagerError> {
        let unit = self
            .units
            .get(unit_name)
            .cloned()
            .ok_or(ProcessManagerError::UnregisteredUnit(unit_name.to_string()))?;

        if self.running.lock().contains_key(unit_name) {
            return Err(ProcessManagerError::AlreadyRunning(unit_name.to_string()));
        }

        for dependency in unit.unit.requires.iter().flatten() {
            tracing::info!("{unit_name} has a dependency on {dependency}");
            let dependency_unit = self.units.get(dependency).cloned().ok_or(
                ProcessManagerError::UnregisteredUnit(dependency.to_string()),
            )?;

            if !self.running.lock().contains_key(&dependency_unit.unit.name) {
                let dependency_name = dependency_unit.unit.name.clone();
                self.start(&dependency_name)?;

                if let Some(healthcheck) = &dependency_unit.service.healthcheck {
                    tracing::info!("{dependency} has a healthcheck defined: {healthcheck}");
                    let healthcheck_components = healthcheck.split_whitespace().collect::<Vec<_>>();
                    let mut healthcheck_command = Command::new(healthcheck_components[0]);
                    for component in healthcheck_components[1..].iter() {
                        let component = component.replace(
                            "$USERPROFILE",
                            dirs::home_dir()
                                .expect("could not find home dir")
                                .to_str()
                                .unwrap(),
                        );
                        healthcheck_command.arg(component);
                    }

                    healthcheck_command.stdout(std::process::Stdio::null());
                    healthcheck_command.stderr(std::process::Stdio::null());

                    let mut status = healthcheck_command.spawn()?.wait()?;

                    while !status.success() {
                        tracing::warn!(
                            "{dependency} failed its healthcheck command, retrying in 2s"
                        );
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        status = healthcheck_command.spawn()?.wait()?;
                    }

                    tracing::info!("{dependency} is healthy");
                }
            }
        }

        let mut command = Command::from(&unit);
        let mut child = command.spawn()?;
        let id = child.id();

        self.running.lock().insert(unit_name.to_string(), id);
        tracing::info!("starting unit: {unit_name}");

        let running = self.running.clone();
        let completed = self.completed.clone();
        let name = unit_name.to_string();
        let kind = unit.service.kind.unwrap_or_default();

        std::thread::spawn(move || {
            match child.wait() {
                Ok(exit_status) => {
                    if exit_status.success() {
                        if matches!(kind, ServiceType::Oneshot) {
                            completed.lock().insert(name.clone(), Utc::now());
                        }

                        tracing::info!("finished unit: {name}");
                    } else {
                        tracing::warn!("killed unit: {name}");
                    }
                }
                Err(error) => {
                    tracing::error!("{error}");
                }
            }

            running.lock().remove(&name);
        });

        Ok(())
    }

    pub fn stop(&mut self, unit_name: &str) -> Result<(), ProcessManagerError> {
        let unit = self
            .units
            .get(unit_name)
            .cloned()
            .ok_or(ProcessManagerError::UnregisteredUnit(unit_name.to_string()))?;

        let running = self.running.lock();

        tracing::info!("stopping unit: {unit_name}");

        match unit.service.shutdown {
            None => {
                let child = *running
                    .get(unit_name)
                    .ok_or(ProcessManagerError::NotRunning(unit_name.to_string()))?;

                Command::new("taskkill")
                    .args(["/F", "/PID", &child.to_string()])
                    .output()?;
            }
            Some(shutdown) => {
                tracing::info!("running shutdown command: {shutdown}");
                let shutdown_components = shutdown.split_whitespace().collect::<Vec<_>>();
                let mut shutdown_command = Command::new(shutdown_components[0]);
                for component in shutdown_components[1..].iter() {
                    let component = component.replace(
                        "$USERPROFILE",
                        dirs::home_dir()
                            .expect("could not find home dir")
                            .to_str()
                            .unwrap(),
                    );
                    shutdown_command.arg(component);
                }

                shutdown_command.stdout(std::process::Stdio::null());
                shutdown_command.stderr(std::process::Stdio::null());

                shutdown_command.output()?;
            }
        }

        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), ProcessManagerError> {
        tracing::info!("shutting down process manager");

        let mut units = vec![];
        for unit in self.running.lock().keys() {
            units.push(unit.clone());
        }

        for unit in units {
            self.stop(&unit)?;
        }

        Ok(())
    }

    pub fn state(&self) -> State {
        let mut units = vec![];
        let running = self.running.lock();
        let completed = self.completed.lock();

        for name in self.units.keys() {
            if let Some(pid) = running.get(name) {
                units.push(UnitStatus {
                    name: name.clone(),
                    state: UnitState::Running(*pid),
                })
            } else if let Some(timestamp) = completed.get(name) {
                units.push(UnitStatus {
                    name: name.clone(),
                    state: UnitState::Completed(*timestamp),
                })
            } else {
                units.push(UnitStatus {
                    name: name.clone(),
                    state: UnitState::Stopped,
                })
            }
        }

        State(units)
    }
}

pub struct State(Vec<UnitStatus>);

impl State {
    pub fn as_table(&self) -> String {
        Table::new(&self.0).to_string()
    }
}

#[derive(Tabled)]
pub enum UnitState {
    Running(u32),
    Stopped,
    Completed(DateTime<Utc>),
    Failed(u32),
}

impl Display for UnitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitState::Running(pid) => write!(f, "Running: {pid}"),
            UnitState::Stopped => write!(f, "Stopped"),
            UnitState::Completed(timestamp) => {
                let local: DateTime<Local> = DateTime::from(*timestamp);
                write!(f, "Completed: {local}")
            }
            UnitState::Failed(exit_code) => write!(f, "Failed: {exit_code}"),
        }
    }
}

#[derive(Tabled)]
pub struct UnitStatus {
    name: String,
    state: UnitState,
}
