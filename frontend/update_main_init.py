import re

content = open("src/main.rs").read()

new_init = """        Workflow {
            nodes: vec![
                oya_frontend::graph::Node {
                    id: oya_frontend::graph::NodeId::new(),
                    name: "HTTP Handler".to_string(),
                    description: "POST /SignupWorkflow/{userId}/run".to_string(),
                    node_type: "http-handler".to_string(),
                    category: oya_frontend::graph::NodeCategory::Entry,
                    icon: "globe".to_string(),
                    x: 350.0,
                    y: 40.0,
                    config: serde_json::json!({"configured": true, "status": "completed", "journalIndex": 0}),
                    last_output: None,
                    selected: false,
                    executing: false,
                    skipped: false,
                    error: None,
                },
                oya_frontend::graph::Node {
                    id: oya_frontend::graph::NodeId::new(),
                    name: "Durable Step".to_string(),
                    description: "Create user in database".to_string(),
                    node_type: "run".to_string(),
                    category: oya_frontend::graph::NodeCategory::Durable,
                    icon: "shield".to_string(),
                    x: 350.0,
                    y: 170.0,
                    config: serde_json::json!({"configured": true, "status": "completed", "durableStepName": "create-user", "journalIndex": 1}),
                    last_output: None,
                    selected: false,
                    executing: false,
                    skipped: false,
                    error: None,
                },
                oya_frontend::graph::Node {
                    id: oya_frontend::graph::NodeId::new(),
                    name: "If / Else".to_string(),
                    description: "Check if user creation succeeded".to_string(),
                    node_type: "condition".to_string(),
                    category: oya_frontend::graph::NodeCategory::Flow,
                    icon: "git-branch".to_string(),
                    x: 350.0,
                    y: 300.0,
                    config: serde_json::json!({"configured": true, "status": "completed", "journalIndex": 2}),
                    last_output: None,
                    selected: false,
                    executing: false,
                    skipped: false,
                    error: None,
                }
            ],
            connections: vec![], // we'll omit connections initially to keep it simple, user can connect them
            viewport: oya_frontend::graph::Viewport { x: 0.0, y: 0.0, zoom: 0.85 },
            execution_queue: vec![],
            current_step: 0,
            history: vec![],
        }"""

pattern = r"Workflow::new\(\)"
content = content.replace("Workflow::new()", new_init, 1) # Only replace the first one in the signal init
content = content.replace("workflow_name = use_signal(|| \"API Data Pipeline\".to_string())", "workflow_name = use_signal(|| \"SignupWorkflow\".to_string())")

with open("src/main.rs", "w") as f:
    f.write(content)
