# Runbook : revenir à une release précédente

## Quand

- Le contrôle de santé de `stack-sync` a échoué : il est **déjà** revenu tout seul à la
  configuration précédente (journal : `journalctl -t stack-sync`). Rien à faire, corriger puis
  publier une nouvelle release.
- La release en place est saine pour Docker mais fautive pour les utilisateurs.

## Procédure

1. Release en place et releases disponibles :

   ```bash
   ssh vps-claude 'cat /home/calixteair/docker/projects/kalidoku/.stack-sync'   # tag, commit, date
   git tag -l 'v*' --sort=-v:refname | head
   ```

2. Redéployer la release précédente. Ses images `vX.Y.Z-*` sont immuables dans Harbor, et sa
   configuration (`infra/`) est relue au tag, dont la signature est vérifiée :

   ```bash
   ssh vps-claude 'sudo /usr/local/sbin/stack-sync.sh kalidoku vX.Y.Z'
   ```

3. Vérifier :

   ```bash
   curl -fsS https://kalidoku.calixteair.fr/api/health
   ssh vps-claude 'sudo docker ps --format "{{.Names}} {{.Image}} {{.Status}}" | grep kalidoku'
   ```

## Migrations de base

Un retour arrière ne défait pas les migrations SQL. Si la release fautive en a appliqué une
incompatible avec la précédente, restaurer la base depuis la sauvegarde (dump nocturne) avant
l'étape 2, ou publier une release corrective plutôt que revenir en arrière.
