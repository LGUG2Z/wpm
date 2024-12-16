use crate::process_manager_status::ProcessManagerStatus;
use crate::unit::Definition;
use crate::unit_status::DisplayedOption;
use crate::unit_status::UnitState;
use crate::unit_status::UnitStatus;
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use parking_lot::Mutex;
use shared_child::SharedChild;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessManagerError {
    #[error("{0} is not a registered unit")]
    UnregisteredUnit(String),
    #[error("{0} is already running")]
    RunningUnit(String),
    #[error("{0} is marked as completed; reset unit before trying again")]
    CompletedUnit(String),
    #[error("{0} is marked as failed; reset unit before trying again")]
    FailedUnit(String),
    #[error("{0} failed its healthcheck; reset unit before trying again")]
    FailedHealthcheck(String),
    #[error("{0} is not running")]
    NotRunning(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("{0} did not spawn a process with a handle")]
    NoHandle(String),
}

pub struct ProcessState {
    pub child: Arc<SharedChild>,
    pub timestamp: DateTime<Utc>,
}

pub struct ProcessManager {
    definitions: HashMap<String, Definition>,
    running: Arc<Mutex<HashMap<String, ProcessState>>>,
    completed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    failed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
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
            definitions: Default::default(),
            running: Arc::new(Default::default()),
            completed: Arc::new(Default::default()),
            failed: Arc::new(Default::default()),
        };

        pm.load_units()?;
        pm.autostart()?;

        Ok(pm)
    }

    pub fn autostart(&mut self) -> Result<(), ProcessManagerError> {
        let mut autostart = vec![];

        for (name, def) in &self.definitions {
            if def.service.autostart {
                autostart.push(name.clone());
            }
        }

        for name in &autostart {
            tracing::info!("{name}: autostarting");
            self.start(name)?;
        }

        Ok(())
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
            let mut unit: Definition = toml::from_str(&std::fs::read_to_string(path)?)?;
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

    pub fn register(&mut self, definition: Definition) {
        let name = definition.unit.name.clone();
        self.definitions
            .insert(definition.unit.name.clone(), definition);
        tracing::info!("{name}: registered unit");
    }

    pub fn start(&mut self, name: &str) -> Result<Arc<SharedChild>, ProcessManagerError> {
        let definition = self
            .definitions
            .get(name)
            .cloned()
            .ok_or(ProcessManagerError::UnregisteredUnit(name.to_string()))?;

        if self.running.lock().contains_key(name) {
            return Err(ProcessManagerError::RunningUnit(name.to_string()));
        }

        if self.completed.lock().contains_key(name) {
            return Err(ProcessManagerError::CompletedUnit(name.to_string()));
        }

        if self.failed.lock().contains_key(name) {
            return Err(ProcessManagerError::FailedUnit(name.to_string()));
        }

        for dep in definition.unit.requires.iter().flatten() {
            tracing::info!("{name}: requires {dep}");
            let dependency = self
                .definitions
                .get(dep)
                .cloned()
                .ok_or(ProcessManagerError::UnregisteredUnit(dep.to_string()))?;

            if !self.running.lock().contains_key(&dependency.unit.name) {
                self.start(&dependency.unit.name)?;
            }
        }

        let id = definition.execute(self.running.clone(), self.completed.clone())?;

        definition.healthcheck(id.clone(), self.running.clone(), self.failed.clone())?;

        Ok(id)
    }

    pub fn stop(&mut self, name: &str) -> Result<(), ProcessManagerError> {
        let unit = self
            .definitions
            .get(name)
            .cloned()
            .ok_or(ProcessManagerError::UnregisteredUnit(name.to_string()))?;

        let mut running = self.running.lock();

        let proc_state = running
            .get(name)
            .ok_or(ProcessManagerError::NotRunning(name.to_string()))?;

        let id = proc_state.child.id();

        tracing::info!("{name}: stopping unit");

        if let Some(shutdown_commands) = unit.service.shutdown {
            for shutdown in shutdown_commands {
                tracing::info!("{name}: running shutdown command - {shutdown}");
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

        tracing::info!("{name}: sending kill signal to {id}");

        proc_state.child.kill()?;
        proc_state.child.wait()?;

        tracing::info!("{name}: process {id} successfully terminated");

        running.remove(name);

        Ok(())
    }

    pub fn reset(&mut self, name: &str) {
        tracing::info!("{name}: resetting unit");
        self.completed.lock().remove(name);
        self.failed.lock().remove(name);
    }

    pub fn shutdown(&mut self) -> Result<(), ProcessManagerError> {
        tracing::info!("wpmd: shutting down process manager");

        let mut units = vec![];
        for unit in self.running.lock().keys() {
            units.push(unit.clone());
        }

        for unit in units {
            self.stop(&unit)?;
        }

        Ok(())
    }

    pub fn unit(&self, name: &str) -> Option<Definition> {
        self.definitions.get(name).cloned()
    }

    pub fn state(&self) -> ProcessManagerStatus {
        let mut units = vec![];
        let running = self.running.lock();
        let completed = self.completed.lock();
        let failed = self.failed.lock();

        for (name, def) in &self.definitions {
            if let Some(proc_state) = running.get(name) {
                let local: DateTime<Local> = DateTime::from(proc_state.timestamp);

                units.push((
                    def.clone(),
                    UnitStatus {
                        name: name.clone(),
                        kind: def.service.kind,
                        state: UnitState::Running,
                        pid: DisplayedOption(Some(proc_state.child.id())),
                        timestamp: DisplayedOption(Some(local.to_string())),
                    },
                ))
            } else if let Some(timestamp) = completed.get(name) {
                let local: DateTime<Local> = DateTime::from(*timestamp);

                units.push((
                    def.clone(),
                    UnitStatus {
                        name: name.clone(),
                        kind: def.service.kind,
                        state: UnitState::Completed,
                        pid: DisplayedOption(None),
                        timestamp: DisplayedOption(Some(local.to_string())),
                    },
                ))
            } else if let Some(timestamp) = failed.get(name) {
                let local: DateTime<Local> = DateTime::from(*timestamp);

                units.push((
                    def.clone(),
                    UnitStatus {
                        name: name.clone(),
                        kind: def.service.kind,
                        state: UnitState::Failed,
                        pid: DisplayedOption(None),
                        timestamp: DisplayedOption(Some(local.to_string())),
                    },
                ))
            } else {
                units.push((
                    def.clone(),
                    UnitStatus {
                        name: name.clone(),
                        kind: def.service.kind,
                        state: UnitState::Stopped,
                        pid: DisplayedOption(None),
                        timestamp: DisplayedOption(None),
                    },
                ))
            }
        }

        ProcessManagerStatus(units)
    }
}
