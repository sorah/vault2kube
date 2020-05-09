local vars = import './vars.libsonnet';
{
  apiVersion: 'rbac.authorization.k8s.io/v1',
  kind: 'ClusterRole',
  metadata: {
    name: vars.name,
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
      verbs: ['get', 'list', 'patch'],
    },
    {
      apiGroups: [''],
      resources: ['secrets'],
      verbs: ['create', 'patch'],
    },
    {
      apiGroups: ['apps'],
      resources: ['deployments', 'daemonsets', 'statefulsets'],
      // resourceNames: [],
      verbs: ['patch'],
    },
  ],
}
