use crate::communication::send_message;
use crate::process_manager::Child;
use crate::process_manager::ProcessManagerError;
use crate::process_manager::ProcessState;
use crate::wpm_data_dir;
use crate::SocketMessage;
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
use std::ops::Not;
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
#[serde(rename_all = "PascalCase")]
pub struct Definition {
    /// Information about this definition and its dependencies
    pub unit: Unit,
    /// Information about what this definition executes
    pub service: Service,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// Information about a wpm definition and its dependencies
#[serde(rename_all = "PascalCase")]
pub struct Unit {
    /// Name of this definition, must be unique
    pub name: String,
    /// Description of this definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Dependencies of this definition, validated at runtime
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<Vec<String>>,
}

#[derive(Default, Serialize, Deserialize, Copy, Clone, JsonSchema)]
/// Information about a wpm definition's restart strategy
pub enum RestartStrategy {
    #[default]
    Never,
    Always,
    OnFailure,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// Information about what a wpm definition executes
#[serde(rename_all = "PascalCase")]
pub struct Service {
    #[serde(alias = "Type")]
    #[serde(default)]
    /// Type of service definition
    pub kind: ServiceKind,
    /// Autostart this definition with wpmd
    #[serde(default)]
    #[serde(skip_serializing_if = "<&bool>::not")]
    pub autostart: bool,
    /// Commands executed before ExecStart in this service definition
    pub exec_start_pre: Option<Vec<ServiceCommand>>,
    /// Command executed by this service definition
    pub exec_start: ServiceCommand,
    /// Commands executed after ExecStart in this service definition
    pub exec_start_post: Option<Vec<ServiceCommand>>,
    /// Shutdown commands for this service definition
    pub exec_stop: Option<Vec<ServiceCommand>>,
    /// Post-shutdown cleanup commands for this service definition
    pub exec_stop_post: Option<Vec<ServiceCommand>>,
    /// Environment variables inherited by all commands in this service definition
    pub environment: Option<Vec<(String, String)>>,
    /// Working directory for this service definition
    pub working_directory: Option<PathBuf>,
    #[serde(default)]
    /// Healthcheck for this service definition
    pub healthcheck: Option<Healthcheck>,
    #[serde(default)]
    /// Restart strategy for this service definition
    pub restart: RestartStrategy,
    /// Time to sleep in seconds before attempting to restart service (default: 1s)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_sec: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
/// A wpm definition command
#[serde(rename_all = "PascalCase")]
pub struct ServiceCommand {
    /// Executable name or absolute path to an executable
    pub executable: PathBuf,
    /// Arguments passed to the executable
    pub arguments: Option<Vec<String>>,
    /// Environment variables for this command
    pub environment: Option<Vec<(String, String)>>,
}

impl ServiceCommand {
    pub fn resolve_user_profile(&mut self) {
        let home_dir = dirs::home_dir()
            .expect("could not find home dir")
            .to_str()
            .unwrap()
            .to_string();

        let stringified = self.executable.to_string_lossy();
        let stringified = stringified.replace("$USERPROFILE", &home_dir);
        let executable = PathBuf::from(stringified);
        self.executable = executable;

        for arg in self.arguments.iter_mut().flatten() {
            *arg = arg.replace("$USERPROFILE", &home_dir);
        }

        for (_, value) in self.environment.iter_mut().flatten() {
            *value = value.replace("$USERPROFILE", &home_dir);
        }
    }

    pub fn to_silent_command(&self, global_environment: Option<Vec<(String, String)>>) -> Command {
        let mut command = Command::new(&self.executable);
        if let Some(arguments) = &self.arguments {
            command.args(arguments);
        }

        if let Some(environment) = global_environment {
            command.envs(environment);
        }

        if let Some(environment) = &self.environment {
            command.envs(environment.clone());
        }

        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());

