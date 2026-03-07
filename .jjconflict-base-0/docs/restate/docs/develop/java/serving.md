# Serving

Source: https://docs.restate.dev/develop/java/serving

Create an endpoint to serve your services.

Restate services can run in two ways: as an HTTP endpoint or as AWS Lambda functions.

## Creating an HTTP endpoint

1. Use either  or  as SDK dependency.
2. Create an endpoint
3. Bind one or multiple services to it
4. Listen on the specified port (default `9080`) for connections and requests.

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingHttp.java#here"}  theme={null}
  import dev.restate.sdk.endpoint.Endpoint;
  import dev.restate.sdk.http.vertx.RestateHttpServer;

  class MyApp {
    public static void main(String[] args) {
      RestateHttpServer.listen(
          Endpoint.bind(new MyService()).bind(new MyObject()).bind(new MyWorkflow()), 8080);
    }
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingHttp.kt#here"}  theme={null}
  fun main() {
    RestateHttpServer.listen(
        endpoint {
          bind(MyService())
          bind(MyObject())
          bind(MyWorkflow())
        })
  }
  ```
</CodeGroup>

## Creating a Lambda handler

1. Use either  or   as SDK dependency.
2. Extend the class `BaseRestateLambdaHandler`
3. Override the register method
4. Bind one or multiple services to the builder

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingLambda.java#here"}  theme={null}
  import dev.restate.sdk.endpoint.Endpoint;
  import dev.restate.sdk.lambda.BaseRestateLambdaHandler;

  class MyLambdaHandler extends BaseRestateLambdaHandler {
    @Override
    public void register(Endpoint.Builder builder) {
      builder.bind(new MyService()).bind(new MyObject());
    }
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/MyLambdaHandler.kt#here"}  theme={null}
  import dev.restate.sdk.endpoint.Endpoint
  import dev.restate.sdk.lambda.BaseRestateLambdaHandler

  class MyLambdaHandler : BaseRestateLambdaHandler() {
    override fun register(builder: Endpoint.Builder) {
      builder.bind(MyService()).bind(MyObject())
    }
  }
  ```
</CodeGroup>

The implementation of your services and handlers remains the same for both deployment options.
Have a look at the [deployment section](/services/deploy/lambda) for guidance on how to deploy your services on AWS Lambda.

<Accordion title="Using Java 21 Virtual Threads">
  If you use a JVM >= 21, you can use virtual threads to run your services:

  <CodeGroup>
    ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingVirtualThreads.java#here"}  theme={null}
    builder.bind(
        new Greeter(),
        HandlerRunner.Options.withExecutor(Executors.newVirtualThreadPerTaskExecutor()));
    ```

    ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingVirtualThreads.kt#here"}  theme={null}
    builder.bind(
        Greeter(),
        HandlerRunner.Options(
            coroutineContext = Executors.newVirtualThreadPerTaskExecutor().asCoroutineDispatcher(),
        ),
    )
    ```
  </CodeGroup>
</Accordion>

## Validating request identity

SDKs can validate that incoming requests come from a particular Restate
instance. You can find out more about request identity in the
[Security docs](/services/security#locking-down-service-access). You will need to use the request identity dependency .

<CodeGroup>
  ```java Java {"CODE_LOAD::java/src/main/java/develop/ServingIdentity.java#here"}  theme={null}
  import dev.restate.sdk.auth.signing.RestateRequestIdentityVerifier;
  import dev.restate.sdk.endpoint.Endpoint;
  import dev.restate.sdk.http.vertx.RestateHttpServer;

  class MySecureApp {
    public static void main(String[] args) {
      var endpoint =
          Endpoint.bind(new MyService())
              .withRequestIdentityVerifier(
                  RestateRequestIdentityVerifier.fromKeys(
                      "publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f"));
      RestateHttpServer.listen(endpoint);
    }
  }
  ```

  ```kotlin Kotlin {"CODE_LOAD::kotlin/src/main/kotlin/develop/ServingIdentity.kt#here"}  theme={null}
  import dev.restate.sdk.auth.signing.RestateRequestIdentityVerifier
  import dev.restate.sdk.http.vertx.RestateHttpServer
  import dev.restate.sdk.kotlin.endpoint.endpoint

  fun main() {
    RestateHttpServer.listen(
        endpoint {
          bind(MyService())
          requestIdentityVerifier =
              RestateRequestIdentityVerifier.fromKeys(
                  "publickeyv1_w7YHemBctH5Ck2nQRQ47iBBqhNHy4FV7t2Usbye2A6f",
              )
        })
  }
  ```
</CodeGroup>