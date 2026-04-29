#[cfg(test)]
use super::NodeCategory;

#[cfg(test)]
#[allow(clippy::too_many_lines)]
pub(super) fn node_metadata(node_type: &str) -> (NodeCategory, String, String) {
    match node_type {
        "http-handler" => (NodeCategory::Entry, "HTTP Handler".to_owned(), "globe".to_owned()),
        "kafka-handler" => (NodeCategory::Entry, "Kafka Consumer".to_owned(), "kafka".to_owned()),
        "cron-trigger" => (NodeCategory::Entry, "Cron Trigger".to_owned(), "clock".to_owned()),
        "workflow-submit" => {
            (NodeCategory::Entry, "Workflow Submit".to_owned(), "play-circle".to_owned())
        }
        "run" => (NodeCategory::Durable, "Durable Step".to_owned(), "shield".to_owned()),
        "service-call" => {
            (NodeCategory::Durable, "Service Call".to_owned(), "arrow-right".to_owned())
        }
        "object-call" => (NodeCategory::Durable, "Object Call".to_owned(), "box".to_owned()),
        "workflow-call" => {
            (NodeCategory::Durable, "Workflow Call".to_owned(), "workflow".to_owned())
        }
        "send-message" => (NodeCategory::Durable, "Send Message".to_owned(), "send".to_owned()),
        "delayed-send" => {
            (NodeCategory::Durable, "Delayed Message".to_owned(), "clock-send".to_owned())
        }
        "get-state" => (NodeCategory::State, "Get State".to_owned(), "download".to_owned()),
        "set-state" => (NodeCategory::State, "Set State".to_owned(), "upload".to_owned()),
        "clear-state" => (NodeCategory::State, "Clear State".to_owned(), "eraser".to_owned()),
        "clear-all" => (NodeCategory::State, "Clear All".to_owned(), "trash".to_owned()),
        "condition" => (NodeCategory::Flow, "If / Else".to_owned(), "git-branch".to_owned()),
        "switch" => (NodeCategory::Flow, "Switch".to_owned(), "git-fork".to_owned()),
        "loop" => (NodeCategory::Flow, "Loop / Iterate".to_owned(), "repeat".to_owned()),
        "parallel" => (NodeCategory::Flow, "Parallel".to_owned(), "layers".to_owned()),
        "compensate" => (NodeCategory::Flow, "Compensate".to_owned(), "undo".to_owned()),
        "sleep" => (NodeCategory::Timing, "Sleep / Timer".to_owned(), "timer".to_owned()),
        "timeout" => (NodeCategory::Timing, "Timeout".to_owned(), "alarm".to_owned()),
        "durable-promise" => {
            (NodeCategory::Signal, "Durable Promise".to_owned(), "sparkles".to_owned())
        }
        "awakeable" => (NodeCategory::Signal, "Awakeable".to_owned(), "bell".to_owned()),
        "resolve-promise" => {
            (NodeCategory::Signal, "Resolve Promise".to_owned(), "check-circle".to_owned())
        }
        "signal-handler" => (NodeCategory::Signal, "Signal Handler".to_owned(), "radio".to_owned()),
        _ => (NodeCategory::Durable, "Unknown Node".to_owned(), "help-circle".to_owned()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::node_metadata;
    use crate::graph::NodeCategory;

    #[test]
    fn given_known_node_type_when_fetching_metadata_then_expected_values_are_returned() {
        let (category, label, icon) = node_metadata("http-handler");

        assert_eq!(category, NodeCategory::Entry);
        assert_eq!(label, "HTTP Handler");
        assert_eq!(icon, "globe");
    }

    #[test]
    fn given_unknown_node_type_when_fetching_metadata_then_defaults_are_returned() {
        let (category, label, icon) = node_metadata("totally-unknown");

        assert_eq!(category, NodeCategory::Durable);
        assert_eq!(label, "Unknown Node");
        assert_eq!(icon, "help-circle");
    }
}
