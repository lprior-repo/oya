//! Node icon definitions and parsing.
//!
//! Implements Scott Wlaschin DDD principles:
//! - Parse, don't validate
//! - Make illegal states unrepresentable
//! - Types act as documentation

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ===========================================================================
// Node Icon Enum
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeIcon {
    Globe,
    Kafka,
    Clock,
    Play,
    Code,
    Call,
    Box,
    GitBranch,
    GitMerge,
    Send,
    Database,
    Save,
    Trash,
    Repeat,
    Layers,
    Undo,
    Moon,
    AlertTriangle,
    Target,
    Radio,
    CheckCircle,
    Bell,
    HelpCircle,
    Shield,
    Download,
    Upload,
    Eraser,
    Eye,
    Timer,
    Alarm,
    ClockSend,
    Sparkles,
    PlayCircle,
    ArrowRight,
    Workflow,
}

impl NodeIcon {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Globe => "globe",
            Self::Kafka => "kafka",
            Self::Clock => "clock",
            Self::Play => "play",
            Self::Code => "code",
            Self::Call => "call",
            Self::Box => "box",
            Self::GitBranch => "git-branch",
            Self::GitMerge => "git-merge",
            Self::Send => "send",
            Self::Database => "database",
            Self::Save => "save",
            Self::Trash => "trash",
            Self::Repeat => "repeat",
            Self::Layers => "layers",
            Self::Undo => "undo",
            Self::Moon => "moon",
            Self::AlertTriangle => "alert-triangle",
            Self::Target => "target",
            Self::Radio => "radio",
            Self::CheckCircle => "check-circle",
            Self::Bell => "bell",
            Self::HelpCircle => "help-circle",
            Self::Shield => "shield",
            Self::Download => "download",
            Self::Upload => "upload",
            Self::Eraser => "eraser",
            Self::Eye => "eye",
            Self::Timer => "timer",
            Self::Alarm => "alarm",
            Self::ClockSend => "clock-send",
            Self::Sparkles => "sparkles",
            Self::PlayCircle => "play-circle",
            Self::ArrowRight => "arrow-right",
            Self::Workflow => "workflow",
        }
    }
}

impl fmt::Display for NodeIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for NodeIcon {
    type Err = UnknownIconError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "globe" => Ok(Self::Globe),
            "kafka" => Ok(Self::Kafka),
            "clock" => Ok(Self::Clock),
            "play" => Ok(Self::Play),
            "code" => Ok(Self::Code),
            "call" => Ok(Self::Call),
            "box" => Ok(Self::Box),
            "git-branch" => Ok(Self::GitBranch),
            "git-merge" | "git-fork" => Ok(Self::GitMerge),
            "send" => Ok(Self::Send),
            "database" => Ok(Self::Database),
            "save" => Ok(Self::Save),
            "trash" => Ok(Self::Trash),
            "repeat" => Ok(Self::Repeat),
            "layers" => Ok(Self::Layers),
            "undo" => Ok(Self::Undo),
            "moon" => Ok(Self::Moon),
            "alert-triangle" => Ok(Self::AlertTriangle),
            "target" => Ok(Self::Target),
            "radio" => Ok(Self::Radio),
            "check-circle" => Ok(Self::CheckCircle),
            "bell" => Ok(Self::Bell),
            "help-circle" => Ok(Self::HelpCircle),
            "shield" => Ok(Self::Shield),
            "download" => Ok(Self::Download),
            "upload" => Ok(Self::Upload),
            "eraser" => Ok(Self::Eraser),
            "eye" => Ok(Self::Eye),
            "timer" => Ok(Self::Timer),
            "alarm" => Ok(Self::Alarm),
            "clock-send" => Ok(Self::ClockSend),
            "sparkles" => Ok(Self::Sparkles),
            "play-circle" => Ok(Self::PlayCircle),
            "arrow-right" => Ok(Self::ArrowRight),
            "workflow" => Ok(Self::Workflow),
            _ => Err(UnknownIconError(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownIconError(pub String);

impl fmt::Display for UnknownIconError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown icon: {}", self.0)
    }
}

impl std::error::Error for UnknownIconError {}
