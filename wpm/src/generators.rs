use crate::unit::CommandHealthcheck;
use crate::unit::Definition;
use crate::unit::Executable;
use crate::unit::Healthcheck;
use crate::unit::ProcessHealthcheck;
use crate::unit::RemoteExecutable;
use crate::unit::RemoteUrl;
use crate::unit::RestartStrategy;
use crate::unit::ScoopExecutable;
use crate::unit::ScoopManifest;
use crate::unit::Service;
use crate::unit::ServiceCommand;
use crate::unit::ServiceKind;
use crate::unit::Unit;
use schemars::schema_for;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;

impl Definition {
    pub fn schemagen() -> String {
        let schema = schema_for!(Self);
        serde_json::to_string_pretty(&schema).unwrap()
    }

    pub fn examplegen() {
        let examples = vec![
            Self {
                schema: None,
                unit: Unit {
                    name: "kanata".to_string(),
                    description: Some("Software keyboard remapper".to_string()),
                    requires: None,
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Scoop(ScoopExecutable::Manifest(ScoopManifest {
                            manifest: RemoteUrl(Url::from_str("https://raw.githubusercontent.com/ScoopInstaller/Extras/653cfbfc224e40343a49510b2f47dd30c5ca7790/bucket/kanata.json").unwrap()),
                            package: "kanata".to_string(),
                            version: "1.8.0".to_string(),
                            target: None
                        })),
                        arguments: Some(vec![
                            "-c".to_string(),
                            "$USERPROFILE/minimal.kbd".to_string(),
                            "--port".to_string(),
                            "9999".to_string(),
                        ]),
                        environment: None,
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::default()),
                    restart: Default::default(),
                    restart_sec: None,
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "masir".to_string(),
                    description: Some("Focus follows mouse for Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Local(PathBuf::from("masir.exe")),
                        arguments: None,
                        environment: None,
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::default()),
                    restart: Default::default(),
                    restart_sec: None,
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "komorebi-bar".to_string(),
                    description: Some("Status bar for komorebi".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    environment: Some(vec![(
                        "KOMOREBI_CONFIG_HOME".to_string(),
                        "$USERPROFILE/.config/komorebi".to_string(),
                    )]),
                    exec_start: ServiceCommand {
                        executable: Executable::Local(PathBuf::from("komorebi-bar.exe")),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "$USERPROFILE/.config/komorebi/komorebi.bar.json".to_string(),
                        ]),
                        environment: None,
                        environment_file: None,
                    },
                    working_directory: None,
                    healthcheck: Some(Healthcheck::default()),
                    restart: Default::default(),
                    restart_sec: None,
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                    environment_file: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "komorebi".to_string(),
                    description: Some("Tiling window management for Windows".to_string()),
                    requires: Some(vec!["whkd".to_string(), "kanata".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Local(PathBuf::from("komorebi.exe")),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "$USERPROFILE/.config/komorebi/komorebi.json".to_string(),
                        ]),
                        environment: Some(vec![(
                            "KOMOREBI_CONFIG_HOME".to_string(),
                            "$USERPROFILE/.config/komorebi".to_string(),
                        )]),
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::Command(CommandHealthcheck {
                        executable: PathBuf::from("komorebic.exe"),
                        arguments: Some(vec!["state".to_string()]),
                        environment: None,
                        delay_sec: 1,
                        retry_limit: None,
                    })),
                    restart: Default::default(),
                    restart_sec: None,
                    exec_stop: Some(vec![ServiceCommand {
                        executable: Executable::Local(PathBuf::from("komorebic.exe")),
                        arguments: Some(vec!["stop".to_string()]),
                        environment: None,
                        environment_file: None,
                    }]),
                    exec_stop_post: Some(vec![ServiceCommand {
                        executable: Executable::Local(PathBuf::from("komorebic.exe")),
                        arguments: Some(vec!["restore-windows".to_string()]),
                        environment: None,
                        environment_file: None,
                    }]),
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "whkd".to_string(),
                    description: Some("Simple hotkey daemon for Windows".to_string()),
                    requires: None,
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Local(PathBuf::from("whkd.exe")),
                        arguments: None,
                        environment: None,
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::default()),
                    restart: RestartStrategy::OnFailure,
                    restart_sec: Some(2),
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "mousemaster".to_string(),
                    description: Some("A keyboard driven interface for mouseless mouse manipulation".to_string()),
                    requires: Some(vec!["whkd".to_string(), "kanata".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Remote(RemoteExecutable {
                            url: RemoteUrl(Url::from_str("https://github.com/petoncle/mousemaster/releases/download/69/mousemaster.exe").unwrap()),
                            hash: "fb01d97beaa9b84ce312e5c5fe2976124c5cb4316a10b4541f985566731a36ab".to_string()
                        }),
                        arguments: Some(vec![
                            "--configuration-file=$USERPROFILE/Downloads/mousemaster.properties".to_string(),
                            "--pause-on-error=false".to_string(),
                        ]),
                        environment: None,
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::Process(ProcessHealthcheck {
                        target: None,
                        delay_sec: 2,
                    })),
                    restart: RestartStrategy::OnFailure,
                    restart_sec: Some(2),
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "komokana".to_string(),
                    description: Some("Automatic application-aware keyboard layer switching for Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string(), "kanata".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Local(PathBuf::from("komokana.exe")),
                        arguments: Some(vec![
                            "--kanata-port".to_string(),
                            "9999".to_string(),
                            "--configuration".to_string(),
                            "$USERPROFILE/komokana.yaml".to_string(),
                            "--default-layer".to_string(),
                            "qwerty".to_string()
                        ]),
                        environment: None,
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::default()),
                    restart: RestartStrategy::OnFailure,
                    restart_sec: Some(2),
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: true,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                schema: None,
                unit: Unit {
                    name: "desktop".to_string(),
                    description: Some("Everything I need to work on Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string(), "komorebi-bar".to_string(), "mousemaster".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Oneshot,
                    exec_start: ServiceCommand {
                        executable: Executable::Local(PathBuf::from("msg.exe")),
                        arguments: Some(vec![
                            "*".to_string(),
                            "Desktop recipe completed!".to_string(),
                        ]),
                        environment: None,
                        environment_file: None,
                    },
                    environment: None,
                    environment_file: None,
                    working_directory: None,
                    healthcheck: None,
                    restart: Default::default(),
                    restart_sec: None,
                    exec_stop: None,
                    exec_stop_post: None,
                    autostart: true,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
        ];

        for format in ["json", "toml"] {
            let parent = Path::new("examples").join(format);
            std::fs::create_dir_all(&parent).unwrap();

            for example in &examples {
                match format {
                    "json" => {
                        let mut example = example.clone();
                        example.schema = Some("https://raw.githubusercontent.com/LGUG2Z/wpm/refs/heads/master/schema.unit.json".to_string());

                        std::fs::write(
                            parent.join(format!("{}.json", example.unit.name)),
                            serde_json::to_string_pretty(&example).unwrap(),
                        )
                        .unwrap();
                    }
                    "toml" => {
                        std::fs::write(
                            parent.join(format!("{}.toml", example.unit.name)),
                            toml::to_string_pretty(&example).unwrap(),
                        )
                        .unwrap();
                    }
                    _ => {}
                }
            }
        }
    }
}
