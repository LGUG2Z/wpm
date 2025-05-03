use crate::communication::send_message;
use crate::process_manager_status::ProcessManagerStatus;
use crate::unit::Definition;
use crate::unit::Executable;
use crate::unit::Healthcheck;
use crate::unit::RestartStrategy;
use crate::unit::ServiceKind;
use crate::unit_status::DisplayedOption;
use crate::unit_status::UnitState;
use crate::unit_status::UnitStatus;
use crate::SocketMessage;
use chrono::DateTime;
use chrono::Local;
use chrono::Utc;
use parking_lot::Mutex;
use shared_child::SharedChild;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessManagerError {
    #[error("{0} is not a registered unit")]
    UnregisteredUnit(String),
    #[error("{0} is already running")]
    RunningUnit(String),
    #[error("{0} is marked as completed; reset unit before trying again")]
    CompletedUnit(String),
    #[error("{0} failed its healthcheck; reset unit before trying again")]
    FailedHealthcheck(String),
    #[error("{0} is not running")]
    NotRunning(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("{0} did not spawn a process with a handle")]
    NoHandle(String),
    #[error("a forking service must have a process healthcheck target defined")]
    InvalidForkingService,
    #[error("a simple service cannot have a separate process healthcheck target")]
    InvalidSimpleService,
    #[error("hash mismatch (expected {expected}, actual {actual})")]
    HashMismatch { expected: String, actual: String },
}

#[derive(Clone)]
pub enum Child {
    Shared(Arc<SharedChild>),
    Pid(u32),
}

impl Child {
    pub fn id(&self) -> u32 {
        match self {
            Child::Shared(shared) => shared.id(),
            Child::Pid(id) => *id,
        }
    }

    pub fn kill(&self) -> std::io::Result<()> {
        match self {
            Child::Shared(shared) => shared.kill(),
            Child::Pid(id) => {
                let pid = *id;
                let s = System::new_all();
                if let Some(process) = s.process(Pid::from_u32(pid)) {
                    process.kill();
                }

                Ok(())
            }
        }
    }

