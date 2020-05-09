local vars = import './vars.libsonnet';
{
  apiVersion: 'batch/v1beta1',
  kind: 'CronJob',
  metadata: {
    name: vars.name,
    namespace: vars.namespace,
  },
  spec: {
    schedule: '18 */6 * * *',
    concurrencyPolicy: 'Replace',
    successfulJobsHistoryLimit: 3,
    failedJobsHistoryLimit: 5,
    jobTemplate: {
      spec: {
        template: {
          spec: {
            automountServiceAccountToken: true,
            serviceAccountName: vars.name,
            restartPolicy: 'Never',
            containers: [
              {
                name: vars.name,
                image: vars.image,
                imagePullPolicy: 'Always',
                command: ['/usr/bin/vault2kube', 'run'],
                env: vars.env,
              },
            ],
          },
        },
      },
    },
  },
}
