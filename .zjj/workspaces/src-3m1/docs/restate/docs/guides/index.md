# Guides

Source: https://docs.restate.dev/guides/index

Learn how to do common tasks with Restate.

## Recipes

<CardGroup>
  <Card title="Cron Jobs" href="/guides/cron">
    Schedule tasks periodically with Restate
  </Card>

  <Card title="Durable Webhooks" href="/guides/durable-webhooks">
    Process webhook events from external services with exactly-once delivery guarantees.
  </Card>

  <Card title="Parallelizing Work" href="/guides/parallelizing-work">
    Execute a list of tasks in parallel and then gather their result.
  </Card>

  <Card title="Rate Limiting" href="/guides/rate-limiting">
    Control request rates and prevent service overload with Restate
  </Card>

  <Card title="Sagas" href="/guides/sagas">
    Implementing undo operations in case of failures, to keep your system consistent
  </Card>
</CardGroup>

## Development Guides

<CardGroup>
  <Card title="Databases and Restate" href="/guides/databases">
    Learn when and how to use databases in combination with Restate.
  </Card>

  <Card title="Error Handling" href="/guides/error-handling">
    Learn how to handle transient and terminal errors in your applications.
  </Card>

  <Card title="Request Lifecycle" href="/guides/request-lifecycle">
    Deep dive into the lifecycle of a request in Restate
  </Card>
</CardGroup>

## Deployment Guides

<CardGroup>
  <Card title="Connecting Kubernetes Services to Restate Cloud" href="/guides/connecting-k8s-services-to-cloud">
    Learn how to connect services on Kubernetes to Restate Cloud.
  </Card>

  <Card title="Kubernetes deployments with Helm" href="/guides/restate-on-kind-with-helm">
    Learn how to deploy a Restate cluster using Helm on a kind Kubernetes cluster.
  </Card>

  <Card title="Kubernetes deployments with Restate Operator" href="/guides/restate-on-kind-with-operator">
    Learn how to deploy single-node Restate with Restate operator on a kind Kubernetes cluster.
  </Card>

  <Card title="Local Restate Cluster with Docker" href="/guides/cluster">
    Learn how to deploy a Restate cluster using Docker Compose.
  </Card>

  <Card title="Scaling to Multi-Node Deployments" href="/guides/local-to-replicated">
    Migrate a single node to a multi-node cluster.
  </Card>
</CardGroup>

## Integrations

<CardGroup>
  <Card title="Restate-Kafka Quickstart" href="/guides/kafka-quickstart">
    Learn how to connect your Restate service to a Kafka topic.
  </Card>

  <Card title="XState" href="/guides/xstate">
    Integrate Restate with XState to implement durable state machines.
  </Card>
</CardGroup>