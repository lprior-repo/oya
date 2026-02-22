import re

with open("src/stage_runtime.rs", "r") as f:
    content = f.read()

# Remove use crate::pipeline::MergeQueuePolicy
content = re.sub(r'use crate::pipeline::MergeQueuePolicy;\n?', '', content)

# Remove pub(super) merge_queue_policy: MergeQueuePolicy, from ShipGateRequest
content = re.sub(r'\s*pub\(super\) merge_queue_policy: MergeQueuePolicy,\n', '\n', content)

# Remove request.merge_queue_policy from execute_ship_gate_with_gate_runner call
content = re.sub(r'execute_ship_gate_with_gate_runner\(request.merge_queue_policy, \|gate\| \{', 'execute_ship_gate_with_gate_runner(|gate| {', content)

# Remove merge_queue_policy: MergeQueuePolicy from execute_ship_gate_with_gate_runner def
content = re.sub(r'\s*merge_queue_policy: MergeQueuePolicy,\n\s*run_gate: F', '\n    run_gate: F', content)

# Remove if !merge_queue_policy.should_run(&gate) { ... }
content = re.sub(r'\s*if !merge_queue_policy\.should_run\(&gate\) \{[\s\S]*?continue;\n\s*\}\n', '\n', content)

# Remove merge_queue_policy argument from cue_monitor_failure call
content = re.sub(r'cue_monitor_failure\(\n\s*merge_queue_policy,\n', 'cue_monitor_failure(\n', content)

# Remove merge_queue_policy: MergeQueuePolicy from cue_monitor_failure def
content = re.sub(r'fn cue_monitor_failure\(\n\s*merge_queue_policy: MergeQueuePolicy,\n', 'fn cue_monitor_failure(\n', content)

# Remove merge_queue_policy from should_validate_cue_monitor call
content = re.sub(r'should_validate_cue_monitor\(merge_queue_policy, gate, gate_evidence\)', 'should_validate_cue_monitor(gate, gate_evidence)', content)

# Update should_validate_cue_monitor definition
old_should_validate = """fn should_validate_cue_monitor(
    merge_queue_policy: MergeQueuePolicy,
    gate: &Gate,
    evidence: &GateEvidence,
) -> bool {
    matches!(merge_queue_policy, MergeQueuePolicy::Skip)
        && *gate == Gate::CueArtifactGenerated
        && evidence.command.contains(":cue-check")
}"""
new_should_validate = """fn should_validate_cue_monitor(
    gate: &Gate,
    evidence: &GateEvidence,
) -> bool {
    *gate == Gate::CueArtifactGenerated
        && evidence.command.contains(":cue-check")
}"""
content = content.replace(old_should_validate, new_should_validate)

# Remove MergeQueuePolicy from tests in the same file
content = re.sub(r'MergeQueuePolicy::Skip,\s*\|_gate\|', '|_gate|', content)
content = re.sub(r'MergeQueuePolicy::Skip,\s*\|gate\|', '|gate|', content)

with open("src/stage_runtime.rs", "w") as f:
    f.write(content)
