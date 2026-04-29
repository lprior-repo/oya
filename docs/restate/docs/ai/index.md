# AI / Agents with Restate

Source: https://docs.restate.dev/ai/index

Learn how to build durable AI agents and integrate popular AI SDKs with Restate.

Restate makes AI agents and workflows innately resilient. Restate provides the reliability infrastructure you need to run AI workloads in production - from simple LLM chains to complex multi-agent systems.

<Card title="restatedev/ai-examples" icon="github" href="https://github.com/restatedev/ai-examples">
  Browse through all AI examples here
</Card>

## Why Restate?

Restate makes building AI workflows and agent easy:

* ✅ Recovery from failures - Never lose agent progress again
* ✅ Built-in session management - Store context in Restate's K/V store
* ✅ Complete observability - Trace every decision and action
* ✅ Composable patterns - From simple agents to complex multi-agent systems
* ✅ Production safety - Approvals, timeouts, rollbacks, and more

Whether you're building chatbots, autonomous agents, or AI-powered workflows, Restate handles the complexity of distributed execution so you can focus on your AI logic.

## LLM & Agent SDK Integrations

Get started quickly by integrating Restate with your favorite LLM and Agent SDKs. Restate works with most AI SDKs that allow you to wrap model calls and tools, enabling durable execution for your AI workloads.

<CardGroup>
  <Card title="Vercel AI SDK" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/vercel.svg?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=2b7c1a2af7665aef39d1e296f1c80e2e" href="/ai/sdk-integrations/vercel-ai-sdk" />

  <Card title="OpenAI Agents SDK" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/openai.webp?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=bd5490a9489b6c4b602e28fe0fa0e6d5" href="/ai/sdk-integrations/openai-agents-sdk" />

  <Card title="LiteLLM" href="/ai/sdk-integrations/litellm" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/lite-llm_icon.webp?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=26d2b56cbaae9765079c2b25b855791e" />

  <Card title="Google ADK" href="/ai/sdk-integrations/google-adk" icon="https://mintcdn.com/restate-6d46e1dc/eyiUDPHMMaoJj2hw/img/ai/sdk-integrations/google-adk.png?fit=max&auto=format&n=eyiUDPHMMaoJj2hw&q=85&s=efb9e6bea73fd103d930af1d22c3234c" />

  <Card title="Integrating with other AI SDKs" href="/ai/sdk-integrations/integration-guide" />
</CardGroup>

## Composable AI Patterns

If you prefer to **own your control flow** and only want to use an LLM SDK for model calls, Restate turns your custom logic into durable, fault-tolerant workflows.

<CardGroup>
  <Card title="Prompt chaining" href="/ai/patterns/prompt-chaining" icon="link">
    Build fault-tolerant processing pipelines with automatic retries and recovery.
  </Card>

  <Card title="Tools and Workflows" href="/ai/patterns/tools" icon="wrench">
    Implement recoverable routing and tool execution with Restate.
  </Card>

  <Card title="Multi-Agent Systems" href="/ai/patterns/multi-agent" icon="users">
    Coordinate multiple AI agents to collaborate on complex tasks and workflows.
  </Card>

  <Card title="Sessions & chat" href="/ai/patterns/sessions-and-chat" icon="comments">
    Build stateful chat sessions with persistent context and concurrency control.
  </Card>

  <Card title="Human-in-the-loop" href="/ai/patterns/human-in-the-loop" icon="user-check">
    Integrate human feedback and approval steps into AI workflows and agent decision-making.
  </Card>

  <Card title="Parallelization" href="/ai/patterns/parallelization" icon="bolt">
    Execute multiple tools and agents concurrently with deterministic recovery.
  </Card>

  <Card title="Competitive racing" href="/ai/patterns/competitive-racing" icon="flag-checkered">
    Start multiple workflows/agents, return the first result, cancel the rest.
  </Card>

  <Card title="Notify when ready" href="/ai/patterns/notify-when-ready" icon="bell">
    Resilient async notifications for late responses of long-running agents.
  </Card>

  <Card title="Roll back on failures" href="/ai/patterns/rollback" icon="arrow-u-turn-up-left">
    Undo completed work when an agent fails to execute the entire workflow.
  </Card>
</CardGroup>

## Other Resources

Blog posts:

* Announcements:
  * [Vercel AI SDK Integration](https://www.restate.dev/blog/building-durable-agents-with-vercel-and-restate)
  * [OpenAI Agents SDK](https://www.restate.dev/blog/durable-orchestration-for-ai-agents-with-restate-and-openai-sdk)
  * [Google ADK](https://www.restate.dev/blog/build-resilient-ai-agents-with-restate-and-google-adk)
* [Durable AI Loops: Fault Tolerance across Frameworks and without Handcuffs](https://www.restate.dev/blog/resilient-serverless-agents)
* [AI Agents should be serverless and durable](https://www.restate.dev/blog/resilient-serverless-agents)
* [A Durable Coding Agent — with Modal and Restate](https://www.restate.dev/blog/durable-coding-agent-with-restate-and-modal)

Webinars:

* [Video: Durable AI Agents with Restate - Community Meeting July 2025](https://www.youtube.com/watch?v=BawfutguT5E)