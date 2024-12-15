use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Serialize, Deserialize)]
pub struct WpmUnit {
    pub unit: Unit,
    pub service: Service,
}

#[derive(Serialize, Deserialize)]
pub struct Unit {
    pub name: String,
    pub description: Option<String>,
    pub requires: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Service {
    pub executable: PathBuf,
    pub arguments: Option<Vec<String>>,
    pub environment: Option<Vec<(String, String)>>,
}

const CREATE_NO_WINDOW: u32 = 0x08000000;

impl From<&WpmUnit> for Command {
    fn from(value: &WpmUnit) -> Self {
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
