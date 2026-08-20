#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleCommand {
    DebugOn,
    DebugOff,
    DebugStatus,
    Help,
}

pub fn parse(input: &str) -> Result<ConsoleCommand, String> {
    match input.trim().to_ascii_lowercase().as_str() {
        "debug on" => Ok(ConsoleCommand::DebugOn),
        "debug off" => Ok(ConsoleCommand::DebugOff),
        "debug" | "debug status" => Ok(ConsoleCommand::DebugStatus),
        "help" => Ok(ConsoleCommand::Help),
        _ => Err(format!("Unknown command: {}", input.trim())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_debug_commands() {
        assert_eq!(parse("debug on"), Ok(ConsoleCommand::DebugOn));
        assert!(parse("drop database").is_err());
    }
}
