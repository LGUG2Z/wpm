use crate::unit::Definition;
use crate::unit::Healthcheck;
use crate::unit::Service;
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
                    kind: ServiceKind::Simple,
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
                    kind: ServiceKind::Simple,
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
                    kind: ServiceKind::Simple,
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
                    kind: ServiceKind::Oneshot,
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
