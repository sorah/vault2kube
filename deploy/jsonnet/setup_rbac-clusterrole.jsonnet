{
  apiVersion: 'rbac.authorization.k8s.io/v1',
  kind: 'ClusterRole',
  metadata: {
    name: 'vault2kube',
  },
  rules: [
    {
      apiGroups: ['vault2kube.sorah.jp'],
      resources: ['vaultstorerules'],
      verbs: ['get', 'list'],
    },
    {
      apiGroups: ['vault2kube.sorah.jp'],
      resources: ['vaultstorerules/status'],
      verbs: ['get', 'list', 'update'],
    },
    {
      apiGroups: [''],
      resources: ['secrets'],
      verbs: ['create', 'update'],
    },
  ],
}
