use crate::process_manager::ProcessManagerError;
use chrono::DateTime;
use chrono::Utc;
use parking_lot::Mutex;
use schemars::schema_for;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use shared_child::SharedChild;
use std::collections::HashMap;
use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::Path;
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
    pub kind: ServiceType,
    /// Autostart this definition with wpmd
    #[serde(default)]
    pub autostart: bool,
    /// Executable name or absolute path to an executable
    pub executable: PathBuf,
    /// Arguments passed to the executable
    pub arguments: Option<Vec<String>>,
    /// Environment variables for this service definition
    pub environment: Option<Vec<(String, String)>>,
    #[serde(default)]
    /// Healthcheck for this service definition
    pub healthcheck: Healthcheck,
    /// Shutdown commands for this definition
    pub shutdown: Option<Vec<String>>,
}

impl Definition {
    pub fn execute(
        &self,
        running: Arc<Mutex<HashMap<String, Arc<SharedChild>>>>,
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

        if matches!(kind, ServiceType::Oneshot) {
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
        running: Arc<Mutex<HashMap<String, Arc<SharedChild>>>>,
        failed: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
    ) -> Result<(), ProcessManagerError> {
        let mut passed = false;
        let name = self.unit.name.clone();

        // we only want to healthcheck long-running services
        if !matches!(self.service.kind, ServiceType::Simple) {
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
            running.lock().insert(name.clone(), child);
        } else {
            tracing::warn!("{name}: failed healthcheck");
            failed.lock().insert(name.clone(), Utc::now());
            return Err(ProcessManagerError::FailedUnit(name.to_string()));
        }

        Ok(())
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
pub enum ServiceType {
    #[default]
    Simple,
    Oneshot,
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

impl From<&Definition> for Command {
    fn from(value: &Definition) -> Self {
        let home = dirs::home_dir().expect("could not find home dir");
        let dir = home.join(".config").join("wpm").join("logs");

        if !dir.is_dir() {
            std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm/logs");
        }

        let file = File::create(dir.join(format!("{}.log", value.unit.name))).unwrap();

        let stdout = file.try_clone().unwrap();
        let stderr = stdout.try_clone().unwrap();

        let mut command = Command::new(&value.service.executable);

        if let Some(environment) = &value.service.environment {
            command.envs(environment.clone());
        }

        if let Some(arguments) = &value.service.arguments {
            command.args(arguments);
        }

        command.creation_flags(CREATE_NO_WINDOW);
        command.stdout(stdout);
        command.stderr(stderr);
        command
    }
}

impl Definition {
    pub fn schemagen() -> String {
        let schema = schema_for!(Self);
        serde_json::to_string_pretty(&schema).unwrap()
    }

    pub fn examplegen() {
        let examples = vec![
            Self {
                unit: Unit {
                    name: "kanata".to_string(),
                    description: Some("software keyboard remapper".to_string()),
                    requires: None,
                },
                service: Service {
                    kind: ServiceType::Simple,
                    executable: PathBuf::from("kanata.exe"),
                    arguments: Some(vec![
                        "-c".to_string(),
                        "$USERPROFILE/minimal.kbd".to_string(),
                        "--port".to_string(),
                        "9999".to_string(),
                    ]),
                    environment: None,
                    healthcheck: Healthcheck::default(),
                    shutdown: None,
                    autostart: false,
                },
            },
            Self {
                unit: Unit {
                    name: "masir".to_string(),
                    description: Some("focus follows mouse for Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: ServiceType::Simple,
                    executable: PathBuf::from("masir.exe"),
                    arguments: None,
                    environment: None,
                    healthcheck: Healthcheck::default(),
                    shutdown: None,
                    autostart: false,
                },
            },
            Self {
                unit: Unit {
                    name: "komorebi".to_string(),
                    description: Some("tiling window management for Windows".to_string()),
                    requires: Some(vec!["whkd".to_string(), "kanata".to_string()]),
                },
                service: Service {
                    kind: ServiceType::Simple,
                    executable: PathBuf::from("komorebi.exe"),
                    arguments: Some(vec![
                        "--config".to_string(),
                        "$USERPROFILE/.config/komorebi/komorebi.json".to_string(),
                    ]),
                    environment: Some(vec![(
                        "KOMOREBI_CONFIG_HOME".to_string(),
                        "$USERPROFILE/.config/komorebi".to_string(),
                    )]),
                    healthcheck: Healthcheck::Command("komorebic state".to_string()),
                    shutdown: Some(vec![
                        "komorebic stop".to_string(),
                        "komorebic restore-windows".to_string(),
                    ]),
                    autostart: false,
                },
            },
            Self {
                unit: Unit {
                    name: "whkd".to_string(),
                    description: Some("simple hotkey daemon for Windows".to_string()),
                    requires: None,
                },
                service: Service {
                    kind: ServiceType::Simple,
                    executable: PathBuf::from("whkd.exe"),
                    arguments: None,
                    environment: None,
                    healthcheck: Healthcheck::default(),
                    shutdown: None,
                    autostart: false,
                },
            },
            Self {
                unit: Unit {
                    name: "desktop".to_string(),
                    description: Some("everything I need to work on Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: ServiceType::Oneshot,
                    executable: PathBuf::from("msg.exe"),
                    arguments: Some(vec![
                        "*".to_string(),
                        "Desktop recipe completed!".to_string(),
                    ]),
                    environment: None,
                    healthcheck: Healthcheck::default(),
                    shutdown: None,
                    autostart: true,
                },
            },
        ];

        for example in examples {
            std::fs::write(
                Path::new("examples").join(format!("{}.toml", example.unit.name)),
                toml::to_string_pretty(&example).unwrap(),
            )
            .unwrap();
        }
    }
}
