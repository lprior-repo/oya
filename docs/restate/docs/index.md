# Welcome to Restate!

Source: https://docs.restate.dev/index

Build innately resilient backends and AI agents

Restate is a lightweight runtime to turn AI agents, workflows, and backend services into durable processes. Focus on your logic, not failure mechanics.
Write normal code and let Restate handles resilience and consistency automatically.

## Key capabilities

**Durable execution**: Code automatically stores completed steps and resumes from where it left off when recovering from failures.

**Built-in state**: Maintain state beyond workflow executions and share it between functions with strong consistency guarantees.

**Reliable communication**: Call services sync or async with guaranteed execution and exactly-once semantics.

**Time-based coordination**: Sleep, schedule, and wait for external events with durable timers.

**Workflows**: Coordinate long-running processes, human approvals, listen to webhooks and other signals.

## Common use cases

<Columns>
  <Card title="AI agents" href="/use-cases/ai-agents">
    Manage stateful AI agents with reliable tool usage and long-running conversations.
  </Card>

  <Card title="Workflows" href="/use-cases/workflows">
    Build approval processes, multi-step operations, and business workflows that survive failures.
  </Card>

  <Card title="Microservice orchestration" href="/use-cases/microservice-orchestration">
    Coordinate calls across multiple services with automatic retries and failure handling.
  </Card>

  <Card title="Event processing" href="/use-cases/event-processing">
    Process events with exactly-once guarantees and automatic retry handling.
  </Card>
</Columns>

## First time here?

<Columns>
  <Card title="Quickstart" href="/quickstart" icon={"rocket"}>
    Build your first Restate service in minutes.
  </Card>

  <Card title="Concepts" href="/foundations/key-concepts" icon={"cube"}>
    Understand the core building blocks.
  </Card>

  <Card title="Tour of Restate" icon={"graduation-cap"}>
    Learn how to build common applications with Restate:

    [AI agents](/tour/vercel-ai-agents) •
    [Workflows](/tour/workflows) •
    [Microservice orchestration](/tour/microservice-orchestration)
  </Card>

  <Card title="Ask AI" href="?assistant=open" icon={"robot"}>
    Chat with our AI assistant to get answers to your questions about Restate.
  </Card>
</Columns>

## Learning resources

<CardGroup>
  <Card title="Examples" icon="code-branch" href="https://github.com/restatedev/examples">
    A collection of examples that illustrate how to use Restate to solve common application challenges.
  </Card>

  <Card title="AI Recipes" icon="robot" href="/ai">
    Learn how to build durable AI applications with Restate: agents, chatbots, multi-agent systems, ...
  </Card>

  <Card title="Guides" icon="book" href="/guides">
    Learn how to do common tasks with Restate: patterns, integrations, deployment tutorials, ...
  </Card>
</CardGroup>

## Reference

<CardGroup>
  <Card title="SDKs" icon="code">
    Implement Restate applications in one of the available SDKs.

    <div>
      [TypeScript](/develop/ts/services) •
      [Java](/develop/java/services) •
      [Kotlin](/develop/java/services) •
      [Python](/develop/python/services) •
      [Go](/develop/go/services) •
      [Rust](https://docs.rs/restate-sdk/latest/restate_sdk/)
    </div>
  </Card>

  <Card title="Service Lifecycle" icon="box">
    Deploy and operate services on your preferred platform.

    [Deploy](/deploy/services/kubernetes) •
    [Invoke](/services/invocation/http) •
    [Versioning](/services/versioning) •
    [Monitor & Inspect](/services/introspection)
  </Card>

  <Card title="Host Restate" icon="cloud" href="/cloud/getting-started">
    Get started immediately with Restate Cloud, or host your own Restate server.
  </Card>
</CardGroup>

## Community

<CardGroup>
  <Card title="Need help?" icon="circle-info">
    Join the Restate Discord or Slack communities.

    <br />

    <div>
      [<Icon icon="discord" />](https://discord.restate.dev)

      [<Icon icon="slack" />](https://slack.restate.dev)
    </div>
  </Card>

  <Card title="Stay up to date" icon="calendar" href="https://lu.ma/restatedev">
    Follow us on Twitter, LinkedIn, Bluesky.

    <br />

    <div>
      [<Icon icon="twitter" />](https://twitter.com/restatedev)

      [<Icon icon="linkedin" />](https://www.linkedin.com/company/restatedev)

      [<Icon icon="bluesky" />](https://bsky.app/profile/restate.dev)
    </div>
  </Card>
</CardGroup>

<CardGroup>
  <Card title="YouTube Channel" href="https://www.youtube.com/@restatedev">
    <Frame>
      <iframe />
    </Frame>

    Watch intro videos, community meetings and talks about Restate.
  </Card>

  <Card title="Events" href="https://lu.ma/restatedev">
    <Frame>
      <iframe />
    </Frame>

    Subscribe and attend our events.
  </Card>
</CardGroup>