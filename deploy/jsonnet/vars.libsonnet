{
  name: 'vault2kube',
  namespace: 'default',
  image: 'sorah/vault2kube:latest',
  env: [
    { name: 'VAULT_ADDR', value: 'https://path.to.your.vault:8200' },
    { name: 'VAULT_K8S_PATH', value: 'auth/kubernetes' },
    { name: 'VAULT_K8S_ROLE', value: $.name },
    // { name: 'VAULT_K8S_TOKEN_PATH', value: '/var/run/secrets/projection-tokens/token' },
    { name: 'RUST_LOG', value: 'info' },
  ],
}
