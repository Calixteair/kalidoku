# Changelog

## [1.1.0](https://github.com/Calixteair/kalidoku/compare/v1.0.0...v1.1.0) (2026-05-19)


### Features

* clash-royale card art (PR B) ([#68](https://github.com/Calixteair/kalidoku/issues/68)) ([095e090](https://github.com/Calixteair/kalidoku/commit/095e0906b5fd4f3360f2ddf08a2bb6b880f20276))
* **core,domain-paris-metro:** word-count predicates + 30-strong predicate pack ([#52](https://github.com/Calixteair/kalidoku/issues/52)) ([60df2d4](https://github.com/Calixteair/kalidoku/commit/60df2d45ca89de303de661fc88583e8005c7fb8a))
* **domain-clash-royale:** first non-transport domain — 121 cards ([#67](https://github.com/Calixteair/kalidoku/issues/67)) ([f41bd28](https://github.com/Calixteair/kalidoku/commit/f41bd28346743fab06fad147ee4c0d2d0d90a624))
* **domain-rer:** RER d'Île-de-France — first multi-domain ship ([#62](https://github.com/Calixteair/kalidoku/issues/62)) ([652dece](https://github.com/Calixteair/kalidoku/commit/652decee44b96e9441c1467e2cb4841570382e5d))
* **infra:** add Meilisearch service for autocomplete ([#45](https://github.com/Calixteair/kalidoku/issues/45)) ([c53de2c](https://github.com/Calixteair/kalidoku/commit/c53de2cf9eff9078e21a406530e71371a2e1b41c))
* ingest fame_score on paris-metro (Wikipedia pageviews) ([#57](https://github.com/Calixteair/kalidoku/issues/57)) ([4020744](https://github.com/Calixteair/kalidoku/commit/4020744a4d27f3e791827cbdf79a98d8473b6156))
* leaderboard refonte — period tabs + 5 sort keys + aggregate metrics ([#58](https://github.com/Calixteair/kalidoku/issues/58)) ([4b5978e](https://github.com/Calixteair/kalidoku/commit/4b5978e3fb6cdc5993e199c5ffc57e3f81f1a24b))
* line badges, rarity tags, dynamic masthead ([#65](https://github.com/Calixteair/kalidoku/issues/65)) ([74ac4dd](https://github.com/Calixteair/kalidoku/commit/74ac4dda04ae83774dd2b3930370fcdd30a1cc03))
* phase 2 finale — originality score + async duel ([#55](https://github.com/Calixteair/kalidoku/issues/55)) ([95974e9](https://github.com/Calixteair/kalidoku/commit/95974e98cdb70f627f49bccdfaa3cfc1e084fc08))
* phase 2 solo + full UI redesign ([#54](https://github.com/Calixteair/kalidoku/issues/54)) ([ae0ed7e](https://github.com/Calixteair/kalidoku/commit/ae0ed7e7582638ec0d642c30cc162490e314a742))
* **server:** auto-upsert active domains rows on boot ([#63](https://github.com/Calixteair/kalidoku/issues/63)) ([f08085b](https://github.com/Calixteair/kalidoku/commit/f08085b5fea13598792a9bc1cd0cf21bca640ceb))
* **server:** Meilisearch-backed autocomplete endpoint ([#47](https://github.com/Calixteair/kalidoku/issues/47)) ([1ad3ced](https://github.com/Calixteair/kalidoku/commit/1ad3cedd3e7382a6312594edc2e2a73c54096018))
* **web:** domain picker via /[domain]/* routing + hub at / ([#64](https://github.com/Calixteair/kalidoku/issues/64)) ([e82b3d2](https://github.com/Calixteair/kalidoku/commit/e82b3d23da4ed4b97cd6a10896deb2c0348ac1ac))
* **worker:** Meilisearch reindex command + cron integration ([#46](https://github.com/Calixteair/kalidoku/issues/46)) ([9012224](https://github.com/Calixteair/kalidoku/commit/90122246114ff5d0d40cea4d34c193b8fc86d60c))


### Bug Fixes

* cell retry, /play redirect, see-solutions, leaderboard key + UX polish ([#53](https://github.com/Calixteair/kalidoku/issues/53)) ([22efa2d](https://github.com/Calixteair/kalidoku/commit/22efa2d9c7d3abe306f8dd9f0a86f51642201396))
* **domain-rer:** rebuild dataset from frwiki + OSM ([#69](https://github.com/Calixteair/kalidoku/issues/69)) ([dca3165](https://github.com/Calixteair/kalidoku/commit/dca31657e052ce33e62662afc21b877ce6c620d6))
* duel domain plumbing + manual fame_score overrides ([#66](https://github.com/Calixteair/kalidoku/issues/66)) ([21d4bb1](https://github.com/Calixteair/kalidoku/commit/21d4bb1c7c1beacc7da4b72b286f00edd24a3230))
* phase1 polish (auth + play UX + responsive) ([#49](https://github.com/Calixteair/kalidoku/issues/49)) ([68bf105](https://github.com/Calixteair/kalidoku/commit/68bf105772bee76e98cd846e3cb1d3dfa970c5af))
* race on POST /api/games (UNIQUE constraint 500) ([#61](https://github.com/Calixteair/kalidoku/issues/61)) ([f1f0201](https://github.com/Calixteair/kalidoku/commit/f1f0201b8287b03f5c7d40bc61aa680333518e0d))
* **server:** drop unwired Altcha gate on duel start_game ([#60](https://github.com/Calixteair/kalidoku/issues/60)) ([8f2baf1](https://github.com/Calixteair/kalidoku/commit/8f2baf13a86bffc823f106e8dfefac0f8455ee7b))
* **server:** set Secure + Max-Age on kd_device cookie ([#51](https://github.com/Calixteair/kalidoku/issues/51)) ([723cb37](https://github.com/Calixteair/kalidoku/commit/723cb3770d38469b642cf4217452d3f6aa1eaa01))
* **web:** 3 post-phase-2 UX bugs (stale daily, empty /play, autocomplete flicker) ([#56](https://github.com/Calixteair/kalidoku/issues/56)) ([ebf09dd](https://github.com/Calixteair/kalidoku/commit/ebf09dd9050625210800a1845a1a0574cbe80326))
* **web:** broken duel link + create-duel CTA on /duel landing ([#59](https://github.com/Calixteair/kalidoku/issues/59)) ([ac433d9](https://github.com/Calixteair/kalidoku/commit/ac433d9dbe8d49a54a92e06e57e50602e3e4793e))
* **web:** drop ICU plural — paraglide doesn't interpret it ([#50](https://github.com/Calixteair/kalidoku/issues/50)) ([9ebf77f](https://github.com/Calixteair/kalidoku/commit/9ebf77fd20bcbdaa0b01fc6ac0b37c3074c576a7))

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
