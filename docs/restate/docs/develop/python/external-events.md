# External Events

Source: https://docs.restate.dev/develop/python/external-events

Handle external events and human-in-the-loop patterns with durable waiting primitives.

Sometimes your handlers need to pause and wait for external processes to complete. This is common in:

* **Human-in-the-loop workflows** (approvals, reviews, manual steps)
* **External system integration** (waiting for webhooks, async APIs)
* **AI agent patterns** (tool execution, human oversight)

This pattern is also known as the **callback** or **task token** pattern.

## Two Approaches

Restate provides two primitives for handling external events:

| Primitive            | Use Case                   | Key Feature                          |
| -------------------- | -------------------------- | ------------------------------------ |
| **Awakeables**       | Services & Virtual Objects | Unique ID-based completion           |
| **Durable Promises** | Workflows only             | Named promises for simpler signaling |

## How it works

Implementing this pattern in a distributed system is tricky, since you need to ensure that the handler can recover from failures and resume waiting for the external event.

Restate promises are durable and distributed. They survive crashes and can be resolved or rejected by any handler in the workflow.

To save costs on FaaS deployments, Restate lets the handler [suspend](/foundations/key-concepts#suspensions-on-faas) while awaiting the promise, and invokes it again when the result is available.

## Awakeables

**Best for:** Services and Virtual Objects where you need to coordinate with external systems.

### Creating and waiting for awakeables

1. **Create an awakeable** - Get a unique ID and promise
2. **Send the ID externally** - Pass the awakeable ID to your external system
3. **Wait for result** - Your handler [suspends](/foundations/key-concepts#suspensions-on-faas) until the external system responds

```py {"CODE_LOAD::python/src/develop/awakeables.py#here"}  theme={null}
id, promise = ctx.awakeable(type_hint=str)

await ctx.run_typed("trigger task", request_human_review, name=name, id=id)

review = await promise
```

<Accordion title="Serialization">
  By default, the SDK serializes the journal entry with the [`json`](https://docs.python.org/3/library/json.html#) library.
  Alternatively, you can specify a [Pydantic model](/develop/python/serialization#pydantic) or [custom serializer](/develop/python/serialization#custom-serialization).
</Accordion>

<Info>
  Note that if you wait for an awakeable in an [exclusive handler](/foundations/handlers#handler-behavior) in a Virtual Object, all other calls to this object will be queued.
</Info>

### Resolving/rejecting Awakeables

External processes complete awakeables in two ways:

* **Resolve** with success data → handler continues normally
* **Reject** with error reason → throws a [terminal error](/develop/python/error-handling) in the waiting handler

#### Via SDK (from other handlers)

**Resolve:**

```python {"CODE_LOAD::python/src/develop/awakeables.py#resolve"}  theme={null}
ctx.resolve_awakeable(name, review)
```

**Reject:**

```python {"CODE_LOAD::python/src/develop/awakeables.py#reject"}  theme={null}
ctx.reject_awakeable(name, "Cannot be reviewed")
```

#### Via HTTP API

External systems can complete awakeables using Restate's HTTP API:

**Resolve with data:**

```shell theme={null}
curl localhost:8080/restate/awakeables/sign_1PePOqp/resolve \
  --json '"Looks good!"'
```

**Reject with error:**

```shell theme={null}
curl localhost:8080/restate/awakeables/sign_1PePOqp/reject \
  -H 'content-type: text/plain' \
  -d 'Review rejected: insufficient documentation'
```

## Durable Promises

**Best for:** Workflows where you need to signal between different workflow handlers.

**Key differences from awakeables:**

* No ID management - use logical names instead
* Scoped to workflow execution lifetime

Use this for:

* Sending data to the run handler
* Have handlers wait for events emitted by the run handler

<Info>
  After a workflow's run handler completes, other handlers can still be called for up to 24 hours (default).
  The results of resolved Durable Promises remain available during this time.
  Update the retention time via the [service configuration](/services/configuration).
</Info>

### Creating and waiting for promises

Wait for a promise by name:

```py {"CODE_LOAD::python/src/develop/durable_promise.py#promise"}  theme={null}
review = await ctx.promise("review", type_hint=str).value()
```

### Resolving/rejecting promises

Resolve/reject from any workflow handler:

```py {"CODE_LOAD::python/src/develop/durable_promise.py#resolve_promise"}  theme={null}
await ctx.promise("review", type_hint=str).resolve(review)
```

### Complete workflow example

```py expandable {"CODE_LOAD::python/src/develop/durable_promise.py#review"}  theme={null}
review_workflow = restate.Workflow("ReviewWorkflow")


@review_workflow.main()
async def run(ctx: restate.WorkflowContext, document_id: str):
    # Send document for review
    await ctx.run_typed("ask review", ask_review, document_id=document_id)

    # Wait for external review submission
    review = await ctx.promise("review", type_hint=str).value()

    # Process the review result
    return process_review(document_id, review)


@review_workflow.handler()
async def submit_review(ctx: restate.WorkflowSharedContext, review: str):
    # Signal the waiting run handler
    await ctx.promise("review", type_hint=str).resolve(review)


app = restate.app([review_workflow])
```

### Two signaling patterns

**External → Workflow** (shown above): External handlers signal the run handler

* Use for human approvals, external API responses, manual interventions
* External handlers call the handler which resolves the promise

**Workflow → External**: Run handler signals other handlers waiting for workflow events

* Use for step completion notifications, status updates, result broadcasting
* Run handler resolves promises that external handlers are awaiting

## Best Practices

* **Use awakeables** for services/objects coordinating with external systems
* **Use durable promises** for workflow signaling
* **Always handle rejections** to gracefully manage failures
* **Include timeouts** for long-running external processes