    pub fn wait(&self) -> std::io::Result<ExitStatus> {
        match self {
            Self::Shared(child) => child.wait(),
            Self::Pid(pid) => {
                let mut system = System::new_all();
                let pid = Pid::from_u32(*pid);
                system.refresh_processes_specifics(
                    ProcessesToUpdate::Some(&[pid]),
                    true,
                    ProcessRefreshKind::everything(),
                );

                if let Some(process) = system.process(pid) {
                    process.wait().ok_or_else(|| {
                        std::io::Error::new(std::io::ErrorKind::Other, "Process wait returned None")
                    })
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Process not found",
                    ))
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ProcessState {
    pub child: Child,
    pub timestamp: DateTime<Utc>,
}

pub struct ProcessManager {
    definitions: HashMap<String, Definition>,
    running: Arc<Mutex<HashMap<String, ProcessState>>>,
    completed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    failed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    terminated: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
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

    pub(crate) fn find_exe(exe_name: &Path) -> Option<PathBuf> {
        let mut name = exe_name.to_path_buf();
        if name.extension().is_none() {
            name = exe_name.with_extension("exe");
        }

        if let Ok(paths) = std::env::var("PATH") {
            for path in std::env::split_paths(&paths) {
                let full_path = path.join(&name);
                if full_path.is_file() {
                    return Some(full_path);
                }
            }
        }
        None
    }

    pub fn init(path: Option<PathBuf>) -> Result<Self, ProcessManagerError> {
        let mut pm = ProcessManager {
            definitions: Default::default(),
            running: Arc::new(Default::default()),
            completed: Arc::new(Default::default()),
            failed: Arc::new(Default::default()),
            terminated: Arc::new(Default::default()),
        };

        pm.load_units(path)?;
        pm.autostart();

        Ok(pm)
    }

    pub fn autostart(&mut self) {
        let mut autostart = vec![];

        for (name, def) in &self.definitions {
            if def.service.autostart {
                autostart.push(name.clone());
            }
        }

        for name in &autostart {
            tracing::info!("{name}: autostarting");
            if let Err(error) = self.start(name) {
                tracing::error!("{error}");
            }
        }
    }

    pub fn retrieve_units(path: Option<PathBuf>) -> Result<Vec<Definition>, ProcessManagerError> {
        let unit_dir = if let Some(path) = path {
            path
        } else {
            Self::unit_directory()
        };

        let read_dir = std::fs::read_dir(unit_dir)?;

        let mut paths = vec![];

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                #[allow(clippy::if_same_then_else)]
                if path.extension() == Some(OsStr::new("json")) {
                    paths.push(path);
                } else if path.extension() == Some(OsStr::new("toml"))
                    && path.file_name() != Some(OsStr::new("taplo.toml"))
                    && path.file_name() != Some(OsStr::new(".taplo.toml"))
                {
                    paths.push(path);
                }
            }
        }

        let mut units = vec![];

        for path in paths {
            let definition: Definition = match path.extension() {
                Some(extension) => match extension.to_string_lossy().to_string().as_str() {
                    "json" => serde_json::from_str(&std::fs::read_to_string(path)?)?,
                    "toml" => toml::from_str(&std::fs::read_to_string(path)?)?,
                    _ => continue,
                },
                None => continue,
            };

            units.push(definition);
        }

        Ok(units)
    }

    pub fn load_units(&mut self, path: Option<PathBuf>) -> Result<(), ProcessManagerError> {
        let unit_dir = if let Some(path) = path {
            path
        } else {
            Self::unit_directory()
        };

        let read_dir = std::fs::read_dir(unit_dir)?;

        let mut units = vec![];

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                #[allow(clippy::if_same_then_else)]
                if path.extension() == Some(OsStr::new("json")) {
                    units.push(path);
                } else if path.extension() == Some(OsStr::new("toml"))
                    && path.file_name() != Some(OsStr::new("taplo.toml"))
                    && path.file_name() != Some(OsStr::new(".taplo.toml"))
                {
                    units.push(path);
                }
            }
        }

