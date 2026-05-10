# OpenBao policy: kalidoku-prod-read
# Apply with: bao policy write kalidoku-prod-read infra/bao/policy.hcl

path "secret/data/kalidoku/prod/*" {
  capabilities = ["read"]
}

path "secret/metadata/kalidoku/prod/*" {
  capabilities = ["read", "list"]
}

# shared SMTP (only if kalidoku ever needs to email — currently we use Resend, leave for symmetry)
path "secret/data/_shared/smtp" {
  capabilities = ["read"]
}
