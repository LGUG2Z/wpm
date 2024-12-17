use crate::process_manager::ProcessManagerError;
use crate::process_manager::ProcessState;
use chrono::DateTime;
use chrono::Utc;
use parking_lot::Mutex;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use shared_child::SharedChild;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// A wpm definition
pub struct Definition {
    /// Information about this definition and its dependencies
    pub unit: Unit,
    /// Information about what this definition executes
    pub service: Service,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// Information about a wpm definition and its dependencies
pub struct Unit {
    /// Name of this definition, must be unique
    pub name: String,
    /// Description of this definition
    pub description: Option<String>,
    /// Dependencies of this definition, validated at runtime
    pub requires: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// Information about what a wpm definition executes
pub struct Service {
    #[serde(alias = "type")]
    #[serde(default)]
    /// Type of service definition
    pub kind: ServiceKind,
    /// Autostart this definition with wpmd
    #[serde(default)]
    pub autostart: bool,
    /// Executable name or absolute path to an executable
    pub executable: PathBuf,
    /// Arguments passed to the executable
    pub arguments: Option<Vec<String>>,
    /// Environment variables for this service definition
    pub environment: Option<Vec<(String, String)>>,
    /// Working directory for this service definition
    pub working_directory: Option<PathBuf>,
    #[serde(default)]
    /// Healthcheck for this service definition
    pub healthcheck: Healthcheck,
    /// Shutdown commands for this definition
    pub shutdown: Option<Vec<String>>,
}

impl Definition {
    pub fn execute(
        &self,
        running: Arc<Mutex<HashMap<String, ProcessState>>>,
        completed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    ) -> Result<Arc<SharedChild>, ProcessManagerError> {
        let name = self.unit.name.to_string();
        tracing::info!("{name}: starting unit");

        let mut command = Command::from(self);
        let child = SharedChild::spawn(&mut command)?;
        let thread_child = Arc::new(child);
        let state_child = thread_child.clone();

        let kind = self.service.kind;

        let completed_thread = completed.clone();
        let running_thread = running.clone();

        if matches!(kind, ServiceKind::Oneshot) {
            std::thread::spawn(move || {
                match thread_child.wait() {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            completed_thread.lock().insert(name.clone(), Utc::now());
                            tracing::info!(
                                "{name}: oneshot unit terminated with successful exit code {}",
                                exit_status.code().unwrap()
                            );
                        } else {
                            tracing::warn!(
                                "{name}: oneshot unit terminated with failure exit code {}",
                                exit_status.code().unwrap()
                            );
                        }
                    }
                    Err(error) => {
                        tracing::error!("{name}: {error}");
                    }
                }

                running_thread.lock().remove(&name);
            });
        }

        Ok(state_child)
    }

    pub fn healthcheck(
        &self,
        child: Arc<SharedChild>,
        running: Arc<Mutex<HashMap<String, ProcessState>>>,
        failed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    ) -> Result<(), ProcessManagerError> {
        let mut passed = false;
        let name = self.unit.name.clone();

        // we only want to healthcheck long-running services
        if !matches!(self.service.kind, ServiceKind::Simple) {
            return Ok(());
        }

        // we don't want to run redundant healthchecks
        if running.lock().contains_key(&name) {
            tracing::info!("{name}: passed healthcheck");
            return Ok(());
        }

        match &self.service.healthcheck {
            Healthcheck::Command(healthcheck) => {
                tracing::info!("{name}: running command healthcheck - {healthcheck}");
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
                let mut max_attempts = 5;

                while !status.success() && max_attempts > 0 {
                    tracing::warn!("{name}: failed healthcheck command, retrying in 2s");
                    std::thread::sleep(Duration::from_secs(2));
                    status = healthcheck_command.spawn()?.wait()?;
                    max_attempts -= 1;
                }

                if max_attempts > 0 {
                    passed = true;
                }
            }
            Healthcheck::LivenessSeconds(seconds) => {
                tracing::info!("{name}: running liveness healthcheck ({seconds}s)");
                std::thread::sleep(Duration::from_secs(*seconds));
                let mut system = System::new_all();
                let pid = Pid::from_u32(child.id());
                system.refresh_processes_specifics(
                    ProcessesToUpdate::Some(&[pid]),
                    true,
                    ProcessRefreshKind::everything(),
                );
                if system.process(pid).is_none() {
                    failed.lock().insert(self.unit.name.clone(), Utc::now());
                } else {
                    passed = true;
                }
            }
        }

        if passed {
            tracing::info!("{name}: passed healthcheck");
            running.lock().insert(
                name.clone(),
                ProcessState {
                    child,
                    timestamp: Utc::now(),
                },
            );
        } else {
            tracing::warn!("{name}: failed healthcheck");
            failed.lock().insert(name.clone(), Utc::now());
            return Err(ProcessManagerError::FailedUnit(name.to_string()));
        }

        Ok(())
    }

    pub fn log_path(&self) -> PathBuf {
        let home = dirs::home_dir().expect("could not find home dir");
        let dir = home.join(".config").join("wpm").join("logs");

        if !dir.is_dir() {
            std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm/logs");
        }

        dir.join(format!("{}.log", self.unit.name))
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub enum Healthcheck {
    Command(String),
    LivenessSeconds(u64),
}

impl Default for Healthcheck {
    fn default() -> Self {
        Self::LivenessSeconds(1)
    }
}

#[derive(Default, Serialize, Deserialize, Copy, Clone, JsonSchema)]
pub enum ServiceKind {
    #[default]
    Simple,
    Oneshot,
}

impl Display for ServiceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceKind::Simple => write!(f, "Simple"),
            ServiceKind::Oneshot => write!(f, "Oneshot"),
        }
    }
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

impl From<&Definition> for Command {
    fn from(value: &Definition) -> Self {
        let file = File::create(value.log_path()).unwrap();

        let stdout = file.try_clone().unwrap();
        let stderr = stdout.try_clone().unwrap();

        let mut command = Command::new(&value.service.executable);

        if let Some(environment) = &value.service.environment {
            command.envs(environment.clone());
        }

        if let Some(arguments) = &value.service.arguments {
            command.args(arguments);
        }

        if let Some(working_directory) = &value.service.working_directory {
            command.current_dir(working_directory);
        }

        command.creation_flags(CREATE_NO_WINDOW);
        command.stdout(stdout);
        command.stderr(stderr);
        command
    }
}
