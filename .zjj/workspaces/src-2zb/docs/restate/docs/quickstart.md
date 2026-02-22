# Quickstart

Source: https://docs.restate.dev/quickstart

Develop and run your first Restate service

This guide takes you through your first steps with Restate.

We will run a simple Restate Greeter service that listens on port `9080` and responds with `You said hi to <name>!` to a `greet` request.

<img alt="Quickstart" />

Select your SDK:

<Tabs>
  <Tab title="TypeScript" icon={"/img/languages/typescript.svg"}>
    <iframe title="YouTube video player" />

    Select your favorite runtime:

    <Tabs>
      <Tab title="Node.js" icon={"/img/quickstart/nodejs.svg"}>
        <Info>
          **Prerequisites**:

          * [NodeJS](https://nodejs.org/en/) >= v20
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example typescript-hello-world &&
              cd typescript-hello-world &&
              npm install
              ```

              ```shell npx theme={null}
              npx -y @restatedev/create-app@latest && cd restate-node-template &&
              npm install
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/node/src/app.ts?collapse_prequel"}  theme={null}
            const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            restate.serve({
              services: [greeter],
              port: 9080,
            });
            ```

            <GitHubLink />

            <Accordion title="See how a failing request is retried">
              Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

              ```shell theme={null}
              curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
              ```

              Expected output:

              ```shell theme={null}
              You said hi to Alice!
              ```

              You can see in the service logs and in the Restate UI how the request gets retried.
              On a retry, it skipped the steps that already succeeded.

              <img alt="Restate UI Durable Execution" />

              Even the sleep is durable and tracked by Restate.
              If you kill/restart the service halfway through, the sleep will only last for what remained.

              <Tip title="Durable Execution">
                Restate persists the progress of the handler.
                Letting you write code that is resilient to failures out of the box.
                Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
              </Tip>
            </Accordion>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Bun" icon={"/img/quickstart/bun.svg"}>
        <Info>
          **Prerequisites**:

          * [Bun](https://bun.sh/docs/installation)
          * [npm CLI](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) >= 9.6.7
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-bun &&
            cd typescript-hello-world-bun &&
            npm install
            ```

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/bun/src/index.ts?collapse_prequel"}  theme={null}
            const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            restate.serve({
              services: [greeter],
              port: 9080,
            });
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Deno" icon={"/img/quickstart/deno.svg"}>
        <Info>
          **Prerequisites**:

          * [Deno](https://deno.land/#installation)
          * [npm CLI](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) >= 9.6.7
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-deno &&
            cd typescript-hello-world-deno
            ```

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            deno task dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/deno/main.ts?collapse_prequel"}  theme={null}
            export const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            const handler = restate.createEndpointHandler({
              services: [greeter],
              bidirectional: true,
            });

            Deno.serve({ port: 9080 }, handler);
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Cloudflare Workers" icon={"/img/quickstart/cloudflare_workers.svg"}>
        <Info>
          **Prerequisites**:

          * [NodeJS](https://nodejs.org/en/) >= v20
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-cloudflare-worker &&
            cd typescript-hello-world-cloudflare-worker &&
            npm install
            ```

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080 --use-http1.1
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080", "use_http_11": true}'
              ```
            </CodeGroup>

            Expected output:

            <CodeGroup>
              ```shell CLI theme={null}
              ❯ SERVICES THAT WILL BE ADDED:
              - Greeter
              Type: Service
              HANDLER  INPUT                                     OUTPUT
              greet    value of content-type 'application/json'  value of content-type 'application/json'


              ✔ Are you sure you want to apply those changes? · yes
              ✅ DEPLOYMENT:
              SERVICE  REV
              Greeter  1
              ```

              ```shell curl theme={null}
              {
              "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                  "services": [
                      {
                          "name": "Greeter",
                          "handlers": [
                              {
                                  "name": "greet",
                                  "ty": "Shared",
                                  "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                  "output_description": "value of content-type 'application/json'"
                              }
                          ],
                          "ty": "Service",
                          "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                          "revision": 1,
                          "public": true,
                          "idempotency_retention": "1day"
                      }
                  ]
              }
              ```
            </CodeGroup>

            &#x20;does not support HTTP2, so we need to tell Restate to use HTTP1.1.

            If you run Restate with Docker, use `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Info title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Info>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/cloudflare-worker/src/index.ts?collapse_prequel"}  theme={null}
            const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            export default {
                fetch: restate.createEndpointHandler({ services: [greeter] })
            };
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Next.js/Vercel" icon={"/img/quickstart/nextjs.svg"}>
        <Info>
          **Prerequisites**:

          * [NodeJS](https://nodejs.org/en/) >= v20
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            ```shell theme={null}
            restate example typescript-hello-world-vercel &&
            cd typescript-hello-world-vercel &&
            npm install
            ```

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            npm run dev
            ```
          </Step>

          <Step title="Register the service">
            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:3000/restate --use-http1.1
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:3000/restate", "use_http_11": true}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                        {
                            "name": "greet",
                            "ty": "Shared",
                            "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                            "output_description": "value of content-type 'application/json'"
                        }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, use `http://host.docker.internal:3000/restate` instead of `http://localhost:3000/restate`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/cloud/connecting-services#connecting-services-in-private-environments) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/typescript/templates/vercel/src/restate/greeter.ts?collapse_prequel"}  theme={null}
            export const greeter = restate.service({
              name: "Greeter",
              handlers: {
                greet: restate.createServiceHandler(
                  { input: restate.serde.schema(Greeting), output: restate.serde.schema(GreetingResponse) },
                  async (ctx: restate.Context, { name }) => {
                    // Durably execute a set of steps; resilient against failures
                    const greetingId = ctx.rand.uuidv4();
                    await ctx.run("Notification", () => sendNotification(greetingId, name));
                    await ctx.sleep({ seconds: 1 });
                    await ctx.run("Reminder", () => sendReminder(greetingId, name));

                    // Respond to caller
                    return { result: `You said hi to ${name}!` };
                  },
                ),
              },
            });

            export type Greeter = typeof greeter;
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>

  <Tab title="Java" icon={"/img/languages/java.svg"}>
    Select your favorite build tool and framework:

    <Tabs>
      <Tab title="Maven + Spring Boot" icon={"/img/quickstart/spring.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-maven-spring-boot &&
              cd java-hello-world-maven-spring-boot
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-maven-spring-boot.zip &&
              unzip java-hello-world-maven-spring-boot.zip -d java-hello-world-maven-spring-boot &&
              rm java-hello-world-maven-spring-boot.zip && cd java-hello-world-maven-spring-boot
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            mvn compile spring-boot:run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-maven-spring-boot/src/main/java/com/example/restatestarter/Greeter.java?collapse_prequel"}  theme={null}
            @RestateService
            public class Greeter {

              @Value("${greetingPrefix}")
              private String greetingPrefix;

              public record Greeting(String name) {}
              public record GreetingResponse(String message) {}

              @Handler
              public GreetingResponse greet(Context ctx, Greeting req) {
                // Durably execute a set of steps; resilient against failures
                String greetingId = ctx.random().nextUUID().toString();
                ctx.run("Notification", () -> sendNotification(greetingId, req.name));
                ctx.sleep(Duration.ofSeconds(1));
                ctx.run("Reminder", () -> sendReminder(greetingId, req.name));

                // Respond to caller
                return new GreetingResponse("You said " + greetingPrefix + " to " + req.name + "!");
              }
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Maven + Quarkus" icon={"/img/quickstart/quarkus.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-maven-quarkus &&
              cd java-hello-world-maven-quarkus
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-maven-quarkus.zip &&
              unzip java-hello-world-maven-quarkus.zip -d java-hello-world-maven-quarkus &&
              rm java-hello-world-maven-quarkus.zip && cd java-hello-world-maven-quarkus
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            quarkus dev
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-maven-quarkus/src/main/java/org/acme/Greeter.java?collapse_prequel"}  theme={null}
            @Service
            public class Greeter {

              @ConfigProperty(name = "greetingPrefix") String greetingPrefix;

              public record Greeting(String name) {}
              public record GreetingResponse(String message) {}

              @Handler
              public GreetingResponse greet(Context ctx, Greeting req) {
                // Durably execute a set of steps; resilient against failures
                String greetingId = ctx.random().nextUUID().toString();
                ctx.run("Notification", () -> sendNotification(greetingId, req.name));
                ctx.sleep(Duration.ofSeconds(1));
                ctx.run("Reminder", () -> sendReminder(greetingId, req.name));

                // Respond to caller
                return new GreetingResponse("You said hi to " + req.name + "!");
              }
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Maven" icon={"/img/quickstart/maven.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-maven &&
              cd java-hello-world-maven
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-maven.zip &&
              unzip java-hello-world-maven.zip -d java-hello-world-maven &&
              rm java-hello-world-maven.zip && cd java-hello-world-maven
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            mvn compile exec:java
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-maven/src/main/java/my/example/Greeter.java?collapse_prequel"}  theme={null}
            @Service
            public class Greeter {

              public record Greeting(String name) {}
              public record GreetingResponse(String message) {}

              @Handler
              public GreetingResponse greet(Context ctx, Greeting req) {
                // Durably execute a set of steps; resilient against failures
                String greetingId = ctx.random().nextUUID().toString();
                ctx.run("Notification", () -> sendNotification(greetingId, req.name));
                ctx.sleep(Duration.ofSeconds(1));
                ctx.run("Reminder", () -> sendReminder(greetingId, req.name));

                // Respond to caller
                return new GreetingResponse("You said hi to " + req.name + "!");
              }

              public static void main(String[] args) {
                RestateHttpServer.listen(Endpoint.bind(new Greeter()));
              }
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Gradle" icon={"/img/quickstart/gradle.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example java-hello-world-gradle &&
              cd java-hello-world-gradle
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/java-hello-world-gradle.zip &&
              unzip java-hello-world-gradle.zip -d java-hello-world-gradle &&
              rm java-hello-world-gradle.zip && cd java-hello-world-gradle
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE, run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            ./gradlew run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```java {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/java/templates/java-gradle/src/main/java/my/example/Greeter.java?collapse_prequel"}  theme={null}
            @Service
            public class Greeter {

                public record Greeting(String name) {}
                public record GreetingResponse(String message) {}

                @Handler
                public GreetingResponse greet(Context ctx, Greeting req) {
                    // Durably execute a set of steps; resilient against failures
                    String greetingId = ctx.random().nextUUID().toString();
                    ctx.run("Notification", () -> sendNotification(greetingId, req.name));
                    ctx.sleep(Duration.ofSeconds(1));
                    ctx.run("Reminder", () -> sendReminder(greetingId, req.name));

                    // Respond to caller
                    return new GreetingResponse("You said hi to " + req.name + "!");
                }

                public static void main(String[] args) {
                    RestateHttpServer.listen(Endpoint.bind(new Greeter()));
                }
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>

  <Tab title="Kotlin" icon={"/img/languages/kotlin.svg"}>
    Select your favorite framework:

    <Tabs>
      <Tab title="Gradle + Spring Boot" icon={"/img/quickstart/spring.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example kotlin-hello-world-gradle-spring-boot &&
              cd kotlin-hello-world-gradle-spring-boot
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/kotlin-hello-world-gradle-spring-boot.zip &&
              unzip kotlin-hello-world-gradle-spring-boot.zip -d kotlin-hello-world-gradle-spring-boot &&
              rm kotlin-hello-world-gradle-spring-boot.zip && cd kotlin-hello-world-gradle-spring-boot
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE and configure it to build with Gradle.
            Run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            ./gradlew bootRun
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```kotlin {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/kotlin/templates/kotlin-gradle-spring-boot/src/main/kotlin/com/example/restatestarter/Greeter.kt?collapse_prequel"}  theme={null}
            @RestateService
            class Greeter {

              @Value("\${greetingPrefix}")
              lateinit var greetingPrefix: String

              @Serializable
              data class Greeting(val name: String)
              @Serializable
              data class GreetingResponse(val message: String)

              @Handler
              suspend fun greet(ctx: Context, req: Greeting): GreetingResponse {
                // Durably execute a set of steps; resilient against failures
                val greetingId = ctx.random().nextUUID().toString()
                ctx.runBlock("Notification") { sendNotification(greetingId, req.name) }
                ctx.sleep(1.seconds)
                ctx.runBlock("Reminder") { sendReminder(greetingId, req.name) }

                // Respond to caller
                return GreetingResponse("You said $greetingPrefix to ${req.name}!")
              }
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Gradle" icon={"/img/quickstart/gradle.svg"}>
        <Info>
          **Prerequisites**:

          * [JDK](https://whichjdk.com/) >= 17
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example kotlin-hello-world-gradle &&
              cd kotlin-hello-world-gradle
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/kotlin-hello-world-gradle.zip &&
              unzip kotlin-hello-world-gradle.zip -d kotlin-hello-world-gradle &&
              rm kotlin-hello-world-gradle.zip && cd kotlin-hello-world-gradle
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            You are all set to start developing your service.
            Open the project in an IDE and configure it to build with Gradle.
            Run your service and let it listen on port `9080` for requests:

            ```shell theme={null}
            ./gradlew run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```kotlin {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/kotlin/templates/kotlin-gradle/src/main/kotlin/my/example/Greeter.kt?collapse_prequel"}  theme={null}
            @Service
            class Greeter {

              @Serializable
              data class Greeting(val name: String)
              @Serializable
              data class GreetingResponse(val message: String)

              @Handler
              suspend fun greet(ctx: Context, req: Greeting): GreetingResponse {
                // Durably execute a set of steps; resilient against failures
                val greetingId = ctx.random().nextUUID().toString()
                ctx.runBlock("Notification") { sendNotification(greetingId, req.name) }
                ctx.sleep(1.seconds)
                ctx.runBlock("Reminder") { sendReminder(greetingId, req.name) }

                // Respond to caller
                return GreetingResponse("You said hi to ${req.name}!")
              }
            }

            fun main() {
              RestateHttpServer.listen(endpoint {
                bind(Greeter())
              })
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
            ```

            Expected output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>

  <Tab title="Go" icon={"/img/languages/go.svg"}>
    <Info>
      **Prerequisites**:

      * Go: >= 1.21.0
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the Greeter service template">
        <CodeGroup>
          ```shell CLI theme={null}
          restate example go-hello-world &&
          cd go-hello-world
          ```

          ```shell wget theme={null}
          wget https://github.com/restatedev/examples/releases/latest/download/go-hello-world.zip &&
          unzip go-hello-world.zip -d go-hello-world &&
          rm go-hello-world.zip && cd go-hello-world
          ```
        </CodeGroup>

        <GitHubLink />
      </Step>

      <Step title="Run the Greeter service">
        Now, start developing your service in `greeter.go`. Run it with:

        ```shell theme={null}
        go run .
        ```

        it will listen on port 9080 for requests.
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`) or via:

        <CodeGroup>
          ```shell CLI theme={null}
          restate deployments register http://localhost:9080
          ```

          ```shell curl theme={null}
          curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
          ```
        </CodeGroup>

        <Expandable title="Output">
          <CodeGroup>
            ```shell CLI theme={null}
            ❯ SERVICES THAT WILL BE ADDED:
            - Greeter
            Type: Service
            HANDLER  INPUT                                     OUTPUT
            Greet    value of content-type 'application/json'  value of content-type 'application/json'


            ✔ Are you sure you want to apply those changes? · yes
            ✅ DEPLOYMENT:
            SERVICE  REV
            Greeter  1
            ```

            ```shell curl theme={null}
            {
                "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "services": [
            {
                "name": "Greeter",
                "handlers": [
            {
                "name": "Greet",
                "ty": "Shared",
                "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                "output_description": "value of content-type 'application/json'"
            }
                ],
                "ty": "Service",
                "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "revision": 1,
                "public": true,
                "idempotency_retention": "1day"
            }
                ]
            }
            ```
          </CodeGroup>
        </Expandable>

        If you run Restate with Docker, use `http://host.docker.internal:9080` instead of `http://localhost:9080`.
      </Step>

      <Step title="Send a request to the Greeter service">
        Invoke the service via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <img alt="Restate UI Playground" />

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/Greeter/Greet --json '"Sarah"'
        ```

        Expected output:

        ```
        You said hi to Sarah!
        ```
      </Step>

      <Step title="Congratulations, you just ran Durable Execution!">
        The invocation you just sent used Durable Execution to make sure the request ran till completion.
        For each request, it sent a notification, slept for a second, and then sent a reminder.

        ```go {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/go/templates/go/greeter.go?collapse_prequel"}  theme={null}
        type Greeter struct{}

        func (Greeter) Greet(ctx restate.Context, name string) (string, error) {
          //  Durably execute a set of steps; resilient against failures
          greetingId := restate.UUID(ctx).String()

          if _, err := restate.Run(ctx,
            func(ctx restate.RunContext) (restate.Void, error) {
              return SendNotification(greetingId, name)
            },
            restate.WithName("Notification"),
          ); err != nil {
            return "", err
          }

          if err := restate.Sleep(ctx, 1*time.Second); err != nil {
            return "", err
          }

          if _, err := restate.Run(ctx,
            func(ctx restate.RunContext) (restate.Void, error) {
              return SendReminder(greetingId, name)
            },
            restate.WithName("Reminder"),
          ); err != nil {
            return "", err
          }

          // Respond to caller
          return "You said hi to " + name + "!", nil
        }
        ```

        <GitHubLink />

        Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

        ```shell theme={null}
        curl localhost:8080/Greeter/Greet --json '"Alice"'
        ```

        Expected output:

        ```
        You said hi to Alice!
        ```

        You can see in the service logs and in the Restate UI how the request gets retried.
        On a retry, it skipped the steps that already succeeded.

        <img alt="Restate UI Durable Execution" />

        Even the sleep is durable and tracked by Restate.
        If you kill/restart the service halfway through, the sleep will only last for what remained.

        <Tip title="Durable Execution">
          Restate persists the progress of the handler.
          Letting you write code that is resilient to failures out of the box.
          Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
        </Tip>
      </Step>
    </Steps>
  </Tab>

  <Tab title="Python" icon={"/img/languages/python.svg"}>
    <Info>
      **Prerequisites**:

      * Python >= v3.11
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the Greeter service template">
        <CodeGroup>
          ```shell CLI theme={null}
          restate example python-hello-world &&
          cd python-hello-world
          ```

          ```shell wget theme={null}
          wget https://github.com/restatedev/examples/releases/latest/download/python-hello-world.zip &&
          unzip python-hello-world.zip -d python-hello-world &&
          rm python-hello-world.zip && cd python-hello-world
          ```
        </CodeGroup>

        <GitHubLink />
      </Step>

      <Step title="Run the Greeter service">
        Run it and let it listen on port 9080 for requests:

        ```shell theme={null}
        uv run .
        ```
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`) or via:

        <CodeGroup>
          ```shell CLI theme={null}
          restate deployments register http://localhost:9080
          ```

          ```shell curl theme={null}
          curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
          ```
        </CodeGroup>

        <Expandable title="Output">
          <CodeGroup>
            ```shell CLI theme={null}
            ❯ SERVICES THAT WILL BE ADDED:
            - Greeter
            Type: Service
            HANDLER  INPUT                                     OUTPUT
            greet    value of content-type 'application/json'  value of content-type 'application/json'


            ✔ Are you sure you want to apply those changes? · yes
            ✅ DEPLOYMENT:
            SERVICE  REV
            Greeter  1
            ```

            ```shell curl theme={null}
            {
                "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "services": [
                    {
                        "name": "Greeter",
                        "handlers": [
                            {
                                "name": "greet",
                                "ty": "Shared",
                                "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                "output_description": "value of content-type 'application/json'"
                            }
                        ],
                        "ty": "Service",
                        "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                        "revision": 1,
                        "public": true,
                        "idempotency_retention": "1day"
                    }
                ]
            }
            ```
          </CodeGroup>
        </Expandable>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
          If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
        </Accordion>
      </Step>

      <Step title="Send a request to the Greeter service">
        Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

        <img alt="Restate UI Playground" />

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
        ```

        Expected output: `You said hi to Sarah!`
      </Step>

      <Step title="Congratulations, you just ran Durable Execution!">
        The invocation you just sent used Durable Execution to make sure the request ran till completion.
        For each request, it sent a notification, slept for a second, and then sent a reminder.

        ```python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/python/templates/python/app/greeter.py?collapse_prequel"}  theme={null}
        greeter = restate.Service("Greeter")


        @greeter.handler()
        async def greet(ctx: restate.Context, req: GreetingRequest) -> Greeting:
            # Durably execute a set of steps; resilient against failures
            greeting_id = str(ctx.uuid())
            await ctx.run_typed("notification", send_notification, greeting_id=greeting_id, name=req.name)
            await ctx.sleep(timedelta(seconds=1))
            await ctx.run_typed("reminder", send_reminder, greeting_id=greeting_id, name=req.name)

            # Respond to caller
            return Greeting(message=f"You said hi to {req.name}!")
        ```

        <GitHubLink />

        Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

        ```shell theme={null}
        curl localhost:8080/Greeter/greet --json '{"name": "Alice"}'
        ```

        Expected output:

        ```shell theme={null}
        You said hi to Alice!
        ```

        You can see in the service logs and in the Restate UI how the request gets retried.
        On a retry, it skipped the steps that already succeeded.

        <img alt="Restate UI Durable Execution" />

        Even the sleep is durable and tracked by Restate.
        If you kill/restart the service halfway through, the sleep will only last for what remained.

        <Tip title="Durable Execution">
          Restate persists the progress of the handler.
          Letting you write code that is resilient to failures out of the box.
          Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
        </Tip>
      </Step>
    </Steps>
  </Tab>

  <Tab title="Rust" icon={"/img/languages/rust.svg"}>
    Select your favorite runtime:

    <Tabs>
      <Tab title="Tokio" icon={"/img/quickstart/tokio.svg"}>
        <Info>
          **Prerequisites**:

          * [Rust](https://rustup.rs/)
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example rust-hello-world &&
              cd rust-hello-world
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/rust-hello-world.zip &&
              unzip rust-hello-world.zip -d rust-hello-world &&
              rm rust-hello-world.zip && cd rust-hello-world
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            cargo run
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
              ```
            </CodeGroup>

            <Expandable title="Output">
              <CodeGroup>
                ```shell CLI theme={null}
                ❯ SERVICES THAT WILL BE ADDED:
                - Greeter
                Type: Service
                HANDLER  INPUT                                     OUTPUT
                greet    value of content-type 'application/json'  value of content-type 'application/json'


                ✔ Are you sure you want to apply those changes? · yes
                ✅ DEPLOYMENT:
                SERVICE  REV
                Greeter  1
                ```

                ```shell curl theme={null}
                {
                    "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                    "services": [
                        {
                            "name": "Greeter",
                            "handlers": [
                                {
                                    "name": "greet",
                                    "ty": "Shared",
                                    "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                    "output_description": "value of content-type 'application/json'"
                                }
                            ],
                            "ty": "Service",
                            "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                            "revision": 1,
                            "public": true,
                            "idempotency_retention": "1day"
                        }
                    ]
                }
                ```
              </CodeGroup>
            </Expandable>

            If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Accordion title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Accordion>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '"Sarah"'
            ```

            Expected output:

            ```log theme={null}
            You said hi to Sarah!
            ```
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```rust {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/rust/templates/rust/src/main.rs?collapse_prequel"}  theme={null}
            #[restate_sdk::service]
            trait Greeter {
                async fn greet(name: String) -> Result<String, HandlerError>;
            }

            struct GreeterImpl;

            impl Greeter for GreeterImpl {
                async fn greet(&self, mut ctx: Context<'_>, name: String) -> Result<String, HandlerError> {
                    // Durably execute a set of steps; resilient against failures
                    let greeting_id = ctx.rand_uuid().to_string();
                    ctx.run(|| send_notification(&greeting_id, &name))
                        .name("notification")
                        .await?;
                    ctx.sleep(Duration::from_secs(1)).await?;
                    ctx.run(|| send_reminder(&greeting_id, &name))
                        .name("reminder")
                        .await?;

                    // Respond to caller
                    Ok(format!("You said hi to {name}"))
                }
            }

            #[tokio::main]
            async fn main() {
                // To enable logging
                tracing_subscriber::fmt::init();

                HttpServer::new(Endpoint::builder().bind(GreeterImpl.serve()).build())
                    .listen_and_serve("0.0.0.0:9080".parse().unwrap())
                    .await;
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '"Alice"'
            ```

            Example output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>

      <Tab title="Shuttle" icon={"/img/quickstart/shuttle-rust.png"}>
        <Info>
          **Prerequisites**:

          * [Rust](https://rustup.rs/)
          * [Shuttle](https://docs.shuttle.dev/getting-started/installation)
        </Info>

        <Steps>
          <Step title="Install Restate Server & CLI">
            Restate is a single self-contained binary. No external dependencies needed.

            <Tabs>
              <Tab title="Homebrew">
                ```shell theme={null}
                brew install restatedev/tap/restate-server restatedev/tap/restate
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Download binaries">
                Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

                <CodeGroup>
                  ```shell MacOS-x64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell MacOS-arm64 theme={null}
                  BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  sudo mv restate $BIN && \
                  sudo mv restate-server $BIN
                  ```

                  ```shell Linux-x64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```

                  ```shell Linux-arm64 theme={null}
                  BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
                  curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
                  tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
                  tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
                  chmod +x restate restate-server && \
                  mv restate $BIN && \
                  mv restate-server $BIN
                  ```
                </CodeGroup>

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="npm">
                ```shell theme={null}
                npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
                ```

                Start the server:

                ```shell theme={null}
                restate-server
                ```
              </Tab>

              <Tab title="Docker">
                Run the Restate Server:

                ```shell theme={null}
                docker run --name restate_dev --rm \
                -p 8080:8080 -p 9070:9070 -p 9071:9071 \
                --add-host=host.docker.internal:host-gateway \
                docker.restate.dev/restatedev/restate:latest
                ```

                Run CLI commands:

                ```shell theme={null}
                docker run -it --network=host \
                docker.restate.dev/restatedev/restate-cli:latest \
                invocations ls
                ```

                Replace `invocations ls` with any CLI subcommand.
              </Tab>
            </Tabs>

            You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
          </Step>

          <Step title="Get the Greeter service template">
            <CodeGroup>
              ```shell CLI theme={null}
              restate example rust-hello-world-shuttle &&
              cd rust-hello-world-shuttle
              ```

              ```shell wget theme={null}
              wget https://github.com/restatedev/examples/releases/latest/download/rust-hello-world-shuttle.zip &&
              unzip rust-hello-world-shuttle.zip -d rust-hello-world-shuttle &&
              rm rust-hello-world-shuttle.zip && cd rust-hello-world-shuttle
              ```
            </CodeGroup>

            <GitHubLink />
          </Step>

          <Step title="Run the Greeter service">
            ```shell theme={null}
            cargo shuttle run --port 9080
            ```
          </Step>

          <Step title="Register the service">
            Tell Restate where the service is running, so Restate can discover and register the services and handlers behind this endpoint.
            You can do this via the UI (`http://localhost:9070`) or via:

            <CodeGroup>
              ```shell CLI theme={null}
              restate deployments register http://localhost:9080 --use-http1.1
              ```

              ```shell curl theme={null}
              curl localhost:9070/deployments --json '{"uri": "http://localhost:9080", "use_http_11": true}'
              ```
            </CodeGroup>

            Expected output:

            <CodeGroup>
              ```shell CLI theme={null}
              ❯ SERVICES THAT WILL BE ADDED:
              - Greeter
              Type: Service
              HANDLER  INPUT                                     OUTPUT
              greet    value of content-type 'application/json'  value of content-type 'application/json'


              ✔ Are you sure you want to apply those changes? · yes
              ✅ DEPLOYMENT:
              SERVICE  REV
              Greeter  1
              ```

              ```shell curl theme={null}
              {
              "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                  "services": [
                      {
                          "name": "Greeter",
                          "handlers": [
                              {
                                  "name": "greet",
                                  "ty": "Shared",
                                  "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                  "output_description": "value of content-type 'application/json'"
                              }
                          ],
                          "ty": "Service",
                          "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                          "revision": 1,
                          "public": true,
                          "idempotency_retention": "1day"
                      }
                  ]
              }
              ```
            </CodeGroup>

            &#x20;does not support HTTP2, so we need to tell Restate to use HTTP1.1.

            If you run Restate with Docker, use `http://host.docker.internal:9080` instead of `http://localhost:9080`.

            <Info title="Restate Cloud">
              When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
              If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
            </Info>
          </Step>

          <Step title="Send a request to the Greeter service">
            Invoke the service via the Restate UI playground: go to `http://localhost:9070`,  click on your service and then on playground.

            <img alt="Restate UI Playground" />

            Or invoke via `curl`:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '{"name": "Sarah"}'
            ```

            Expected output: `You said hi to Sarah!`
          </Step>

          <Step title="Congratulations, you just ran Durable Execution!">
            The invocation you just sent used Durable Execution to make sure the request ran till completion.
            For each request, it sent a notification, slept for a second, and then sent a reminder.

            ```rust {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/examples/refs/heads/main/rust/templates/rust-shuttle/src/main.rs?collapse_prequel"}  theme={null}
            #[restate_sdk::service]
            trait Greeter {
                async fn greet(name: String) -> Result<String, HandlerError>;
            }

            struct GreeterImpl;

            impl Greeter for GreeterImpl {
                async fn greet(&self, mut ctx: Context<'_>, name: String) -> Result<String, HandlerError> {
                    // Durably execute a set of steps; resilient against failures
                    let greeting_id = ctx.rand_uuid().to_string();
                    ctx.run(|| send_notification(&greeting_id, &name))
                        .name("notification")
                        .await?;
                    ctx.sleep(Duration::from_secs(1)).await?;
                    ctx.run(|| send_reminder(&greeting_id, &name))
                        .name("reminder")
                        .await?;

                    // Respond to caller
                    Ok(format!("You said hi to {name}"))
                }
            }

            #[shuttle_runtime::main]
            async fn main() -> Result<RestateShuttleEndpoint, shuttle_runtime::Error> {
                Ok(RestateShuttleEndpoint::new(
                    Endpoint::builder().bind(GreeterImpl.serve()).build(),
                ))
            }
            ```

            <GitHubLink />

            Send a request for `Alice` to see how the service behaves when it occasionally fails to send the reminder and notification:

            ```shell theme={null}
            curl localhost:8080/Greeter/greet --json '"Alice"'
            ```

            Example output:

            ```shell theme={null}
            You said hi to Alice!
            ```

            You can see in the service logs and in the Restate UI how the request gets retried.
            On a retry, it skipped the steps that already succeeded.

            <img alt="Restate UI Durable Execution" />

            Even the sleep is durable and tracked by Restate.
            If you kill/restart the service halfway through, the sleep will only last for what remained.

            <Tip title="Durable Execution">
              Restate persists the progress of the handler.
              Letting you write code that is resilient to failures out of the box.
              Have a look at the [Durable Execution page](/foundations/key-concepts#durable-execution) to learn more.
            </Tip>
          </Step>
        </Steps>
      </Tab>
    </Tabs>
  </Tab>
</Tabs>

<Tip>
  [Once you are done with the quickstart, check out how to write Restate apps with AI assistants.](/develop/ai-assistant)
</Tip>