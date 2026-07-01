# Intégration Linux — kyclient/kycontroller pour kyberfrog

Suivi de l'enquête sur la disponibilité d'un binaire `kyclient`/`kycontroller`
Linux x86_64 utilisable par `kyberfrog` (cf. limite déjà notée dans
`packaging/linux/README.md`). *What/why/how*, même format que
`VIDEO_MATRIX_MIXER.md`.

## État actuel

`kyberfrog` compile et tourne nativement sur Ubuntu 24.04 (supervisor + web
UI), mais tout viewer échoue au démarrage :

```
[viewer-1] failed to spawn kyclient: No such file or directory (os error 2)
```

Aucun binaire `kyclient`/`kycontroller`/`kyavserver` n'était présent sur la
machine, ni dans aucun paquet `apt`, ni publié en Release GitLab connue.

## Sous-modules ajoutés (`third_party/`)

- **`third_party/kyctl`** (`git@gitlab.com:kyber-frog/kyctl.git`) — contient
  **et** `kyclient` **et** `kycontroller` (crates séparées, `Cargo.toml` à la
  racine + un par binaire). A un job CI
  `kyclient-build-debian-trixie-x86_64` (`.gitlab-ci-kyclient.yml`) qui prouve
  qu'un build Debian/x86_64 existe **dans leur pipeline** — donc la cible est
  atteignable, mais rien n'indique qu'un artefact en soit publié quelque part
  d'accessible.
- **`third_party/kyber-desktop`** (`git@gitlab.com:kyber-frog/kyber-desktop.git`)
  — gardé pour son script `scripts/linux/run_kyclient.sh` (wrapper qui
  positionne `PATH`/`LD_LIBRARY_PATH` autour d'un `kyclient` déjà construit).
  Pas une source de build en soi.

Initialisation : `make git-submodules` (cible ajoutée au `Makefile` racine).

## Chemin de build identifié — et son blocage

`third_party/kyctl/build-linux.sh` sait construire les deux binaires :

```sh
./build-linux.sh -k build-kyclient      # -k = va chercher les dépendances sources
./build-linux.sh build-kycontroller     # kycontroller n'a pas besoin des deps natives (-s implicite)
```

**Bloqué en pratique** : l'option `-k` (`checkout_deps`) clone en repos frères
`core/kymux`, `core/kyutil`, `core/kynput`, `core/kymedia` — un groupe GitLab
**`core`, différent de `kyber-frog`**, auquel ce compte (`romain.henry.22`)
n'a **pas accès** :

```
ERROR: The project you were looking for could not be found or you don't have permission to view it.
```

Même blocage rencontré sur les sous-modules imbriqués de `kyber-desktop`
(`core/kysdk`) et sur `deps/winit` — laisse penser que `core/*` et `deps/*`
sont des groupes GitLab à accès restreint, séparés de `kyber-frog`.

`kycontroller` seul (`build-kycontroller`, sans `-k`) ne dépend pas de ces
libs natives — **à tenter en premier**, c'est potentiellement débloqué dès
maintenant contrairement à `kyclient`.

## Licence

`build-linux.sh` porte l'en-tête *"SPDX-License-Identifier:
LicenseRef-Kyber-Commercial OR AGPL-3.0"* — double licence commerciale/AGPL,
à garder en tête si `kyctl` est un jour redistribué avec un build kyberfrog
Linux (contrairement à `kyberfrog` lui-même, purement AGPL-3.0).

## Prochaines étapes possibles

1. **Tenter `build-kycontroller` seul** (pas de blocage `core/*` connu) —
   validerait au moins l'émission côté Linux.
2. **Demander l'accès au groupe GitLab `core`** (et `deps` pour
   `kyber-desktop`) — probablement une démarche humaine (accès équipe), pas
   quelque chose de contournable techniquement.
3. **Chercher un artefact déjà publié** par le job CI
   `kyclient-build-debian-trixie-x86_64` (Releases/CI artifacts du repo
   `kyctl`) — à vérifier une fois l'accès `core/*` clarifié, certains
   artefacts CI sont parfois téléchargeables même sans accès aux dépendances
   sources.
4. Sans l'un des deux chemins ci-dessus, la réception réelle d'un flux Kyber
   sur Linux reste **hors de portée** — `kyberfrog` sur Ubuntu reste limité au
   supervisor/web UI/édition de config, comme documenté dans
   `packaging/linux/README.md`.
