# Changelog

## 1.0.0 (2026-05-10)


### Features

* **core:** JSON Schema validation + nouveaux prédicats attr_list_size_* ([#42](https://github.com/Calixteair/kalidoku/issues/42)) ([da961de](https://github.com/Calixteair/kalidoku/commit/da961de333c858bc88d97abad819210416876d59))
* **core:** predicates + loader + CSP generator + CLI ([#10](https://github.com/Calixteair/kalidoku/issues/10)) ([997ab48](https://github.com/Calixteair/kalidoku/commit/997ab4898365132fcbec52d9627f8ff20a5fbb12))
* **domain-paris-metro:** full 303-stations dataset via OSM Overpass ([#13](https://github.com/Calixteair/kalidoku/issues/13)) ([40be9a8](https://github.com/Calixteair/kalidoku/commit/40be9a8fc8726b31e1088e520b5f012234121a6e))
* **security:** replace hCaptcha with self-hosted Altcha PoW ([#9](https://github.com/Calixteair/kalidoku/issues/9)) ([2088cf3](https://github.com/Calixteair/kalidoku/commit/2088cf3730d934644787c2d285e21fe5c8f86cce))
* **server:** full Axum backend skeleton + Altcha + migrations ([#15](https://github.com/Calixteair/kalidoku/issues/15)) ([c7872c9](https://github.com/Calixteair/kalidoku/commit/c7872c9061a8dc5b0ffae58875857f558c596895))
* **web:** i18n légal + E2E play + locale dynamique ([#38](https://github.com/Calixteair/kalidoku/issues/38)) ([b58e60d](https://github.com/Calixteair/kalidoku/commit/b58e60df1f11e2ffa8d847dbed744e3570df4747))
* **web:** pages + components + i18n + PWA + tests (mobile-first) ([#14](https://github.com/Calixteair/kalidoku/issues/14)) ([678247f](https://github.com/Calixteair/kalidoku/commit/678247fb4b7fc58e8fd4b5e765b53b6279c6c51d))
* **worker:** câble --once sur core::generator + test d'idempotence ([#40](https://github.com/Calixteair/kalidoku/issues/40)) ([a3fcb43](https://github.com/Calixteair/kalidoku/commit/a3fcb43cd1a27560dfbd21208cf0824a528bcc9f))
* **worker:** cron + once + queue scaffolding ([#11](https://github.com/Calixteair/kalidoku/issues/11)) ([d544e9e](https://github.com/Calixteair/kalidoku/commit/d544e9e0b74314ee102a35bb49b4f5a155cb94b2))


### Bug Fixes

* 5 production bugs reported by the first user session ([#34](https://github.com/Calixteair/kalidoku/issues/34)) ([46e7c53](https://github.com/Calixteair/kalidoku/commit/46e7c53a3e2aef18059e792fddd584dae75ed4d4))
* **bao:** use legacy octal 0600 in agent.hcl ([#1](https://github.com/Calixteair/kalidoku/issues/1)) ([1f831b5](https://github.com/Calixteair/kalidoku/commit/1f831b5021d0e1a15777f02cc9a2d64c9344d396))
* **infra:** pass VALKEY_PASSWORD to valkey container env ([#6](https://github.com/Calixteair/kalidoku/issues/6)) ([2fc51e3](https://github.com/Calixteair/kalidoku/commit/2fc51e3a7119acce48a9ae681cf72e265c2293c9))
* **server,deps:** remove unused openidconnect crate (drops vuln rustls 0.21 chain) ([#44](https://github.com/Calixteair/kalidoku/issues/44)) ([46f65da](https://github.com/Calixteair/kalidoku/commit/46f65da557ee22bcd1ba77ef39fde2e35e3a42ab))
* **server,web,infra:** production fixes for MVP deploy ([#33](https://github.com/Calixteair/kalidoku/issues/33)) ([ba8c2fa](https://github.com/Calixteair/kalidoku/commit/ba8c2fae1b794254a22c85b8f56f6c84d3190a7a))
* **server,web:** allow anonymous daily play + bilingual header/footer ([#35](https://github.com/Calixteair/kalidoku/issues/35)) ([032c663](https://github.com/Calixteair/kalidoku/commit/032c663ff4dd29f3d783043fc780d6ba84e57b8f))
* **server:** expose peer IP to rate-limit via connect_info ([#32](https://github.com/Calixteair/kalidoku/issues/32)) ([a9a5a93](https://github.com/Calixteair/kalidoku/commit/a9a5a936d9b554a972cc8ee3b2e53e071187e417))
* **server:** make scaffold actually start ([#5](https://github.com/Calixteair/kalidoku/issues/5)) ([263a051](https://github.com/Calixteair/kalidoku/commit/263a051450ce7b01c466db573504a9d695562207))
* **server:** persist device row + reuse in OIDC callback ([#37](https://github.com/Calixteair/kalidoku/issues/37)) ([dfe378c](https://github.com/Calixteair/kalidoku/commit/dfe378c8ba916750501e8a23b65e3ebd5cebfe29))
* **server:** upsert device row on every request to prevent FK errors ([#36](https://github.com/Calixteair/kalidoku/issues/36)) ([e7447d8](https://github.com/Calixteair/kalidoku/commit/e7447d8fa6ecc59ae669d89801ff4248a398d8cb))
* **web:** bump nginx to 1.30-alpine3.23 (CVE-2025-15467) ([#4](https://github.com/Calixteair/kalidoku/issues/4)) ([008e78f](https://github.com/Calixteair/kalidoku/commit/008e78f103c169ef68d2ce67defc396036138a42))
* **web:** use plugin-message-format@2 (old URL was 404) ([#2](https://github.com/Calixteair/kalidoku/issues/2)) ([a1c11d5](https://github.com/Calixteair/kalidoku/commit/a1c11d5f368f99c2b64b2319ccbf1af782c59632))
