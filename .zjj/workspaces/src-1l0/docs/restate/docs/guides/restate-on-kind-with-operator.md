# Kubernetes deployments with Restate Operator

Source: https://docs.restate.dev/guides/restate-on-kind-with-operator

Learn how to deploy single-node Restate with Restate operator on a kind Kubernetes cluster.

This guide walks you through deploying a Restate cluster on Kubernetes using the Restate Operator.

We will use [kind](https://kind.sigs.k8s.io/) to create a local Kubernetes cluster for testing purposes.

<Steps>
  <Step title="Prerequisites">
    Install the following on your local machine:

    * [Docker](https://docs.docker.com/)
    * [Helm](https://helm.sh/docs/intro/install/)
    * [kubectl](https://kubernetes.io/docs/tasks/tools/#kubectl)
    * [Restate](/installation)
    * [kind](https://kind.sigs.k8s.io/docs/user/quick-start)

    This guide was written with kind v0.30.0, [Restate Operator](https://github.com/restatedev/restate-operator/tree/main) v1.8.1, and Restate v1.5.0.
    Contact us on [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev) if something does not work as expected.
  </Step>

  <Step title="Create a kind cluster">
    ```bash theme={null}
    kind create cluster
    ```

    Set `kubectl` context to `kind-kind`:

    ```bash theme={null}
    kubectl cluster-info --context kind-kind
    ```
  </Step>

  <Step title="Install the Restate Operator">
    Install the Restate Operator via Helm:

    ```bash theme={null}
    helm install restate-operator \
      oci://ghcr.io/restatedev/restate-operator-helm \
      --namespace restate-operator \
      --create-namespace
    ```

    To install the operator, you need to be able to create namespaces and CRDs.
  </Step>

  <Step title="Create a Restate cluster">
    Create a `RestateCluster` manifest in a file called `restate-cluster.yaml`:

    ```yaml restate-cluster.yaml theme={null}
    apiVersion: restate.dev/v1
    kind: RestateCluster
    metadata:
      name: restate-test
    spec:
      compute:
        image: restatedev/restate:1.5
      storage:
        storageRequestBytes: 2147483648 # 2 GiB
      security:
        disableNetworkPolicies: true
    ```

    Note that we disable network policies here for simplicity, but in a production environment, you should configure appropriate network policies for security.

    To learn more about the `RestateCluster` spec options, see the [`RestateCluster` pkl definition](https://github.com/restatedev/restate-operator/blob/main/crd/RestateCluster.pkl).

    Apply the manifest:

    ```bash theme={null}
    kubectl apply -n restate-test -f restate-cluster.yaml
    ```

    A single-node Restate deployment will be created in the `restate-test` namespace.

    Wait until the `restate-0` pod is in the `Ready` state:

    ```bash theme={null}
    kubectl get pods -n restate-test -w
    ```
  </Step>

  <Step title="Forward the ports of the Restate ingress and the UI to your local machine">
    ```bash theme={null}
    kubectl port-forward svc/restate -n restate-test 8080:8080 9070:9070
    ```

    Now you can access the Restate UI at `http://localhost:9070`.
  </Step>

  <Step title="Build and upload a Restate service Docker image to the kind cluster">
    For example, download the TypeScript Hello World service:

    ```bash theme={null}
    restate example typescript-hello-world &&
    cd typescript-hello-world &&
    npm install
    ```

    Build a Docker image for the service:

    ```bash theme={null}
    docker build -t my-restate-service:0.0.1 .
    ```

    Upload the image to the kind cluster:

    ```bash theme={null}
    kind load docker-image my-restate-service:0.0.1
    ```
  </Step>

  <Step title="Deploy the Restate service">
    Create a `RestateDeployment` manifest for the service in a file called `service-deployment.yaml`:

    ```yaml service-deployment.yaml theme={null}
    apiVersion: restate.dev/v1beta1
    kind: RestateDeployment
    metadata:
      name: my-app
    spec:
      replicas: 1
      restate:
        register:
          service:
            name: restate
            namespace: restate-test
      selector:
        matchLabels:
          app: my-app
      template:
        metadata:
          labels:
            app: my-app
        spec:
          containers:
          - name: app
            image: my-restate-service:0.0.1
            ports:
            - name: restate
              containerPort: 9080
    ```

    Apply the manifest to deploy the service:

    ```bash theme={null}
    kubectl apply -f service-deployment.yaml -n restate-test
    ```

    You should now see the deployed service listed in the Restate UI:

    <Frame>
      <img />
    </Frame>

    The Restate Operator automatically [registered](/services/versioning#registering-a-deployment) the service with the Restate cluster.
  </Step>

  <Step title="Invoke the service">
    In the Restate UI playground, you can now invoke the service. In the overview page, click on the `greet` handler of your service to open the playground.
    Then send a request:

    <Frame>
      <img />
    </Frame>
  </Step>

  <Step title="🎉 You did it!">
    You have successfully deployed a Restate cluster and a Restate service on a local kind Kubernetes cluster using the Restate Operator!

    Check out the [Restate Operator docs](/server/deploy/kubernetes#restate-kubernetes-operator) for further information.
  </Step>
</Steps>

After you are done experimenting, you can delete the kind cluster:

```bash theme={null}
kind delete cluster
```