// CI/CD de kalidoku, exécutée par Jenkins (jenkins.calixteair.fr) avec la bibliothèque partagée
// calixteair (dépôt jenkins-lib) : Rust (fmt, clippy, tests contre Postgres et Valkey, cargo-audit,
// cargo-deny), web (pnpm : lint, typecheck, tests), gitleaks et Snyk, trois images + Trivy, Harbor.
//
//   PR               → tout est testé et construit, rien n'est publié
//   main             → images <composant>-sha-<commit> et <composant>-latest, sans déploiement
//   tag vX.Y.Z signé → images vX.Y.Z-<composant> (immuables), puis déploiement : le VPS (stack-sync)
//                      vérifie la signature, épingle les images et installe infra/docker/compose.yaml
//                      et infra/bao/agent.hcl. release-please prépare la version, Calixteair signe le tag.
@Library('calixteair@v1') _

// Outils Rust de la CI, installés une fois dans le cache Cargo du workspace (.ci/cargo)
String cargoTools = 'export CARGO_HOME=$PWD/.ci/cargo && export PATH=$CARGO_HOME/bin:$PATH' +
  ' && { command -v cargo-audit >/dev/null || cargo install --locked cargo-audit --version 0.22.2; }' +
  ' && { command -v cargo-deny >/dev/null || cargo install --locked cargo-deny --version 0.20.2; }'
// Avis suivis dans #41 (openidconnect 3.5 → 4.x) et dépendances transitives sans correctif : même liste que deny.toml
String cargoAudit = 'cargo audit --deny warnings --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2025-0111 --ignore RUSTSEC-2025-0134 --ignore RUSTSEC-2026-0098 --ignore RUSTSEC-2026-0099 --ignore RUSTSEC-2026-0104 --ignore RUSTSEC-2026-0173'
// pnpm du projet (packageManager de web/) activé par corepack dans /tmp : l'agent n'est pas root
String pnpm = 'mkdir -p /tmp/bin && corepack enable --install-directory /tmp/bin && export PATH=/tmp/bin:$PATH'

standardPipeline(
  test: [
    // Sans infos de debug et 4 tâches : l'édition de liens des binaires de test ne tenait pas dans les 3 Go
    // du démon dind (ld tué, signal 9) ; même résultat de tests, bien moins de mémoire
    [image: 'rust:1.88-bookworm',
     args: '-e HOME=/tmp -e CARGO_PROFILE_DEV_DEBUG=0 -e CARGO_PROFILE_TEST_DEBUG=0 -e CARGO_BUILD_JOBS=4' +
           ' -e DATABASE_URL=postgres://kalidoku@postgres:5432/kalidoku_test -e REDIS_URL=redis://valkey:6379/0',
     command: "${cargoTools} && cargo fmt --all -- --check" +
              ' && cargo clippy --workspace --all-targets --all-features -- -D warnings' +
              ' && cargo test --workspace --all-features' +
              " && ${cargoAudit} && cargo deny check",
     services: [
       [name: 'postgres', image: 'postgres:16-alpine',
        env: [POSTGRES_USER: 'kalidoku', POSTGRES_DB: 'kalidoku_test', POSTGRES_HOST_AUTH_METHOD: 'trust'],
        health: 'pg_isready -U kalidoku -d kalidoku_test'],
       [name: 'valkey', image: 'valkey/valkey:7.2-alpine', health: 'valkey-cli ping | grep -q PONG'],
     ]],
    // corepack lit packageManager dans web/ (la racine n'en a pas) : tout se lance depuis web/
    [image: 'node:22.23.4-bookworm-slim',
     args: '-e HOME=/tmp -e COREPACK_HOME=/tmp/corepack -e npm_config_store_dir=/tmp/pnpm-store -e CI=1',
     command: "${pnpm} && cd web && pnpm install --frozen-lockfile && pnpm run openapi:gen" +
              ' && pnpm run i18n:compile && pnpm run lint && pnpm run typecheck && pnpm test'],
  ],
  // Cache Cargo (.ci : sources des crates, avec leurs exemples npm/composer) et target/ hors de l'analyse
  snyk: [deps: '--all-projects --exclude=.ci,target'],
  registry: [url: 'registry.calixteair.fr/kalidoku', bao: 'cicd/harbor/kalidoku'],
  images: [
    [name: 'kalidoku', tagPrefix: 'server', context: '.', dockerfile: 'infra/docker/Dockerfile.server'],
    [name: 'kalidoku', tagPrefix: 'worker', context: '.', dockerfile: 'infra/docker/Dockerfile.worker'],
    [name: 'kalidoku', tagPrefix: 'web', context: '.', dockerfile: 'infra/docker/Dockerfile.web'],
  ],
  signedReleases: true,
  deploy: [stack: 'kalidoku', url: 'https://kalidoku.calixteair.fr/'],
)
