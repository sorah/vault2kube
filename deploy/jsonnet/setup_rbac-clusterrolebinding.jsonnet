local vars = import './vars.libsonnet';
{
  apiVersion: 'rbac.authorization.k8s.io/v1',
  kind: 'ClusterRoleBinding',
  metadata: {
    name: vars.name,
  },
  roleRef: {
    apiGroup: 'rbac.authorization.k8s.io',
    kind: 'ClusterRole',
    name: vars.name,
  },
  subjects: [
    {
      kind: 'ServiceAccount',
      namespace: vars.namespace,
      name: vars.name,
    },
  ],
}
