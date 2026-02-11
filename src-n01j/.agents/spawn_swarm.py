#!/usr/bin/env python3
"""
Spawn 12 parallel agents for bead processing swarm.
This script generates the Task tool calls needed to launch all agents.
"""

import os

PROMPT_TEMPLATE_PATH = "/home/lewis/src/oya/.agents/agent_prompt.md"

def generate_agent_tasks():
    """Generate Task tool invocations for all 12 agents."""

    with open(PROMPT_TEMPLATE_PATH, 'r') as f:
        prompt_template = f.read()

    print("# Spawn 12-Agent Swarm")
    print("# Copy-paste these Task tool calls into Claude Code:\n")

    for i in range(1, 13):
        prompt = prompt_template.replace("{N}", str(i))

        print(f"# === Agent {i} ===")
        print(f'Task(')
        print(f'    description="Agent {i} process bead through pipeline",')
        print(f'    prompt="""{prompt}""",')
        print(f'    subagent_type="general-purpose",')
        print(f'    run_in_background=True,')
        print(f'    max_turns=50')
        print(f')')
        print()

if __name__ == "__main__":
    generate_agent_tasks()
