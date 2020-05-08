{
  apiVersion: 'rbac.authorization.k8s.io/v1',
  kind: 'ClusterRoleBinding',
  metadata: {
    name: 'vault2kube',
  },
  roleRef: {
    apiGroup: 'rbac.authorization.k8s.io',
    kind: 'ClusterRole',
    name: 'vault2kube',
  },
  subjects: [
    {
      kind: 'ServiceAccount',
      namespace: 'default',
      name: 'vault2kube',
    },
  ],
}