        for path in units {
            let mut definition: Definition = match path.extension() {
                Some(extension) => match extension.to_string_lossy().to_string().as_str() {
                    "json" => serde_json::from_str(&std::fs::read_to_string(path)?)?,
                    "toml" => toml::from_str(&std::fs::read_to_string(path)?)?,
                    _ => continue,
                },
                None => continue,
            };

            definition.resolve_resources()?;

            if matches!(definition.service.kind, ServiceKind::Forking) {
                let mut is_valid_forking_service = false;
                if let Some(Healthcheck::Process(proc)) = &definition.service.healthcheck {
                    if proc.target.is_some() {
                        is_valid_forking_service = true;
                    }
                }

                if !is_valid_forking_service {
                    return Err(ProcessManagerError::InvalidForkingService);
                }
            }

            if matches!(definition.service.kind, ServiceKind::Simple) {
                let mut is_invalid_simple_service = false;
                if let Some(Healthcheck::Process(proc)) = &definition.service.healthcheck {
                    if proc.target.is_some() {
                        is_invalid_simple_service = true;
                    }
                }

                if is_invalid_simple_service {
                    return Err(ProcessManagerError::InvalidSimpleService);
                }
            }

            let home_dir = dirs::home_dir()
                .expect("could not find home dir")
                .to_str()
                .unwrap()
                .to_string();

            if let Some(working_directory) = definition.service.working_directory.as_mut() {
                let stringified = working_directory.to_string_lossy();
                let stringified = stringified.replace("$USERPROFILE", &home_dir);
                let directory = PathBuf::from(stringified);

                *working_directory = directory;
            }

            if let Some(environment_file) = &definition.service.environment_file {
                let stringified = environment_file.to_string_lossy();
                let stringified = stringified.replace("$USERPROFILE", &home_dir);
                let environment_file = PathBuf::from(stringified);

                if let Ok(environment) =
                    serde_envfile::from_file::<serde_envfile::Value>(&environment_file)
                {
                    for (k, v) in environment.iter() {
                        match &mut definition.service.environment {
                            None => {
                                definition.service.environment = Some(vec![(k.clone(), v.clone())])
                            }
                            Some(e) => {
                                e.push((k.clone(), v.clone()));
                            }
                        }
                    }
                }
            }

            for (_, value) in definition.service.environment.iter_mut().flatten() {
                *value = value.replace("$USERPROFILE", &home_dir);
            }

            for cmd in definition.service.exec_start_pre.iter_mut().flatten() {
                cmd.resolve_user_profile();
            }

            definition.service.exec_start.resolve_user_profile();

            for cmd in definition.service.exec_start_post.iter_mut().flatten() {
                cmd.resolve_user_profile();
            }

            for cmd in definition.service.exec_stop.iter_mut().flatten() {
                cmd.resolve_user_profile();
            }

            for cmd in definition.service.exec_stop_post.iter_mut().flatten() {
                cmd.resolve_user_profile();
            }

            if definition
                .service
                .exec_start
                .executable
                .pathbuf()?
                .canonicalize()
                .is_err()
            {
                match Self::find_exe(&definition.service.exec_start.executable.pathbuf()?) {
                    Some(path) => {
                        definition.service.exec_start.executable = Executable::Local(path)
                    }
                    None => {
                        tracing::warn!(
                            "{}: could not find executable in $PATH, skipping unit",
                            definition.unit.name
                        );
                        continue;
                    }
                }
            }

            for command in definition.service.exec_start_pre.iter_mut().flatten() {
                if command.executable.pathbuf()?.canonicalize().is_err() {
                    match Self::find_exe(&command.executable.pathbuf()?) {
                        Some(path) => command.executable = Executable::Local(path),
                        None => {
                            tracing::warn!(
                            "{}: could not find pre-start command executable in $PATH, skipping unit",
                            definition.unit.name
                        );
                            continue;
                        }
                    }
                }
            }

            for command in definition.service.exec_start_post.iter_mut().flatten() {
                if command.executable.pathbuf()?.canonicalize().is_err() {
                    match Self::find_exe(&command.executable.pathbuf()?) {
                        Some(path) => command.executable = Executable::Local(path),
                        None => {
                            tracing::warn!(
                            "{}: could not find post-start command executable in $PATH, skipping unit",
                            definition.unit.name
                        );
                            continue;
                        }
                    }
                }
            }

            for command in definition.service.exec_stop.iter_mut().flatten() {
                if command.executable.pathbuf()?.canonicalize().is_err() {
                    match Self::find_exe(&command.executable.pathbuf()?) {
                        Some(path) => command.executable = Executable::Local(path),
                        None => {
                            tracing::warn!(
                            "{}: could not find shutdown command executable in $PATH, skipping unit",
                            definition.unit.name
                        );
                            continue;
                        }
                    }
                }
            }

            for command in definition.service.exec_stop_post.iter_mut().flatten() {
                if command.executable.pathbuf()?.canonicalize().is_err() {
                    match Self::find_exe(&command.executable.pathbuf()?) {
                        Some(path) => command.executable = Executable::Local(path),
                        None => {
                            tracing::warn!(
                            "{}: could not find cleanup command executable in $PATH, skipping unit",
                            definition.unit.name
                        );
                            continue;
                        }
                    }
                }
            }

            if matches!(definition.service.kind, ServiceKind::Simple)
                && definition.service.healthcheck.is_none()
            {
                definition.service.healthcheck = Some(Healthcheck::default());
            }

            if matches!(definition.service.kind, ServiceKind::Oneshot)
                && definition.service.healthcheck.is_some()
            {
                definition.service.healthcheck = None;
            }

            if let Some(Healthcheck::Command(command)) = &mut definition.service.healthcheck {
                command.resolve_user_profile();

                if command.executable.canonicalize().is_err() {
                    match Self::find_exe(&command.executable) {
                        Some(path) => command.executable = path,
                        None => {
                            tracing::warn!(
                            "{}: could not find healthcheck command executable in $PATH, skipping unit",
                            definition.unit.name
                        );
                            continue;
                        }
                    }
                }
            }

            self.register(definition);
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

        self.failed.lock().remove(name);
        self.terminated.lock().remove(name);

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

        let mut retry_limit = definition.service.exec_start.retry_limit.unwrap_or(5);
        let mut process_id = None;
        while retry_limit > 0 {
            let id = definition.execute(
                self.running.clone(),
                self.completed.clone(),
                self.terminated.clone(),
            )?;

            match definition.healthcheck(
                id.clone(),
                self.running.clone(),
                self.failed.clone(),
                self.terminated.clone(),
            ) {
                Ok(_) => {
                    process_id = Some(id);
                    break;
                }
                Err(error) => {
                    retry_limit -= 1;
                    if retry_limit == 0 {
                        return Err(error);
                    }
                }
            }
        }

        #[allow(clippy::unwrap_used)]
        Ok(process_id.unwrap())
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
            .cloned()
            .ok_or(ProcessManagerError::NotRunning(name.to_string()))?;

        let id = proc_state.child.id();

        tracing::info!("{name}: stopping unit");

        if let Some(shutdown_commands) = unit.service.exec_stop {
            for command in shutdown_commands {
                let stringified = if let Some(args) = &command.arguments {
                    format!("{} {}", command.executable, args.join(" "))
                } else {
                    command.executable.to_string()
                };

                tracing::info!("{name}: executing shutdown command - {stringified}");
                let mut command = command.to_silent_command(unit.service.environment.clone());
                command.output()?;
            }
        }

        tracing::info!("{name}: sending kill signal to {id}");

        // remove first to avoid race condition with the other child.wait()
        // call spawned in a thread by Unit.execute()
        let tmp_proc_state = running.remove(name).unwrap();

        if let Err(error) = proc_state.child.kill() {
            // If there are any errors in killing the process, it's still considered to be running
            // so we reinsert before returning the errors
            running.insert(name.to_string(), tmp_proc_state);
            return Err(error.into());
        }

        if let Err(error) = proc_state.child.wait() {
            if matches!(error.kind(), std::io::ErrorKind::NotFound) {
                tracing::warn!("{name}: process {id} not found; assuming successful termination");
            } else {
                running.insert(name.to_string(), tmp_proc_state);
                return Err(error.into());
            }
        }

        tracing::info!("{name}: process {id} successfully terminated");

        if let Some(cleanup_commands) = unit.service.exec_stop_post {
            for command in cleanup_commands {
                let stringified = if let Some(args) = &command.arguments {
                    format!("{} {}", command.executable, args.join(" "))
                } else {
                    command.executable.to_string()
                };

                tracing::info!("{name}: executing cleanup command - {stringified}");
                let mut command = command.to_silent_command(unit.service.environment.clone());
                command.output()?;
            }
        }

        let thread_name = name.to_string();
        if matches!(unit.service.restart, RestartStrategy::Always) {
            std::thread::spawn(move || {
                let restart_sec = unit.service.restart_sec.unwrap_or(1);
                tracing::info!("{thread_name}: restarting terminated process in {restart_sec}s");
                std::thread::sleep(Duration::from_secs(restart_sec));

                if let Err(error) =
                    send_message("wpmd.sock", SocketMessage::Start(vec![thread_name.clone()]))
                {
                    tracing::error!("{thread_name}: {error}");
                }
            });
        }

        Ok(())
    }

    pub fn reset(&mut self, name: &str) {
        tracing::info!("{name}: resetting unit");
        self.completed.lock().remove(name);
        self.failed.lock().remove(name);
        self.terminated.lock().remove(name);
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
        let terminated = self.terminated.lock();

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
            } else if let Some(timestamp) = terminated.get(name) {
                let local: DateTime<Local> = DateTime::from(*timestamp);

                units.push((
                    def.clone(),
                    UnitStatus {
                        name: name.clone(),
                        kind: def.service.kind,
                        state: UnitState::Terminated,
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
