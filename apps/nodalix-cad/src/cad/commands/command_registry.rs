use super::parser::{parse_command_line, resolve_command_name};
use crate::cad::document::CADDocument;
use crate::cad::history::HistoryManager;
use crate::cad::selection::SelectionManager;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedCommand {
    ActivateTool(&'static str),
    Undo,
    Redo,
    DeleteSelection,
    Save,
    Open,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandOutcome {
    Handled,
    ActivateTool(&'static str),
    Message(String),
    NotHandled,
}

pub struct CommandRegistry;

impl CommandRegistry {
    /// True when the first token is a bare delegated command (no coordinate arguments).
    pub fn is_registry_delegated(input: &str) -> bool {
        let parts = parse_command_line(input);
        let Some(name) = parts.first() else {
            return false;
        };
        let canonical = resolve_command_name(name);
        match canonical.as_str() {
            "DELETE" => true,
            "UNDO" | "REDO" => parts.len() == 1,
            "LINE" | "PLINE" | "RECTANGLE" | "CIRCLE" | "MOVE" => parts.len() == 1,
            _ => false,
        }
    }

    pub fn parse(input: &str) -> ParsedCommand {
        let parts = parse_command_line(input);
        let Some(name) = parts.first() else {
            return ParsedCommand::Unknown(String::new());
        };
        let canonical = resolve_command_name(name);
        match canonical.as_str() {
            "LINE" | "PLINE" | "RECTANGLE" | "CIRCLE" | "MOVE" | "SELECT" | "PAN" => {
                ParsedCommand::ActivateTool(tool_id_for_command(&canonical))
            }
            "DELETE" => ParsedCommand::DeleteSelection,
            "UNDO" => ParsedCommand::Undo,
            "REDO" => ParsedCommand::Redo,
            "SAVE" => ParsedCommand::Save,
            "OPEN" => ParsedCommand::Open,
            _ => ParsedCommand::Unknown(canonical),
        }
    }

    /// Core-side dispatch. UI-specific actions (save dialog, tool pointer routing) return
    /// `CommandOutcome::ActivateTool` or `NotHandled` for the legacy layer to complete.
    pub fn dispatch(
        parsed: &ParsedCommand,
        document: &mut CADDocument,
        selection: &mut SelectionManager,
        history: &mut HistoryManager,
    ) -> CommandOutcome {
        match parsed {
            ParsedCommand::Undo => {
                if history.undo(document) {
                    CommandOutcome::Message("Undo".to_string())
                } else {
                    CommandOutcome::Message("Nothing to undo".to_string())
                }
            }
            ParsedCommand::Redo => {
                if history.redo(document) {
                    CommandOutcome::Message("Redo".to_string())
                } else {
                    CommandOutcome::Message("Nothing to redo".to_string())
                }
            }
            ParsedCommand::DeleteSelection => {
                let ids = selection.selected_ids();
                if ids.is_empty() {
                    return CommandOutcome::Message("No selection".to_string());
                }
                for id in ids {
                    document.remove_entity(&id);
                }
                selection.clear();
                CommandOutcome::Message("Deleted selection".to_string())
            }
            ParsedCommand::ActivateTool(tool_id) => CommandOutcome::ActivateTool(tool_id),
            ParsedCommand::Save | ParsedCommand::Open => CommandOutcome::NotHandled,
            ParsedCommand::Unknown(name) => {
                CommandOutcome::Message(format!("Unknown command: {name}"))
            }
        }
    }
}

fn tool_id_for_command(command: &str) -> &'static str {
    match command {
        "LINE" => "line",
        "PLINE" => "polyline",
        "RECTANGLE" => "rectangle",
        "CIRCLE" => "circle",
        "MOVE" => "move",
        "SELECT" => "select",
        "PAN" => "pan",
        _ => "select",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_delete_alias() {
        assert_eq!(CommandRegistry::parse("e"), ParsedCommand::DeleteSelection);
    }

    #[test]
    fn parse_line_and_rectangle_aliases() {
        assert_eq!(
            CommandRegistry::parse("l"),
            ParsedCommand::ActivateTool("line")
        );
        assert_eq!(
            CommandRegistry::parse("rec"),
            ParsedCommand::ActivateTool("rectangle")
        );
        assert_eq!(CommandRegistry::parse("u"), ParsedCommand::Undo);
    }

    #[test]
    fn delegated_only_for_bare_commands() {
        assert!(CommandRegistry::is_registry_delegated("line"));
        assert!(!CommandRegistry::is_registry_delegated("line 0,0 10,10"));
        assert!(CommandRegistry::is_registry_delegated("delete"));
        assert!(CommandRegistry::is_registry_delegated("undo"));
        assert!(CommandRegistry::is_registry_delegated("redo"));
    }

    #[test]
    fn parse_undo_redo_aliases() {
        assert_eq!(CommandRegistry::parse("u"), ParsedCommand::Undo);
        assert_eq!(CommandRegistry::parse("redo"), ParsedCommand::Redo);
    }

    #[test]
    fn dispatch_delete_selection() {
        use crate::cad::entities::{BaseEntity, CADEntity, LineEntity};
        use crate::cad::geometry::Point2;
        use crate::cad::history::HistoryManager;

        let mut document = CADDocument::new_empty();
        document.add_entity(CADEntity::Line(LineEntity::new(
            BaseEntity::new("entity-1", "layer-default"),
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
        )));
        let mut selection = SelectionManager::new();
        selection.select_one("entity-1");
        let mut history = HistoryManager::new();
        let parsed = ParsedCommand::DeleteSelection;
        let outcome =
            CommandRegistry::dispatch(&parsed, &mut document, &mut selection, &mut history);
        assert_eq!(
            outcome,
            CommandOutcome::Message("Deleted selection".to_string())
        );
        assert!(document.model_space.entities.is_empty());
    }
}