        command
    }
}

impl Definition {
    pub fn execute(
        &self,
        running: Arc<Mutex<HashMap<String, ProcessState>>>,
        completed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
        terminated: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    ) -> Result<Arc<SharedChild>, ProcessManagerError> {
        let name = self.unit.name.to_string();
        tracing::info!("{name}: starting unit");

        for command in self.service.exec_start_pre.iter().flatten() {
            let stringified = if let Some(args) = &command.arguments {
                format!(
                    "{} {}",
                    command.executable.to_string_lossy(),
                    args.join(" ")
                )
            } else {
                command.executable.to_string_lossy().to_string()
            };

            tracing::info!("{name}: executing pre-start command - {stringified}");
            let mut command = command.to_silent_command(self.service.environment.clone());
            command.output()?;
        }

        let mut command = Command::from(self);
        let child = SharedChild::spawn(&mut command)?;
        let thread_child = Arc::new(child);
        let state_child = thread_child.clone();

        let completed_thread = completed.clone();
        let running_thread = running.clone();
        let exec_start_post_thread = self.service.exec_start_post.clone();
        let exec_stop_thread = self.service.exec_stop.clone();
        let environment_thread = self.service.environment.clone();

        match self.service.kind {
            ServiceKind::Simple => {
                self.monitor_child(
                    Child::Shared(thread_child),
                    running_thread,
                    terminated.clone(),
                );
            }
            // oneshots block the main thread
            ServiceKind::Oneshot => {
                match thread_child.wait() {
                    Ok(exit_status) => {
                        if exit_status.success() {
                            completed_thread.lock().insert(name.clone(), Utc::now());
                            tracing::info!(
                                "{name}: oneshot unit terminated with successful exit code {}",
                                exit_status.code().unwrap()
                            );

                            for command in exec_start_post_thread.iter().flatten() {
                                let stringified = if let Some(args) = &command.arguments {
                                    format!(
                                        "{} {}",
                                        command.executable.to_string_lossy(),
                                        args.join(" ")
                                    )
                                } else {
                                    command.executable.to_string_lossy().to_string()
                                };

                                tracing::info!(
                                    "{name}: executing post-start command - {stringified}"
                                );
                                let mut command =
                                    command.to_silent_command(environment_thread.clone());
                                let _ = command.output();
                            }

                            for command in exec_stop_thread.iter().flatten() {
                                let stringified = if let Some(args) = &command.arguments {
                                    format!(
                                        "{} {}",
                                        command.executable.to_string_lossy(),
                                        args.join(" ")
                                    )
                                } else {
                                    command.executable.to_string_lossy().to_string()
                                };

                                tracing::info!("{name}: executing cleanup command - {stringified}");
                                let mut command =
                                    command.to_silent_command(environment_thread.clone());
                                let _ = command.output();
                            }
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
            }
            // forking also blocks the main thread
            ServiceKind::Forking => match thread_child.wait() {
                Ok(exit_status) => {
                    if exit_status.success() {
                        tracing::info!(
                            "{name}: forking unit terminated with successful exit code {}",
                            exit_status.code().unwrap()
                        );
                    } else {
                        tracing::warn!(
                            "{name}: forking unit terminated with failure exit code {}",
                            exit_status.code().unwrap()
                        );
                    }
                }
                Err(error) => {
                    tracing::error!("{name}: {error}");
                }
            },
        }

        Ok(state_child)
    }

    pub fn healthcheck(
        &self,
        child: Arc<SharedChild>,
        running: Arc<Mutex<HashMap<String, ProcessState>>>,
        failed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
        terminated: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    ) -> Result<(), ProcessManagerError> {
        let mut passed = false;
        let name = self.unit.name.clone();

        // we only want to healthcheck long-running services
        if !matches!(
            self.service.kind,
            ServiceKind::Simple | ServiceKind::Forking
        ) {
            return Ok(());
        }

        // we don't want to run redundant healthchecks
        if running.lock().contains_key(&name) {
            tracing::info!("{name}: passed healthcheck");
            return Ok(());
        }

        let mut forked_pid = None;

        match &self.service.healthcheck {
            Some(Healthcheck::Command(healthcheck)) => {
                let seconds = healthcheck.delay_sec;

                let stringified = if let Some(args) = &healthcheck.arguments {
                    format!(
                        "{} {}",
                        healthcheck.executable.to_string_lossy(),
                        args.join(" ")
                    )
                } else {
                    healthcheck.executable.to_string_lossy().to_string()
                };

                tracing::info!("{name}: running command healthcheck - {stringified} ({seconds}s)");
                std::thread::sleep(Duration::from_secs(healthcheck.delay_sec));

                let mut command = healthcheck.to_silent_command(self.service.environment.clone());

                let mut status = command.spawn()?.wait()?;
                let mut max_attempts = healthcheck.retry_limit.unwrap_or(5);

                while !status.success() && max_attempts > 0 {
                    tracing::warn!("{name}: failed healthcheck command, retrying in {seconds}s");
                    std::thread::sleep(Duration::from_secs(seconds));
                    status = command.spawn()?.wait()?;
                    max_attempts -= 1;
                }

                if max_attempts > 0 {
                    passed = true;
                }
            }
            Some(Healthcheck::Process(healthcheck)) => {
                let seconds = healthcheck.delay_sec;

                match &healthcheck.target {
                    None => {
                        let child_pid = child.id();
                        tracing::info!(
                            "{name}: running pid {child_pid} liveness healthcheck ({seconds}s)"
                        );
                        std::thread::sleep(Duration::from_secs(healthcheck.delay_sec));
                        let mut system = System::new_all();
                        let pid = Pid::from_u32(child_pid);
                        system.refresh_processes_specifics(
                            ProcessesToUpdate::Some(&[pid]),
                            true,
                            ProcessRefreshKind::everything(),
                        );

                        if system.process(pid).is_some() {
                            passed = true;
                        }
                    }
                    Some(target) => {
                        tracing::info!("{name}: running process liveness healthcheck ({seconds}s)");
                        std::thread::sleep(Duration::from_secs(healthcheck.delay_sec));
                        let mut system = System::new_all();
                        system.refresh_processes_specifics(
                            ProcessesToUpdate::All,
                            true,
                            ProcessRefreshKind::everything(),
                        );

                        let proc_name = target.file_name().unwrap_or_default();

                        for p in system.processes_by_name(proc_name) {
                            if forked_pid.is_none() {
                                forked_pid = Some(p.pid().as_u32());
                                passed = true;
                            }
                        }

                        if let Some(pid) = forked_pid {
                            self.monitor_child(Child::Pid(pid), running.clone(), terminated);
                        }
                    }
                }
            }
            None => {}
        }

        if passed {
            tracing::info!("{name}: passed healthcheck");
            running.lock().insert(
                name.clone(),
                match forked_pid {
                    None => ProcessState {
                        child: Child::Shared(child),
                        timestamp: Utc::now(),
                    },
                    Some(pid) => ProcessState {
                        child: Child::Pid(pid),
                        timestamp: Utc::now(),
                    },
                },
            );

            for command in self.service.exec_start_post.iter().flatten() {
                let stringified = if let Some(args) = &command.arguments {
                    format!(
                        "{} {}",
                        command.executable.to_string_lossy(),
                        args.join(" ")
                    )
                } else {
                    command.executable.to_string_lossy().to_string()
                };

                tracing::info!("{name}: executing post-start command - {stringified}");

                let mut command = command.to_silent_command(self.service.environment.clone());
                command.output()?;
            }
        } else {
            tracing::warn!("{name}: failed healthcheck");
            failed.lock().insert(name.clone(), Utc::now());
            return Err(ProcessManagerError::FailedHealthcheck(name.to_string()));
        }

        Ok(())
    }

    pub fn monitor_child(
        &self,
        child: Child,
        running: Arc<Mutex<HashMap<String, ProcessState>>>,
        terminated: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    ) {
        let running_thread = running.clone();
        let terminated_thread = terminated.clone();

        let name = self.unit.name.clone();
        let exec_stop_post = self.service.exec_stop_post.clone();
        let environment = self.service.environment.clone();
        let restart_strategy = self.service.restart;
        let restart_sec = self.service.restart_sec.unwrap_or(1);

        std::thread::spawn(move || {
            match child.wait() {
                Ok(exit_status) => {
                    // Execute cleanup commands
                    for command in exec_stop_post.iter().flatten() {
                        let stringified = if let Some(args) = &command.arguments {
                            format!(
                                "{} {}",
                                command.executable.to_string_lossy(),
                                args.join(" ")
                            )
                        } else {
                            command.executable.to_string_lossy().to_string()
                        };

                        tracing::info!("{name}: executing cleanup command - {stringified}");
                        let mut command = command.to_silent_command(environment.clone());
                        let _ = command.output();
                    }

                    // Handle process termination
                    if running_thread.lock().contains_key(&name) {
                        let should_restart = if exit_status.success() {
                            tracing::warn!(
                                "{name}: process {} terminated with success exit code {}",
                                child.id(),
                                exit_status.code().unwrap()
                            );

                            matches!(restart_strategy, RestartStrategy::Always)
                        } else {
                            tracing::warn!(
                                "{name}: process {} terminated with failure exit code {}",
                                child.id(),
                                exit_status.code().unwrap()
                            );

                            matches!(
                                restart_strategy,
                                RestartStrategy::Always | RestartStrategy::OnFailure
                            )
                        };

                        if should_restart {
                            running_thread.lock().remove(&name);
                            tracing::info!(
                                "{name}: restarting terminated process in {restart_sec}s"
                            );

                            std::thread::sleep(Duration::from_secs(restart_sec));

                            // Send reset and start messages
                            for message in [
                                SocketMessage::Reset(vec![name.to_string()]),
                                SocketMessage::Start(vec![name.to_string()]),
                            ] {
                                if let Err(error) = send_message("wpmd.sock", message) {
                                    tracing::error!("{name}: {error}");
                                }
                            }

                            return;
                        } else {
                            terminated_thread.lock().insert(name.clone(), Utc::now());
                        }
                    }
                }
                Err(error) => {
                    tracing::error!("{name}: {error}");
                }
            }

            running_thread.lock().remove(&name);
        });
    }

    pub fn log_path(&self) -> PathBuf {
        wpm_data_dir()
            .join("logs")
            .join(format!("{}.log", self.unit.name))
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub enum Healthcheck {
    Command(CommandHealthcheck),
    Process(ProcessHealthcheck),
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
/// A service liveness healthcheck based on the successful exit code of a command
#[serde(rename_all = "PascalCase")]
pub struct CommandHealthcheck {
    /// Executable name or absolute path to an executable
    pub executable: PathBuf,
    /// Arguments passed to the executable
    pub arguments: Option<Vec<String>>,
    /// Environment variables for this command
    pub environment: Option<Vec<(String, String)>>,
    /// The number of seconds to delay before checking for liveness
    pub delay_sec: u64,
    /// The maximum number of retries (default: 5)
    pub retry_limit: Option<u8>,
}

impl CommandHealthcheck {
    pub fn resolve_user_profile(&mut self) {
        let home_dir = dirs::home_dir()
            .expect("could not find home dir")
            .to_str()
            .unwrap()
            .to_string();

        let stringified = self.executable.to_string_lossy();
        let stringified = stringified.replace("$USERPROFILE", &home_dir);
        let executable = PathBuf::from(stringified);
        self.executable = executable;

        for arg in self.arguments.iter_mut().flatten() {
            *arg = arg.replace("$USERPROFILE", &home_dir);
        }

        for (_, value) in self.environment.iter_mut().flatten() {
            *value = value.replace("$USERPROFILE", &home_dir);
        }
    }

    pub fn to_silent_command(&self, global_environment: Option<Vec<(String, String)>>) -> Command {
        let mut command = Command::new(&self.executable);
        if let Some(arguments) = &self.arguments {
            command.args(arguments);
        }

        if let Some(environment) = global_environment {
            command.envs(environment);
        }

        if let Some(environment) = &self.environment {
            command.envs(environment.clone());
        }

        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());

        command
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// A process liveness healthcheck either based on an automatic PID or an optional binary
#[serde(rename_all = "PascalCase")]
pub struct ProcessHealthcheck {
    /// An optional binary with which to check process liveness
    pub target: Option<PathBuf>,
    /// The number of seconds to delay before checking for liveness
    pub delay_sec: u64,
}

impl Default for Healthcheck {
    fn default() -> Self {
        Self::Process(ProcessHealthcheck {
            target: None,
            delay_sec: 1,
        })
    }
}

#[derive(Default, Serialize, Deserialize, Copy, Clone, JsonSchema)]
pub enum ServiceKind {
    #[default]
    Simple,
    Oneshot,
    Forking,
}

impl Display for ServiceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceKind::Simple => write!(f, "Simple"),
            ServiceKind::Oneshot => write!(f, "Oneshot"),
            ServiceKind::Forking => write!(f, "Forking"),
        }
    }
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

impl From<&Definition> for Command {
    fn from(value: &Definition) -> Self {
        let file = File::create(value.log_path()).unwrap();

        let stdout = file.try_clone().unwrap();
        let stderr = stdout.try_clone().unwrap();

        let mut command = Command::new(&value.service.exec_start.executable);

        if let Some(environment) = &value.service.environment {
            command.envs(environment.clone());
        }

        if let Some(arguments) = &value.service.exec_start.arguments {
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
