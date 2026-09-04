use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum MessagingSystemCommand {
    Info,
    Help,
    Rotate,
    Switch { selection: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagingCommandParse {
    NotCommand,
    InvalidSwitch,
    Command(MessagingSystemCommand),
}

pub fn parse_messaging_command(input: &str) -> MessagingCommandParse {
    let command = input.trim().to_ascii_lowercase();
    match command.as_str() {
        "/info" => MessagingCommandParse::Command(MessagingSystemCommand::Info),
        "/help" => MessagingCommandParse::Command(MessagingSystemCommand::Help),
        "/new" | "/clear" => MessagingCommandParse::Command(MessagingSystemCommand::Rotate),
        _ if command.starts_with("/switch") => {
            let suffix = command["/switch".len()..].trim();
            match suffix
                .parse::<usize>()
                .ok()
                .filter(|selection| *selection > 0)
            {
                Some(selection) => {
                    MessagingCommandParse::Command(MessagingSystemCommand::Switch { selection })
                }
                None => MessagingCommandParse::InvalidSwitch,
            }
        }
        _ => MessagingCommandParse::NotCommand,
    }
}

pub fn sort_scope_ids(scope_ids: &mut [String]) {
    scope_ids.sort();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_is_case_insensitive_and_accepts_compact_switch() {
        assert_eq!(
            parse_messaging_command(" /SWITCH2 "),
            MessagingCommandParse::Command(MessagingSystemCommand::Switch { selection: 2 })
        );
        assert_eq!(
            parse_messaging_command("/switch 0"),
            MessagingCommandParse::InvalidSwitch
        );
        assert_eq!(
            parse_messaging_command("hello"),
            MessagingCommandParse::NotCommand
        );
    }
}
