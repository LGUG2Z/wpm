use crate::process_manager::ProcessManagerError;
use crate::unit::Definition;
use crate::unit::Healthcheck;
use crate::unit_status::UnitState;
use crate::unit_status::UnitStatus;
use tabled::Table;

pub struct ProcessManagerStatus(pub Vec<(Definition, UnitStatus)>);

impl ProcessManagerStatus {
    pub fn as_table(&self) -> String {
        Table::new(self.0.iter().map(|(_, status)| status).collect::<Vec<_>>()).to_string()
    }

    pub fn unit_status(&self, name: &str) -> Result<String, ProcessManagerError> {
        match self.0.iter().find(|(def, _status)| def.unit.name == name) {
            None => Ok(format!("Unregistered unit: {name}")),
            Some((definition, status)) => {
                let log_path = definition.log_path();
                let mut output = Vec::new();

                match status.state {
                    UnitState::Running => {
                        output.append(&mut vec![
                            format!("● Status of {name}:"),
                            format!("  Kind: {}", definition.service.kind),
                            format!("  State: Running since {}", status.timestamp.to_string()),
                            format!("  PID: {}", status.pid),
                            format!("  Log: {}", log_path.to_string_lossy()),
                        ]);
                    }
                    UnitState::Stopped => {
                        output.append(&mut vec![
                            format!("● Status of {name}:"),
                            format!("  Kind: {}", definition.service.kind),
                            "  State: Stopped".to_string(),
                            format!("  Log file: {}", log_path.to_string_lossy()),
                        ]);
                    }
                    UnitState::Completed => {
                        output.append(&mut vec![
                            format!("● Status of {name}:"),
                            format!("  Kind: {}", definition.service.kind),
                            format!("  State: Completed at {}", status.timestamp),
                            format!("  Log file: {}", log_path.to_string_lossy()),
                        ]);
                    }
                    UnitState::Failed => {
                        output.append(&mut vec![
                            format!("● Status of {name}:"),
                            format!("  Kind: {}", definition.service.kind),
                            format!("  State: Failed at {}", status.timestamp),
                            format!("  Log file: {}", log_path.to_string_lossy()),
                        ]);
                    }
                    UnitState::Terminated => {
                        output.append(&mut vec![
                            format!("● Status of {name}:"),
                            format!("  Kind: {}", definition.service.kind),
                            format!("  State: Terminated at {}", status.timestamp),
                            format!("  Log file: {}", log_path.to_string_lossy()),
                        ]);
                    }
                }

                if let Some(args) = &definition.service.exec_start.arguments {
                    let arguments = args.join(" ");
                    let arguments = arguments.replace("/", "\\");
                    output.push(format!(
                        "  ExecStart: {} {arguments}",
                        definition
                            .service
                            .exec_start
                            .executable
                            .to_string()
                            .replace("/", "\\")
                    ));
                } else {
                    output.push(format!(
                        "  ExecStart: {}",
                        definition
                            .service
                            .exec_start
                            .executable
                            .to_string()
                            .replace("/", "\\")
                    ));
                }

                match &definition.service.healthcheck {
                    Some(Healthcheck::Command(command)) => {
                        if let Some(args) = &command.arguments {
                            let arguments = args.join(" ");
                            let arguments = arguments.replace("/", "\\");
                            output.push(format!(
                                "  Healthcheck: {} {arguments}",
                                command.executable.to_string_lossy()
                            ));
                        } else {
                            output.push(format!(
                                "  Healthcheck: {}",
                                command.executable.to_string_lossy()
                            ));
                        }
                    }
                    Some(Healthcheck::Process(healthcheck)) => {
                        let seconds = healthcheck.delay_sec;
                        match &healthcheck.target {
                            None => {
                                output.push(format!(
                                    "  Healthcheck: Liveness check after {seconds}s",
                                ));
                            }
                            Some(target) => {
                                output.push(format!(
                                    "  Healthcheck: Liveness check for {} after {seconds}s",
                                    target.display()
                                ));
                            }
                        }
                    }
                    None => {}
                }

                if let Some(shutdowns) = &definition.service.exec_stop {
                    output.push("  ExecStop:".to_string());
                    for command in shutdowns {
                        if let Some(args) = &command.arguments {
                            let arguments = args.join(" ");
                            let arguments = arguments.replace("/", "\\");
                            output.push(format!("    {} {arguments}", command.executable));
                        } else {
                            output.push(format!("    {}", command.executable));
                        }
                    }
                }

                if let Some(shutdowns) = &definition.service.exec_stop_post {
                    output.push("  ExecStopPost:".to_string());
                    for command in shutdowns {
                        if let Some(args) = &command.arguments {
                            let arguments = args.join(" ");
                            let arguments = arguments.replace("/", "\\");
                            output.push(format!("    {} {arguments}", command.executable,));
                        } else {
                            output.push(format!("    {}", command.executable));
                        }
                    }
                }

                if let Some(environment) = &definition.service.environment {
                    let vars = environment
                        .iter()
                        .map(|(a, b)| format!("{a}={b}"))
                        .collect::<Vec<_>>();

                    output.push("  Environment (Service):".to_string());
                    for var in vars {
                        let var = var.replace("/", "\\");
                        output.push(format!("    {var}"));
                    }
                }

                if let Some(environment) = &definition.service.exec_start.environment {
                    let vars = environment
                        .iter()
                        .map(|(a, b)| format!("{a}={b}"))
                        .collect::<Vec<_>>();

                    output.push("  Environment (ExecStart):".to_string());
                    for var in vars {
                        let var = var.replace("/", "\\");
                        output.push(format!("    {var}"));
                    }
                }

                if let Some(requires) = &definition.unit.requires {
                    let requires = requires.join(" ");
                    output.push(format!("  Requires: {requires}",));
                }

                let log_contents = std::fs::read_to_string(log_path)?;
                let lines = log_contents
                    .lines()
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>();
                let last_ten_lines = lines.iter().rev().take(10).rev().collect::<Vec<_>>();

                if !last_ten_lines.is_empty() {
                    output.push("\nRecent logs:".to_string());
                    for line in last_ten_lines {
                        output.push(format!("  {line}"));
                    }
                }

                Ok(output.join("\n"))
            }
        }
    }
}
