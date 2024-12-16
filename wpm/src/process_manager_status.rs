use crate::process_manager::ProcessManagerError;
use crate::unit::Definition;
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
            Some((definition, status)) => match status.state {
                UnitState::Running => {
                    let log_path = definition.log_path();

                    let mut output = vec![
                        format!("● Status of {name}:"),
                        format!("  Kind: {}", definition.service.kind),
                        format!("  State: Running since {}", status.timestamp.to_string()),
                        format!("  PID: {}", status.pid),
                        format!("  Log file: {}", log_path.to_string_lossy()),
                    ];

                    if let Some(args) = &definition.service.arguments {
                        let arguments = args.join(" ");
                        let arguments = arguments.replace("/", "\\");
                        output.push(format!(
                            "  Command: {} {arguments}",
                            definition.service.executable.to_string_lossy()
                        ));
                    } else {
                        output.push(format!(
                            "  Command: {}",
                            definition.service.executable.to_string_lossy()
                        ));
                    }

                    if let Some(environment) = &definition.service.environment {
                        let environment = environment
                            .iter()
                            .map(|(a, b)| format!("{a}={b}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let environment = environment.replace("/", "\\");
                        output.push(format!("  Environment: {environment}",));
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
                UnitState::Stopped => {
                    let log_path = definition.log_path();

                    let mut output = vec![
                        format!("● Status of {name}:"),
                        format!("  Kind: {}", definition.service.kind),
                        "  State: Stopped".to_string(),
                        format!("  Log file: {}", log_path.to_string_lossy()),
                    ];

                    if let Some(args) = &definition.service.arguments {
                        let arguments = args.join(" ");
                        let arguments = arguments.replace("/", "\\");
                        output.push(format!(
                            "  Command: {} {arguments}",
                            definition.service.executable.to_string_lossy()
                        ));
                    } else {
                        output.push(format!(
                            "  Command: {}",
                            definition.service.executable.to_string_lossy()
                        ));
                    }

                    if let Some(environment) = &definition.service.environment {
                        let environment = environment
                            .iter()
                            .map(|(a, b)| format!("{a}={b}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let environment = environment.replace("/", "\\");
                        output.push(format!("  Environment: {environment}",));
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
                UnitState::Completed => {
                    let log_path = definition.log_path();

                    let mut output = vec![
                        format!("● Status of {name}:"),
                        format!("  Kind: {}", definition.service.kind),
                        format!("  State: Completed at {}", status.timestamp),
                        format!("  Log file: {}", log_path.to_string_lossy()),
                    ];

                    if let Some(args) = &definition.service.arguments {
                        let arguments = args.join(" ");
                        let arguments = arguments.replace("/", "\\");
                        output.push(format!(
                            "  Command: {} {arguments}",
                            definition.service.executable.to_string_lossy()
                        ));
                    } else {
                        output.push(format!(
                            "  Command: {}",
                            definition.service.executable.to_string_lossy()
                        ));
                    }

                    if let Some(environment) = &definition.service.environment {
                        let environment = environment
                            .iter()
                            .map(|(a, b)| format!("{a}={b}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let environment = environment.replace("/", "\\");
                        output.push(format!("  Environment: {environment}",));
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
                UnitState::Failed => {
                    let log_path = definition.log_path();

                    let mut output = vec![
                        format!("● Status of {name}:"),
                        format!("  Kind: {}", definition.service.kind),
                        format!("  State: Failed at {}", status.timestamp),
                        format!("  Log file: {}", log_path.to_string_lossy()),
                    ];

                    if let Some(args) = &definition.service.arguments {
                        let arguments = args.join(" ");
                        let arguments = arguments.replace("/", "\\");
                        output.push(format!(
                            "  Command: {} {arguments}",
                            definition.service.executable.to_string_lossy()
                        ));
                    } else {
                        output.push(format!(
                            "  Command: {}",
                            definition.service.executable.to_string_lossy()
                        ));
                    }

                    if let Some(environment) = &definition.service.environment {
                        let environment = environment
                            .iter()
                            .map(|(a, b)| format!("{a}={b}"))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let environment = environment.replace("/", "\\");
                        output.push(format!("  Environment: {environment}",));
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
            },
        }
    }
}
