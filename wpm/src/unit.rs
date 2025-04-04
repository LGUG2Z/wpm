use crate::communication::send_message;
use crate::process_manager::Child;
use crate::process_manager::ProcessManagerError;
use crate::process_manager::ProcessState;
use crate::reqwest_client;
use crate::resource_regex;
use crate::wpm_log_dir;
use crate::wpm_store_dir;
use crate::SocketMessage;
use chrono::DateTime;
use chrono::Utc;
use dirs::home_dir;
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
use url::Url;

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
/// A wpm definition
#[serde(rename_all = "PascalCase")]
pub struct Definition {
    /// JSON Schema definition for auto completions
    #[serde(rename(serialize = "$schema"))]
    pub schema: Option<String>,
    /// Information about this definition and its dependencies
    pub unit: Unit,
    /// Remote resources used by this definition
    pub resources: Option<HashMap<String, Url>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_start_pre: Option<Vec<ServiceCommand>>,
    /// Command executed by this service definition
    pub exec_start: ServiceCommand,
    /// Commands executed after ExecStart in this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_start_post: Option<Vec<ServiceCommand>>,
    /// Shutdown commands for this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_stop: Option<Vec<ServiceCommand>>,
    /// Post-shutdown cleanup commands for this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_stop_post: Option<Vec<ServiceCommand>>,
    /// Environment variables inherited by all commands in this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<(String, String)>>,
    /// Path to an environment file, containing environment variables inherited by all commands in this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_file: Option<PathBuf>,
    /// Working directory for this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<PathBuf>,
    #[serde(default)]
    /// Healthcheck for this service definition
    #[serde(skip_serializing_if = "Option::is_none")]
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
    /// Executable (local file, remote file, or Scoop package)
    pub executable: Executable,
    /// Arguments passed to the executable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,
    /// Environment variables for this command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<(String, String)>>,
    /// Path to an environment file, containing environment variables for this command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_file: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
pub enum Executable {
    /// A remote executable file verified using a SHA256 hash
    Remote(RemoteExecutable),
    /// A local executable file
    Local(PathBuf),
    /// An executable file with a Scoop package dependency
    Scoop(ScoopExecutable),
}
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct RemoteExecutable {
    /// Url to a remote executable
    pub url: Url,
    /// Sha256 hash of the remote executable at
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
pub enum ScoopExecutable {
    // TODO: this will depend on a hosted service accessible through the individual
    // commercial use license which can identify a manifest revision in a scoop bucket's
    // git history using a combination of bucket + package + version
    Package(ScoopPackage),
    /// A Scoop package identified using a raw manifest
    Manifest(ScoopManifest),
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub enum ScoopBucket {
    Main,
    Extras,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ScoopPackage {
    /// Bucket that the package is found in
    bucket: ScoopBucket,
    /// Name of the package
    package: String,
    /// Version of the package
    version: String,
    /// Target executable in the package
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(rename_all = "PascalCase")]
pub struct ScoopManifest {
    /// Name of the package
    pub package: String,
    /// Version of the package
    pub version: String,
    /// Url to a Scoop manifest
    pub manifest: Url,
    /// Target executable in the package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

impl Display for Executable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.pathbuf().unwrap().to_string_lossy())
    }
}

impl TryInto<PathBuf> for Executable {
    type Error = ProcessManagerError;

    fn try_into(self) -> Result<PathBuf, Self::Error> {
        self.pathbuf()
    }
}

impl Executable {
    pub fn pathbuf(&self) -> Result<PathBuf, ProcessManagerError> {
        match self {
            Executable::Local(local) => Ok(local.clone()),
            Executable::Remote(remote) => {
                let cached_executable_path = self.cached_executable_path()?;
                if cached_executable_path.is_file() {
                    tracing::debug!(
                        "using cached executable {}",
                        cached_executable_path.display()
                    );
                    Ok(cached_executable_path)
                } else {
                    tracing::info!("downloading and caching executable from {}", remote.url);
                    self.download_remote_executable()?;
                    Ok(cached_executable_path)
                }
            }
            Executable::Scoop(scoop) => match scoop {
                ScoopExecutable::Package(_) => todo!(),
                ScoopExecutable::Manifest(manifest) => {
                    let cached_executable_path = self.cached_executable_path()?;
                    if cached_executable_path.is_file() {
                        tracing::debug!(
                            "using scoop executable {}",
                            cached_executable_path.display()
                        );

                        Ok(cached_executable_path)
                    } else {
                        tracing::info!(
                            "installing scoop manifest {}",
                            manifest.manifest.to_string()
                        );
                        self.download_remote_executable()?;
                        Ok(cached_executable_path)
                    }
                }
            },
        }
    }

    pub fn cached_executable_path(&self) -> Result<PathBuf, ProcessManagerError> {
        match self {
            Executable::Local(executable) => Ok(executable.clone()),
            Executable::Remote(remote) => {
                let stringified = remote.url.to_string();
                let filename = stringified
                    .split('/')
                    .next_back()
                    .map(String::from)
                    .unwrap_or_default();

                let mut stringified_parent = stringified.clone();
                stringified_parent = stringified_parent
                    .trim_start_matches("https://")
                    .to_string();
                stringified_parent = stringified_parent.trim_end_matches(&filename).to_string();
                stringified_parent = stringified_parent.replace("/", "_").to_string();
                stringified_parent = stringified_parent.trim_end_matches("_").to_string();

                let cache_parent_dir = wpm_store_dir().join(&stringified_parent);
                std::fs::create_dir_all(&cache_parent_dir)?;
                Ok(cache_parent_dir.join(filename).clone())
            }
            Executable::Scoop(scoop) => match scoop {
                ScoopExecutable::Package(_) => todo!(),
                ScoopExecutable::Manifest(manifest) => Ok(home_dir()
                    .unwrap()
                    .join("scoop")
                    .join("apps")
                    .join(&manifest.package)
                    .join(&manifest.version)
                    .join(
                        manifest
                            .target
                            .clone()
                            .unwrap_or_else(|| format!("{}.exe", manifest.package)),
                    )),
            },
        }
    }

    pub fn download_remote_executable(&self) -> Result<(), ProcessManagerError> {
        match self {
            Executable::Local(_) => {}
            Executable::Remote(remote) => {
                if let Ok(path) = self.cached_executable_path() {
                    let bytes = reqwest_client()
                        .get(remote.url.to_string())
                        .send()?
                        .bytes()?;

                    let digest = sha256::digest(&*bytes);

                    if digest == remote.hash {
                        std::fs::write(&path, bytes)?;
                        tracing::info!("downloaded remote executable to {}", path.display());
                    } else {
                        tracing::error!(
                            "remote executable hash mismatch for {} (expected {digest}, actual {})",
                            remote.url,
                            remote.hash
                        );
                        return Err(ProcessManagerError::HashMismatch {
                            actual: digest,
                            expected: remote.hash.clone(),
                        });
                    }
                }
            }
            Executable::Scoop(scoop) => match scoop {
                ScoopExecutable::Package(_) => {}
                ScoopExecutable::Manifest(manifest) => {
                    let scoop = home_dir()
                        .unwrap()
                        .join("scoop")
                        .join("shims")
                        .join("scoop.cmd");

                    let output = Command::new(scoop)
                        .arg("install")
                        .arg(manifest.manifest.to_string())
                        .output()?;

                    println!("{}", String::from_utf8_lossy(&output.stdout));

                    if !output.status.success() {
                        tracing::error!(
                            "failed to install scoop manifest {}",
                            manifest.manifest.to_string()
                        );
                    }
                }
            },
        }

        Ok(())
    }
}

impl ServiceCommand {
    pub fn resolve_user_profile(&mut self) {
        let home_dir = dirs::home_dir()
            .expect("could not find home dir")
            .to_str()
            .unwrap()
            .to_string();

        if matches!(self.executable, Executable::Local(_)) {
            let stringified = self.executable.to_string();
            let stringified = stringified.replace("$USERPROFILE", &home_dir);
            let executable = PathBuf::from(stringified);
            self.executable = Executable::Local(executable);
        }

        for arg in self.arguments.iter_mut().flatten() {
            *arg = arg.replace("$USERPROFILE", &home_dir);
        }

        if let Some(environment_file) = &self.environment_file {
            let stringified = environment_file.to_string_lossy();
            let stringified = stringified.replace("$USERPROFILE", &home_dir);
            let environment_file = PathBuf::from(stringified);

            if let Ok(environment) =
                serde_envfile::from_file::<serde_envfile::Value>(&environment_file)
            {
                for (k, v) in environment.iter() {
                    match &mut self.environment {
                        None => self.environment = Some(vec![(k.clone(), v.clone())]),
                        Some(e) => {
                            e.push((k.clone(), v.clone()));
                        }
                    }
                }
            }
        }

        for (_, value) in self.environment.iter_mut().flatten() {
            *value = value.replace("$USERPROFILE", &home_dir);
        }
    }

    pub fn to_silent_command(&self, global_environment: Option<Vec<(String, String)>>) -> Command {
        let mut command = Command::new(self.executable.pathbuf().unwrap());
        if let Some(arguments) = &self.arguments {
            command.args(arguments);
        }

        let mut environment_variables = vec![];

        if let Some(environment) = global_environment {
            environment_variables.extend(environment.clone());
        }

        if let Some(environment) = &self.environment {
            environment_variables.extend(environment.clone());
        }

        if !environment_variables.is_empty() {
            command.envs(environment_variables);
        }

        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());

        command
    }
}

fn replace_interpolations(input: &str, resources: &HashMap<String, PathBuf>) -> String {
    let mut output = input.to_string();

    let re = resource_regex();

    output = re
        .replace_all(&output, |caps: &regex::Captures| {
            let identifier = &caps[1];

            resources
                .get(identifier)
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_else(|| caps[0].to_string()) // If not found, leave the original string
        })
        .into_owned();
    output
}

impl Definition {
    pub fn resolve_resources(&mut self) -> Result<(), ProcessManagerError> {
        if let Some(resources) = &self.resources {
            let mut resource_map = HashMap::new();
            'resources: for (identifier, url) in resources {
                match store_ref_for_url(url) {
                    Err(error) => {
                        tracing::error!("{error}");
                        continue 'resources;
                    }
                    Ok(store_ref) => {
                        if !store_ref.is_file() {
                            tracing::info!(
                                "{}: adding resource {} to store",
                                self.unit.name,
                                store_ref.display()
                            );
                            match reqwest_client().get(url.to_string()).send() {
                                Err(error) => {
                                    tracing::error!("{error}");
                                    continue 'resources;
                                }
                                Ok(response) => {
                                    std::fs::write(&store_ref, response.bytes()?)?;
                                }
                            }
                        } else {
                            tracing::debug!(
                                "{}: found resource {} in store",
                                self.unit.name,
                                store_ref.display()
                            )
                        }

                        resource_map.insert(identifier.clone(), store_ref);
                    }
                }
            }

            for (_, v) in self.service.environment.iter_mut().flatten() {
                *v = replace_interpolations(v, &resource_map);
            }

            for arg in self.service.exec_start.arguments.iter_mut().flatten() {
                *arg = replace_interpolations(arg, &resource_map);
            }

            for (_, v) in self.service.exec_start.environment.iter_mut().flatten() {
                *v = replace_interpolations(v, &resource_map);
            }

            for exec in self.service.exec_stop.iter_mut().flatten() {
                for arg in exec.arguments.iter_mut().flatten() {
                    *arg = replace_interpolations(arg, &resource_map);
                }
            }

            for exec in self.service.exec_stop_post.iter_mut().flatten() {
                for arg in exec.arguments.iter_mut().flatten() {
                    *arg = replace_interpolations(arg, &resource_map);
                }

                for (_, v) in exec.environment.iter_mut().flatten() {
                    *v = replace_interpolations(v, &resource_map);
                }
            }

            for exec in self.service.exec_start_post.iter_mut().flatten() {
                for arg in exec.arguments.iter_mut().flatten() {
                    *arg = replace_interpolations(arg, &resource_map);
                }

                for (_, v) in exec.environment.iter_mut().flatten() {
                    *v = replace_interpolations(v, &resource_map);
                }
            }

            for exec in self.service.exec_start_pre.iter_mut().flatten() {
                for arg in exec.arguments.iter_mut().flatten() {
                    *arg = replace_interpolations(arg, &resource_map);
                }

                for (_, v) in exec.environment.iter_mut().flatten() {
                    *v = replace_interpolations(v, &resource_map);
                }
            }
        }

