use std::borrow::Cow;
use std::fmt;

use warp_i18n::tr;
use warpui::Action;
use warpui::keymap::{BindingDescription, DescriptionContext};

use crate::WorkspaceAction;
use crate::server::telemetry::AddTabWithShellSource;
use crate::terminal::available_shells::AvailableShell;
use crate::terminal::view::TerminalAction;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NewSessionOptionId(pub(crate) String);
impl NewSessionOptionId {
    #[cfg_attr(not(feature = "local_tty"), allow(dead_code))]
    pub(super) fn new(s: String) -> Self {
        Self(s)
    }
}

#[derive(Debug)]
pub(super) enum Direction {
    Down,
    Right,
    Up,
    Left,
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Direction::Down => "Down",
                Direction::Right => "Right",
                Direction::Up => "Up",
                Direction::Left => "Left",
            }
        )
    }
}

#[derive(Debug)]
pub(super) enum NewSessionConfig {
    NewTab(AvailableShell),
    NewWindow(AvailableShell),
    Split(Direction, AvailableShell),
}

impl NewSessionConfig {
    fn shell(&self) -> &AvailableShell {
        match self {
            NewSessionConfig::NewTab(shell) => shell,
            NewSessionConfig::NewWindow(shell) => shell,
            NewSessionConfig::Split(_, shell) => shell,
        }
    }
}

#[derive(Debug)]
/// An option for creating a new terminal session
///
/// Contains configuration information like:
/// - which shell to use
/// - how to display the option in the command palette
pub struct NewSessionOption {
    id: NewSessionOptionId,
    description: BindingDescription,
    config: NewSessionConfig,
}

impl NewSessionOption {
    pub fn id(&self) -> &NewSessionOptionId {
        &self.id
    }

    /// Returns the description (a.k.a. the top line in the command palette entry)
    pub fn description(&self) -> &str {
        self.description.in_context(DescriptionContext::Default)
    }

    pub(super) fn binding_description(&self) -> &BindingDescription {
        &self.description
    }
}

impl NewSessionOption {
    pub(super) fn new(id: NewSessionOptionId, config: NewSessionConfig) -> Self {
        let (display, english) = match &config {
            NewSessionConfig::NewTab(shell) => (
                format!(
                    "{} {}",
                    tr("search.new_session.tab", "Create New Tab:"),
                    shell.short_name()
                ),
                format!("Create New Tab: {}", shell.short_name()),
            ),
            NewSessionConfig::NewWindow(shell) => (
                format!(
                    "{} {}",
                    tr("search.new_session.window", "Create New Window:"),
                    shell.short_name()
                ),
                format!("Create New Window: {}", shell.short_name()),
            ),
            NewSessionConfig::Split(direction, shell) => {
                let label = match direction {
                    Direction::Down => tr("search.new_session.split_down", "Split Pane Down:"),
                    Direction::Right => tr("search.new_session.split_right", "Split Pane Right:"),
                    Direction::Up => tr("search.new_session.split_up", "Split Pane Up:"),
                    Direction::Left => tr("search.new_session.split_left", "Split Pane Left:"),
                };
                (
                    format!("{label} {}", shell.short_name()),
                    format!("Split Pane {direction}: {}", shell.short_name()),
                )
            }
        };
        let description = BindingDescription::new_preserve_case(display).with_search_alias(english);
        Self {
            id,
            description,
            config,
        }
    }

    /// Returns an action that should be triggered if this entry is accepted
    pub fn action(&self) -> Box<dyn Action> {
        match &self.config {
            NewSessionConfig::NewTab(shell) => Box::new(WorkspaceAction::AddTabWithShell {
                shell: shell.clone(),
                source: AddTabWithShellSource::CommandPalette,
            }),
            NewSessionConfig::NewWindow(shell) => Box::new(WorkspaceAction::AddWindowWithShell {
                shell: shell.clone(),
            }),
            NewSessionConfig::Split(Direction::Down, shell) => {
                Box::new(TerminalAction::SplitDown(Some(shell.clone())))
            }
            NewSessionConfig::Split(Direction::Up, shell) => {
                Box::new(TerminalAction::SplitUp(Some(shell.clone())))
            }
            NewSessionConfig::Split(Direction::Right, shell) => {
                Box::new(TerminalAction::SplitRight(Some(shell.clone())))
            }
            NewSessionConfig::Split(Direction::Left, shell) => {
                Box::new(TerminalAction::SplitLeft(Some(shell.clone())))
            }
        }
    }

    /// Returns the details (a.k.a. the second line in the command palette entry)
    pub fn details(&self) -> Cow<'_, str> {
        self.config.shell().details()
    }
}
