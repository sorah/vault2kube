# Vault2kube: manage Vault lease on Kubernetes secrets and keep it fresh

![docker-build](https://github.com/sorah/vault2kube/workflows/docker-build/badge.svg)

## What's this

Fetch a secret from Vault, and copy it to a k8s secret. Then rollout restart on specified k8s resources.

This tool is intended to run using k8s CronJob, and use with Vault _leased, managed_ secrets (e.g. database creds).
Note that the motivation behind this tool is not copying _static_ secrets from Vault; even is possible.

### Difference with vault-k8s

[hashicorp/vault-k8s](https://github.com/hashicorp/vault-k8s) is the official tool to integrate Vault secrets with Kubernetes.
This tool uses MutationWebhook to inject a `vault agent` into a pod, and the injected agent authenticate against Vault using a service account associated to the pod, then keep secrets updated.

This design allows to cover various use cases, however it is not simple for just copying leaesd, short-lived secrets (e.g. database) from Vault.

vault-k8s would be a good choice when you cannot trust k8s secrets store, or you need to interact with Vault to do some advanced usage.

## Docker repository

https://hub.docker.com/r/sorah/vault2kube

## Setup

Check [deploy/setup.yml](./deploy/setup.yml) and apply:

```
cp deploy/setup.yml /tmp/
vim /tmp/setup.yml
kubectl apply -f /tmp/setup.yml
```

By default this enables ClusterRole to update the entire secrets in your cluster. If you're not in favor of this whole cluster setup, you may use namespaced Role instead.
(Hint: you can use `--namespace` command line argument to enable namespaced API requests)

### Configuing connection to Vault

The following environment variables are supported, and some of them are required:

- Connection
  - `VAULT_ADDR` (required)
  - `VAULT_NAMESPACE`
  - `VAULT_CACERT`
- Authentication (required, choose from one of these)
  1. Bearer
    - `VAULT_TOKEN`
  2. Kubernetes
    - `VAULT_K8S_PATH` (required, e.g. `auth/kubernetes`)
    - `VAULT_K8S_ROLE` (required)
    - `VAULT_K8S_TOKEN_PATH` (optional, default to `/var/run/secrets/kubernetes.io/serviceaccount/token`)

### Kubernetes Authentication

Default to in-cluster config, but can refer to `$KUBECONFIG` (`~/.kube/config`).

(When not using in-cluster config, Kubernetes Vault authentication is unavailable)

## Usage

### Configure a rule

vault2kube uses CRD to configure a rule and persist its state. Create `VaultStoreRule` like as follows:

``` yaml
apiVersion: "vault2kube.sorah.jp/v1"
kind: VaultStoreRule
metadata:
  name: foo
  namespace: default
spec:
  ## Path to source secret on Vault (what you specify to `vault read` command)
  sourcePath: my/path/to/database-mount/creds/my-database-role

  ## Destination secret to store its lease as a k8s secret
  destinationName: my-database-creds
  # Templates to render stringData.
  templates:
    - key: password
      # Template is rendered using Handlebars against `.data` lease response
      template: '{{password}}'
    - key: username
      template: '{{username}}'

  ## Restart resource on any Vault lease rotation
  # Deployment, DaemonSet, StatefulSet is supported
  rolloutRestarts:
    - kind: Deployment
      name: blog

  ## Handling lease TTL
  # At least either renewBeforeSeconds or rotateBeforeSeconds must be given. 
  # Specifying both options are possible. Then rule will try to renew as long as possible, then rotate.

  # Enable this to renew while max_ttl. Specify threshold by seconds until expiry to perform a renew.
  renewBeforeSeconds: 604800
  # Enable this to rotate when reaching max_ttl. Note that even this parameter is omit, leases will be
  # rotated when a renewed ttl is capped to max_ttl.
  rotateBeforeSeconds: 259200
  # Enable this to revoke the last lease in subsequent run after rotation. Default to 1.
  revokeAfterSeconds: 3600
```

### Confirm working

Then trigger a job, and confirm the result.

```
kubectl create job --from=cronjob/vault2kube vault2kube-manual-${USER}-$(date +%s)
kubectl get secret my-database-creds
```

## Advanced topics

### Request force renew/rotate

Setting the following annotations to VaultStoreRule lets Vault2kube perform early renew/rotate:

- Renew: `vault2kube.sorah.jp/renewRequestedAt` (ISO8601 datetime format)
- Rotate: `vault2kube.sorah.jp/rotateRequestedAt` (ISO8601 datetime format)

Actions are performed when a specified datetime is newer than the last successful run (.status.lastSuccessfulRunAt).


## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/sorah/vault2kube.

## License

The tool is available as open source under the terms of the [MIT License](https://opensource.org/licenses/MIT).