        Ok(())
    }

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
                format!("{} {}", command.executable, args.join(" "))
            } else {
                command.executable.to_string()
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
                                    format!("{} {}", command.executable, args.join(" "))
                                } else {
                                    command.executable.to_string()
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
                                    format!("{} {}", command.executable, args.join(" "))
                                } else {
                                    command.executable.to_string()
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
                    format!("{} {}", command.executable, args.join(" "))
                } else {
                    command.executable.to_string()
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
                            format!("{} {}", command.executable, args.join(" "))
                        } else {
                            command.executable.to_string()
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
        wpm_log_dir().join(format!("{}.log", self.unit.name))
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<String>>,
    /// Environment variables for this command
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<(String, String)>>,
    /// The number of seconds to delay before checking for liveness
    pub delay_sec: u64,
    /// The maximum number of retries (default: 5)
    #[serde(skip_serializing_if = "Option::is_none")]
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

        let mut environment_variables = vec![];

        if let Some(environment) = global_environment {
            environment_variables.extend(environment.clone());
        }

        if let Some(environment) = &self.environment {
            environment_variables.extend(environment.clone());
        }

        if !environment_variables.is_empty() {
            command.envs(environment_variables);
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
    #[serde(skip_serializing_if = "Option::is_none")]
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

        let mut command = Command::new(value.service.exec_start.executable.pathbuf().unwrap());

        let mut environment_variables = vec![];

        if let Some(environment) = &value.service.environment {
            environment_variables.extend(environment.clone());
        }

        if let Some(environment) = &value.service.exec_start.environment {
            environment_variables.extend(environment.clone());
        }

        if !environment_variables.is_empty() {
            command.envs(environment_variables);
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

fn store_ref_for_url(url: &Url) -> Result<PathBuf, ProcessManagerError> {
    let stringified = url.to_string();
    let filename = stringified
        .split('/')
        .next_back()
        .map(String::from)
        .unwrap_or_default();

    let mut stringified_parent = stringified.clone();
    stringified_parent = stringified_parent
        .trim_start_matches("https://")
        .to_string();
    stringified_parent = stringified_parent.trim_end_matches(&filename).to_string();
    stringified_parent = stringified_parent.replace("/", "_").to_string();
    stringified_parent = stringified_parent.trim_end_matches("_").to_string();

    let cache_parent_dir = wpm_store_dir().join(&stringified_parent);
    std::fs::create_dir_all(&cache_parent_dir)?;

    Ok(cache_parent_dir.join(filename))
}
