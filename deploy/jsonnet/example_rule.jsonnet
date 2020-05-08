{
  apiVersion: 'vault2kube.sorah.jp/v1',
  kind: 'VaultStoreRule',
  metadata: {
    name: 'vault2kube-test',
    namespace: 'default',
  },
  spec: {
    sourcePath: 'database/creds/xxx',
    destinationName: 'vault2kube-test',
    templates: [
      {
        key: 'username',
        template: '{{username}}',
      },
      {
        key: 'password',
        template: '{{password}}',
      },
    ],
    rolloutRestarts: [
      { kind: 'Deployment', name: 'hello-node' },
    ],
    renewBeforeSeconds: 604800,
    rotateBeforeSeconds: 259200,
    revokeAfterSeconds: 1,
  },
}
