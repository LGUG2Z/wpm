use schemars::schema_for;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fs::File;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct WpmUnit {
    pub unit: Unit,
    pub service: Service,
}

impl WpmUnit {
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
                    kind: Some(ServiceType::Simple),
                    executable: PathBuf::from("kanata.exe"),
                    arguments: Some(vec![
                        "-c".to_string(),
                        "$USERPROFILE/minimal.kbd".to_string(),
                        "--port".to_string(),
                        "9999".to_string(),
                    ]),
                    environment: None,
                    healthcheck: None,
                    shutdown: None,
                },
            },
            Self {
                unit: Unit {
                    name: "masir".to_string(),
                    description: Some("focus follows mouse for Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: Some(ServiceType::Simple),
                    executable: PathBuf::from("masir.exe"),
                    arguments: None,
                    environment: None,
                    healthcheck: None,
                    shutdown: None,
                },
            },
            Self {
                unit: Unit {
                    name: "komorebi".to_string(),
                    description: Some("tiling window management for Windows".to_string()),
                    requires: Some(vec!["whkd".to_string(), "kanata".to_string()]),
                },
                service: Service {
                    kind: Some(ServiceType::Simple),
                    executable: PathBuf::from("komorebi.exe"),
                    arguments: Some(vec![
                        "--config".to_string(),
                        "$USERPROFILE/.config/komorebi/komorebi.json".to_string(),
                    ]),
                    environment: Some(vec![(
                        "KOMOREBI_CONFIG_HOME".to_string(),
                        "$USERPROFILE/.config/komorebi".to_string(),
                    )]),
                    healthcheck: Some("komorebic state".to_string()),
                    shutdown: Some("komorebic stop".to_string()),
                },
            },
            Self {
                unit: Unit {
                    name: "whkd".to_string(),
                    description: Some("simple hotkey daemon for Windows".to_string()),
                    requires: None,
                },
                service: Service {
                    kind: Some(ServiceType::Simple),
                    executable: PathBuf::from("whkd.exe"),
                    arguments: None,
                    environment: None,
                    healthcheck: None,
                    shutdown: None,
                },
            },
            Self {
                unit: Unit {
                    name: "desktop".to_string(),
                    description: Some("everything I need to work on Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: Some(ServiceType::Oneshot),
                    executable: PathBuf::from("msg.exe"),
                    arguments: Some(vec![
                        "*".to_string(),
                        "Desktop recipe completed!".to_string(),
                    ]),
                    environment: None,
                    healthcheck: None,
                    shutdown: None,
                },
            },
        ];

        for example in examples {
            println!("# {}.toml", example.unit.name);
            println!("{}", toml::to_string_pretty(&example).unwrap())
        }
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct Unit {
    pub name: String,
    pub description: Option<String>,
    pub requires: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, JsonSchema)]
pub struct Service {
    #[serde(alias = "type")]
    pub kind: Option<ServiceType>,
    pub executable: PathBuf,
    pub arguments: Option<Vec<String>>,
    pub environment: Option<Vec<(String, String)>>,
    pub healthcheck: Option<String>,
    pub shutdown: Option<String>,
}

#[derive(Default, Serialize, Deserialize, Copy, Clone, JsonSchema)]
pub enum ServiceType {
    #[default]
    Simple,
    Oneshot,
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
