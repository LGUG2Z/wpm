use crate::unit::Definition;
use crate::unit::Healthcheck;
use crate::unit::RestartStrategy;
use crate::unit::Service;
use crate::unit::ServiceCommand;
use crate::unit::ServiceKind;
use crate::unit::Unit;
use schemars::schema_for;
use std::path::Path;
use std::path::PathBuf;

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
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: PathBuf::from("kanata.exe"),
                        arguments: Some(vec![
                            "-c".to_string(),
                            "$USERPROFILE/minimal.kbd".to_string(),
                            "--port".to_string(),
                            "9999".to_string(),
                        ]),
                        environment: None,
                    },
                    environment: None,
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
                unit: Unit {
                    name: "masir".to_string(),
                    description: Some("focus follows mouse for Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: PathBuf::from("masir.exe"),
                        arguments: None,
                        environment: None,
                    },
                    environment: None,
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
                unit: Unit {
                    name: "komorebi-bar".to_string(),
                    description: Some("status bar for komorebi".to_string()),
                    requires: Some(vec!["komorebi".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    environment: Some(vec![(
                        "KOMOREBI_CONFIG_HOME".to_string(),
                        "$USERPROFILE/.config/komorebi".to_string(),
                    )]),
                    exec_start: ServiceCommand {
                        executable: PathBuf::from("komorebi-bar.exe"),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "$USERPROFILE/.config/komorebi/komorebi.bar.json".to_string(),
                        ]),
                        environment: None,
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
                },
            },
            Self {
                unit: Unit {
                    name: "komorebi".to_string(),
                    description: Some("tiling window management for Windows".to_string()),
                    requires: Some(vec!["whkd".to_string(), "kanata".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: PathBuf::from("komorebi.exe"),
                        arguments: Some(vec![
                            "--config".to_string(),
                            "$USERPROFILE/.config/komorebi/komorebi.json".to_string(),
                        ]),
                        environment: Some(vec![(
                            "KOMOREBI_CONFIG_HOME".to_string(),
                            "$USERPROFILE/.config/komorebi".to_string(),
                        )]),
                    },
                    environment: None,
                    working_directory: None,
                    healthcheck: Some(Healthcheck::Command(ServiceCommand {
                        executable: PathBuf::from("komorebic.exe"),
                        arguments: Some(vec!["state".to_string()]),
                        environment: None,
                    })),
                    restart: Default::default(),
                    restart_sec: None,
                    exec_stop: Some(vec![ServiceCommand {
                        executable: PathBuf::from("komorebic.exe"),
                        arguments: Some(vec!["stop".to_string()]),
                        environment: None,
                    }]),
                    exec_stop_post: Some(vec![ServiceCommand {
                        executable: PathBuf::from("komorebic.exe"),
                        arguments: Some(vec!["restore-windows".to_string()]),
                        environment: None,
                    }]),
                    autostart: false,
                    exec_start_pre: None,
                    exec_start_post: None,
                },
            },
            Self {
                unit: Unit {
                    name: "whkd".to_string(),
                    description: Some("simple hotkey daemon for Windows".to_string()),
                    requires: None,
                },
                service: Service {
                    kind: ServiceKind::Simple,
                    exec_start: ServiceCommand {
                        executable: PathBuf::from("whkd.exe"),
                        arguments: None,
                        environment: None,
                    },
                    environment: None,
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
                unit: Unit {
                    name: "desktop".to_string(),
                    description: Some("everything I need to work on Windows".to_string()),
                    requires: Some(vec!["komorebi".to_string(), "komorebi-bar".to_string()]),
                },
                service: Service {
                    kind: ServiceKind::Oneshot,
                    exec_start: ServiceCommand {
                        executable: PathBuf::from("msg.exe"),
                        arguments: Some(vec![
                            "*".to_string(),
                            "Desktop recipe completed!".to_string(),
                        ]),
                        environment: None,
                    },
                    environment: None,
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

        for example in examples {
            std::fs::write(
                Path::new("examples").join(format!("{}.toml", example.unit.name)),
                toml::to_string_pretty(&example).unwrap(),
            )
            .unwrap();
        }
    }
}
