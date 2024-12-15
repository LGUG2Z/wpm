use crate::unit::WpmUnit;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::process::Command;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum ProcessManagerError {
    #[error("{0} is not a registered unit")]
    UnregisteredUnit(String),
    #[error("{0} is already running")]
    AlreadyRunning(String),
    #[error("{0} is not running")]
    NotRunning(String),
    #[error(transparent)]
    Io(#[from] tokio::io::Error),
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error("{0} did not spawn a process with a handle")]
    NoHandle(String),
}

pub struct ProcessManager {
    units: HashMap<String, WpmUnit>,
    running: Arc<Mutex<HashMap<String, u32>>>,
}

impl ProcessManager {
    pub fn init() -> Result<Self, ProcessManagerError> {
        let mut pm = ProcessManager {
            units: Default::default(),
            running: Arc::new(Default::default()),
        };

        pm.load_units()?;
        Ok(pm)
    }

    pub fn load_units(&mut self) -> Result<(), ProcessManagerError> {
        let home = dirs::home_dir().expect("could not find home dir");
        let dir = home.join(".config").join("wpm");

        if !dir.is_dir() {
            std::fs::create_dir_all(&dir).expect("could not create ~/.config/wpm");
        }

        let read_dir = std::fs::read_dir(&dir)?;

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

    pub async fn start(&mut self, unit_name: &str) -> Result<(), ProcessManagerError> {
        let unit = self
            .units
            .get(unit_name)
            .cloned()
            .ok_or(ProcessManagerError::UnregisteredUnit(unit_name.to_string()))?;

        if self.running.lock().await.contains_key(unit_name) {
            return Err(ProcessManagerError::AlreadyRunning(unit_name.to_string()));
        }

        for dependency in unit.unit.requires.iter().flatten() {
            tracing::info!("{unit_name} has a dependency on {dependency}");
            let dependency_unit = self.units.get(dependency).cloned().ok_or(
                ProcessManagerError::UnregisteredUnit(dependency.to_string()),
            )?;

            if !self
                .running
                .lock()
                .await
                .contains_key(&dependency_unit.unit.name)
            {
                let dependency_name = dependency_unit.unit.name.clone();
                Box::pin(self.start(&dependency_name)).await?;

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

                    let mut status = healthcheck_command.spawn()?.wait().await?;

                    while !status.success() {
                        tracing::warn!(
                            "{dependency} failed its healthcheck command, retrying in 2s"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        status = healthcheck_command.spawn()?.wait().await?;
                    }

                    tracing::info!("{dependency} is healthy");
                }
            }
        }

        let mut command = Command::from(&unit);
        let mut child = command.spawn()?;
        let id = child
            .id()
            .ok_or(ProcessManagerError::NoHandle(unit_name.to_string()))?;

        self.running.lock().await.insert(unit_name.to_string(), id);
        tracing::info!("starting unit: {unit_name}");

        let running = self.running.clone();
        let name = unit_name.to_string();

        tokio::spawn(async move {
            match child.wait().await {
                Ok(exit_status) => {
                    if exit_status.success() {
                        tracing::info!("finished unit: {name}");
                    } else {
                        tracing::warn!("killed unit: {name}");
                    }
                }
                Err(error) => {
                    tracing::error!("{error}");
                }
            }

            running.lock().await.remove(&name);
        });

        Ok(())
    }

    pub async fn stop(&mut self, unit_name: &str) -> Result<(), ProcessManagerError> {
        let unit = self
            .units
            .get(unit_name)
            .cloned()
            .ok_or(ProcessManagerError::UnregisteredUnit(unit_name.to_string()))?;

        let running = self.running.lock().await;

        tracing::info!("stopping unit: {unit_name}");

        match unit.service.shutdown {
            None => {
                let child = *running
                    .get(unit_name)
                    .ok_or(ProcessManagerError::NotRunning(unit_name.to_string()))?;

                Command::new("taskkill")
                    .args(["/F", "/PID", &child.to_string()])
                    .output()
                    .await?;
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

                shutdown_command.output().await?;
            }
        }

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), ProcessManagerError> {
        tracing::info!("shutting down process manager");

        let mut units = vec![];
        for unit in self.running.lock().await.keys() {
            units.push(unit.clone());
        }

        for unit in units {
            self.stop(&unit).await?;
        }

        Ok(())
    }
}
