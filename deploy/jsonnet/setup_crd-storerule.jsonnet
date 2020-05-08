{
  apiVersion: 'apiextensions.k8s.io/v1',
  kind: 'CustomResourceDefinition',
  metadata: {
    name: 'vaultstorerules.vault2kube.sorah.jp',
  },
  spec: {
    group: 'vault2kube.sorah.jp',
    scope: 'Namespaced',
    names: {
      plural: 'vaultstorerules',
      singular: 'vaultstorerule',
      kind: 'VaultStoreRule',
      shortNames: ['vaultrule'],
    },
    versions: [
      {
        name: 'v1',
        served: true,
        storage: true,
        subresources: {
          status: {},
        },
        schema: {
          openAPIV3Schema: {
            type: 'object',
            properties: {
              spec: {
                type: 'object',
                properties: {
                  sourcePath: { type: 'string' },
                  destinationName: { type: 'string' },
                  templates: {
                    type: 'array',
                    items: {
                      type: 'object',
                      properties: {
                        key: { type: 'string' },
                        template: { type: 'string' },
                      },
                    },
                    required: ['key', 'template'],
                  },
                  rolloutRestarts: {
                    type: 'array',
                    nullable: true,
                    items: {
                      type: 'object',
                      properties: {
                        kind: { type: 'string' },
                        name: { type: 'string' },
                      },
                      required: ['kind', 'name'],
                    },
                  },
                  renewBeforeSeconds: { type: 'number', minimum: 0, nullable: true },
                  rotateBeforeSeconds: { type: 'number', minimum: 0, nullable: true },
                  revokeAfterSeconds: { type: 'number', minimum: 0, default: 1, nullable: true },
                },
                required: ['sourcePath', 'destinationName', 'templates'],
              },
              status: {
                type: 'object',
                properties: {
                  leaseId: { type: 'string', nullable: true },
                  ttl: { type: 'number', minimum: 0, nullable: true },
                  expiresAt: { type: 'string', nullable: true },
                  nextLeaseId: { type: 'string', nullable: true },
                  lastLeaseId: { type: 'string', nullable: true },
                  rotatedAt: { type: 'string', nullable: true },
                  lastRunStartedAt: { type: 'string', nullable: true },
                  lastSuccessfulRunAt: { type: 'string', nullable: true },
                },
              },
            },
          },
        },
      },
    ],
  },
}
