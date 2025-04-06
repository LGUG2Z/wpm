use crate::unit::CommandHealthcheck;
use crate::unit::Definition;
use crate::unit::Executable;
use crate::unit::Healthcheck;
use crate::unit::ProcessHealthcheck;
use crate::unit::RemoteExecutable;
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
                resources: Some(
                    [(
                        String::from("CONFIGURATION_FILE"),
                        Url::from_str("https://gist.githubusercontent.com/LGUG2Z/bbafc51ddde2bd1462151cfcc3f7f489/raw/28e24c4a493166fa866ae24ebc4ed8df7f164bd1/minimal.clj").unwrap()
                    )]
                        .into_iter()
                        .collect()
                ),
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Scoop(ScoopExecutable::Manifest(ScoopManifest {
                            manifest: Url::from_str("https://raw.githubusercontent.com/ScoopInstaller/Extras/8a6d8ff0f3963611ae61fd9f45ff36e3c321c8b5/bucket/kanata.json").unwrap(),
                            package: "kanata".to_string(),
                            version: "1.8.1".to_string(),
                            target: None
                        })),
                        arguments: Some(vec![
                            "-c".to_string(),
                            "{{ Resources.CONFIGURATION_FILE }}".to_string(),
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
                    name: "komorebi-bar".to_string(),
                    description: Some("Status bar for komorebi".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                resources: Some(
                    [(
                        String::from("CONFIGURATION_FILE"),
                        Url::from_str("https://raw.githubusercontent.com/LGUG2Z/komorebi/refs/tags/v0.1.35/docs/komorebi.bar.example.json").unwrap()
                    )]
                        .into_iter()
                        .collect()
                ),
                service: Service {
                    kind: ServiceKind::Simple,
                    environment: Some(vec![(
                        "KOMOREBI_CONFIG_HOME".to_string(),
                        "$USERPROFILE/.config/komorebi".to_string(),
                    )]),
                    exec_start: ServiceCommand {
                        executable: Executable::Scoop(ScoopExecutable::Manifest(ScoopManifest {
                            manifest: Url::from_str("https://raw.githubusercontent.com/ScoopInstaller/Extras/8e21dc2cd902b865d153e64a078d97d3cd0593f7/bucket/komorebi.json").unwrap(),
                            package: "komorebi".to_string(),
                            version: "0.1.35".to_string(),
                            target: Some("komorebi-bar.exe".to_string())
                        })),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "{{ Resources.CONFIGURATION_FILE }}".to_string(),
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
                resources: Some(
                    [(
                        String::from("CONFIGURATION_FILE"),
                        Url::from_str("https://raw.githubusercontent.com/LGUG2Z/komorebi/refs/tags/v0.1.35/docs/komorebi.example.json").unwrap()
                    )]
                        .into_iter()
                        .collect()
                ),
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Scoop(ScoopExecutable::Manifest(ScoopManifest {
                            manifest: Url::from_str("https://raw.githubusercontent.com/ScoopInstaller/Extras/8e21dc2cd902b865d153e64a078d97d3cd0593f7/bucket/komorebi.json").unwrap(),
                            package: "komorebi".to_string(),
                            version: "0.1.35".to_string(),
                            target: Some("komorebi.exe".to_string())
                        })),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "{{ Resources.CONFIGURATION_FILE }}".to_string(),
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
                    exec_start_pre: Some(vec![ServiceCommand {
                        executable: Executable::Local(PathBuf::from("komorebic.exe")),
                        arguments: Some(vec!["fetch-asc".to_string()]),
                        environment: None,
                        environment_file: None,
                    }]),
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
                resources: Some(
                    [(
                        String::from("CONFIGURATION_FILE"),
                        Url::from_str("https://raw.githubusercontent.com/LGUG2Z/komorebi/refs/tags/v0.1.35/docs/whkdrc.sample").unwrap()
                    )]
                        .into_iter()
                        .collect()
                ),
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Scoop(ScoopExecutable::Manifest(ScoopManifest {
                            manifest: Url::from_str("https://raw.githubusercontent.com/ScoopInstaller/Extras/112fd691392878f8c4e9e9703dde3d1d182941e3/bucket/whkd.json").unwrap(),
                            package: "whkd".to_string(),
                            version: "0.2.7".to_string(),
                            target: None,
                        })),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "{{ Resources.CONFIGURATION_FILE }}".to_string(),
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
                resources: Some(
                    [(
                        String::from("CONFIGURATION_FILE"),
                        Url::from_str("https://raw.githubusercontent.com/petoncle/mousemaster/refs/tags/73/configuration/neo-mousekeys-ijkl.properties").unwrap()
                    )]
                        .into_iter()
                        .collect()
                ),
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Remote(RemoteExecutable {
                            url: Url::from_str("https://github.com/petoncle/mousemaster/releases/download/73/mousemaster.exe").unwrap(),
                            hash: "7b696461e128aec9cc50d187d8656123a6e7a4e6b1d9ec1dbe504ad2de3cad25".to_string()
                        }),
                        arguments: Some(vec![
                            "--configuration-file={{ Resources.CONFIGURATION_FILE }}".to_string(),
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
                resources: Some(
                    [(
                        String::from("CONFIGURATION_FILE"),
                        Url::from_str("https://raw.githubusercontent.com/LGUG2Z/komokana/refs/tags/v0.1.5/komokana.example.yaml").unwrap()
                    )]
                        .into_iter()
                        .collect()
                ),
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: Executable::Scoop(ScoopExecutable::Manifest(ScoopManifest {
                            manifest: Url::from_str("https://raw.githubusercontent.com/ScoopInstaller/Extras/e633292b4e1101273caac59ffcb4a7ce7ee7a2e8/bucket/komokana.json").unwrap(),
                            package: "komokana".to_string(),
                            version: "0.1.5".to_string(),
                            target: None,
                        })),
                        arguments: Some(vec![
                            "--kanata-port".to_string(),
                            "9999".to_string(),
                            "--configuration".to_string(),
                            "{{ Resources.CONFIGURATION_FILE }}".to_string(),
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
                    autostart: false,
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
                resources: None,
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
                    autostart: false,
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
