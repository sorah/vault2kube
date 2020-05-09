local vars = import './vars.libsonnet';
{
  apiVersion: 'v1',
  kind: 'ServiceAccount',
  metadata: {
    name: vars.name,
    namespace: vars.namespace,
  },
}
