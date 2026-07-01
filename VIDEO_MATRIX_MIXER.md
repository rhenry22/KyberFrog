# KyberFrog — Matrice vidéo & Mélangeur (vision & backlog)

Ce document cadre l'**Amélioration 2bis** : une matrice de routing vidéo
unifiée (Kyber + Spout + capture PCIe) et un mélangeur (compositing
multi-layers) qui s'appuie dessus. Même format que `IMPROVEMENTS.md` : *what*,
*why*, *how*. Numéros stables une fois le chantier démarré.

> Ne pas confondre avec l'étape 2 déjà planifiée dans `CLAUDE.md`
> ("Spout output from kyclient") : ce document va plus loin — matrice N×N et
> mélangeur — et peut la réutiliser comme brique de base (un viewer qui sort en
> Spout est une source de plus dans la matrice).

## Vision

Aujourd'hui KyberFrog sait émettre et recevoir des flux Kyber point à point.
L'objectif est d'ajouter une couche de **routing** (n'importe quelle source →
n'importe quelle destination, comme un routeur SDI logiciel) et une couche de
**mixing** (composer plusieurs sources en une sortie, via Resolume Arena déjà
présent côté régie). Les deux briques restent **distinctes** :

- **Matrice** : registre de sources et de destinations, table de routage,
  aucune notion de compositing.
- **Mélangeur** : une des destinations possibles de la matrice ; une instance
  de mélangeur a plusieurs entrées (layers) prises dans la matrice et produit
  une sortie qui redevient elle-même une source de la matrice.

```
[Sources]                [Matrice]              [Destinations]
 Kyber viewer  ─┐                          ┌─▶ kyclient / affichage direct
 Spout local    ─┼──▶  table de routage  ──┼─▶ Mélangeur (Resolume) ─▶ nouvelle source
 Capture BM PCIe─┘                          └─▶ Kyber transmitter (ré-émission)
```

## Composants

### 1. `kyfrogmix` — nouveau binaire supervisé
Un 4ᵉ type de processus, au même titre que `kycontroller`/`kyavserver`/
`kyclient`, ajouté au `Manager` de `supervisor.rs`. Rôle : piloter Resolume
Arena (OSC + éventuellement son API REST) pour appliquer la configuration de
layers décidée par l'opérateur, et exposer un statut (Running/Stopped/erreur
de connexion OSC) dans le `StatusMap` existant (`Key::Mix(name)` à ajouter à
l'enum `Key`).

- **Why un binaire séparé plutôt qu'intégré à `kyclient`** : Resolume est un
  process externe déjà installé côté régie ; `kyfrogmix` n'est qu'un traducteur
  "config KyberFrog → commandes OSC Resolume", pas un moteur de rendu. Le
  garder séparé respecte le pattern "un binaire = une responsabilité" déjà en
  place et permet de superviser sa reconnexion OSC indépendamment.
- **How** : un client OSC (crate `rosc` ou équivalent), une correspondance
  déclarative "layer KyberFrog → layer/clip Resolume" dans le TOML, un
  supervise-loop dans `supervisor.rs` réutilisant le `Spec` existant (pas de
  process enfant à spawn ici puisque Resolume tourne déjà — plutôt un mode
  "sidecar" sans Job Object).

### 2. La matrice de routing (dans `shared`)
Nouveau modèle dans `shared/src/matrix.rs` : `Source { id, kind: Kyber|Spout|
CaptureCard, .. }`, `Destination { id, kind: KyclientDisplay|MixerInput|
KyberReemit }`, et une table `routes: Vec<(SourceId, DestinationId)>`
persistée dans `kyberfrog.toml` aux côtés de `emission`/`reception`.

- **Why dans `shared`** : c'est la même logique que `Config` — un modèle de
  données pur, testable sans Windows, chargé/sauvé une seule fois.
- **How** : réutiliser `Key`-style typed ids pour éviter les erreurs de string
  matching déjà évitées ailleurs dans le projet. Le routing n'est qu'une table
  de correspondance ; la validation (une source ne peut nourrir un mélangeur
  que si elle expose un flux compatible) est une fonction pure testable comme
  `gen.rs`.

### 3. Sources de la matrice

#### 3a. Kyber viewer comme source
Un viewer existant (`kyclient`) republié en Spout local (dépend de
l'Amélioration 2 déjà planifiée dans `CLAUDE.md`) devient automatiquement une
source disponible pour la matrice.

#### 3b. Spout local
Réutilise `spout.rs` (déjà utilisé pour le picker "Add" transmitter) comme
énumération de sources — mais ici en lecture continue, pas juste un instantané
pour un formulaire.

#### 3c. Capture Blackmagic (PCIe)
Nouvelle source hardware. Cf. `Blackmagic_PCie_int.md` (actuellement vide — à
remplir en parallèle avec les specs DeckLink SDK : device enum, format de
capture, publication en Spout local pour rester dans le même bus que 3a/3b
plutôt que d'inventer un chemin de données parallèle).

- **Why republier en Spout plutôt qu'un chemin dédié** : une seule
  représentation interne de "flux vidéo local" (Spout) simplifie la matrice —
  toute source finit en Spout avant d'entrer dans le routing, qu'elle vienne
  de Kyber, d'une capture, ou d'une app tierce.
- **How** : un petit binaire ou module utilisant le DeckLink SDK (capture) →
  Spout sender, supervisé comme les autres (`Key::Capture(name)`).

### 4. Sortie du mélangeur
Le résultat composité par Resolume ressort par les deux chemins déjà maîtrisés
par le projet :
- **Spout local** (Resolume le fait nativement) → redevient une source de la
  matrice, disponible pour un nouveau routing (ex: preview locale).
- **Flux Kyber (QUIC)** : un `kycontroller` dont la source Spout pointe vers la
  sortie Resolume — réutilise tel quel le mécanisme `spout_sender` pinning
  déjà en place dans `gen.rs`, aucun nouveau code de transport nécessaire.

### 5. Interface dédiée (control surface)
Une UI **séparée** du dashboard web 7700 (pas un onglet) :

- **Why séparée** : le dashboard actuel est un outil de supervision/config
  (latence de rafraîchissement acceptable) ; un control-surface de mixage a des
  besoins de réactivité et d'ergonomie différents (drag & drop de routing,
  preview vidéo live, va au clavier/MIDI potentiellement) qui justifient un
  binaire/process front dédié plutôt que de complexifier `web/index.html`.
- **What** : deux vues/onglets **séparés** (pas de combiné) — `Routing`
  (patchbay sources × destinations) et `Mélangeur` (layers, PGM/PST,
  transitions). L'opérateur navigue explicitement entre les deux plutôt que
  d'ouvrir la matrice en surimpression du mélangeur.
- **Cible device** : écran tactile dédié régie, format compact **10-15"
  (≤1920×1080)**. Contrainte de design : setup réel prévu à l'échelle
  **4-8 layers / 8-16 sources**, donc plus de contenu que d'espace — la
  densité se gère par **groupement + scroll par catégorie**, pas en réduisant
  la taille des zones tactiles (qui doivent rester ≥120×80px, pensées pour le
  doigt, pas la souris précise). Pas de vignettes vidéo live en V1 (texte/
  icônes suffisent) — le rendu live viendra dans un chantier séparé une fois
  la charge GPU/réseau évaluée.
- **Style de référence** : hybride — densité proche d'un logiciel VJ
  (Resolume/TouchDesorer) pour la pile de layers et le picker de sources,
  mais interactions **one-tap** (pas de drag précis exigé) pour rester
  fiable au doigt sur écran compact.

#### Vue Mélangeur (mockup)

```
┌─────────────────────────────────────────────────────────────────┐
│ [ROUTING]  [MÉLANGEUR]                          ● REC  ⚙ Config │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────┐   ┌───────────────┐                          │
│  │               │   │               │      TRANSITION          │
│  │      PGM      │ ▶ │      PST      │   ┌───┬───┬───┬───┐      │
│  │   (en sortie)  │   │  (prochain)   │   │CUT│MIX│WIPE│STING│  │
│  │               │   │               │   └───┴───┴───┴───┘      │
│  └───────────────┘   └───────────────┘   ┌─────────────────┐    │
│                                            │  ▓▓▓▓▓░░░░░░░░ │ T-bar│
│                                            └─────────────────┘    │
├─────────────────────────────────────────────────────────────────┤
│ LAYERS (empilement, haut = dessus)              [+ Layer]        │
│ ┌────┬──────────────┬────────┬──────┬────────┬────┬──────────┐  │
│ │ 👁 │ Layer 4 (top)│ Src: ▾ │ Blend│ Opacity│ FX │  ⋮ (drag) │  │
│ │ 👁 │ Layer 3       │ Src: ▾ │ Blend│ Opacity│ FX │  ⋮        │  │
│ │ 👁 │ Layer 2       │ Src: ▾ │ Blend│ Opacity│ FX │  ⋮        │  │
│ │ 👁 │ Layer 1 (bg)  │ Src: ▾ │ Blend│ Opacity│ FX │  ⋮        │  │
│ └────┴──────────────┴────────┴──────┴────────┴────┴──────────┘  │
├─────────────────────────────────────────────────────────────────┤
│ SOURCES (groupées par type, scroll horizontal dans chaque ligne) │
│ Kyber   ▶ [Regie-1][Regie-2][Regie-3]···                         │
│ Spout   ▶ [Resolume-out][App-X]···                                │
│ Capture ▶ [BM-Sub0][BM-Sub1][BM-Sub2][BM-Sub3]                    │
└─────────────────────────────────────────────────────────────────┘
```

**Rationale des interactions :**
- Bouton **Source → tap → tap layer cible** : le drag-and-drop reste possible
  à la souris mais n'est jamais requis au doigt — chaque assignation a un
  chemin one-tap.
- Le sélecteur `Src: ▾` par ligne de layer duplique l'accès (picker plein
  écran, sources groupées) pour changer de source sans redescendre chercher
  dans la bande du bas — utile une fois la pile scrollée hors vue.
- `👁` = mute/unmute rapide sans retirer le layer de la pile (garde le mapping
  OSC Resolume intact pendant un mute).
- T-bar glissable + boutons CUT/MIX/WIPE/STING comme raccourcis one-tap qui
  animent le T-bar automatiquement — évite de dépendre d'un geste de glisse
  précis en usage live sous pression.
- Liste de layers scrollable verticalement au-delà de ~5 lignes visibles
  (couvre le setup cible de 8 layers max).

#### Vue Routing (mockup)

```
┌─────────────────────────────────────────────────────────────────┐
│ [ROUTING]  [MÉLANGEUR]                                            │
├─────────────────────────────────────────────────────────────────┤
│              │ Kyclient-1 │ Kyclient-2 │ Mixer L1 │ Mixer L2 │... │
│──────────────┼────────────┼────────────┼──────────┼──────────┼───│
│ Regie-1(Kyb) │     ●      │            │          │          │   │
│ Regie-2(Kyb) │            │     ●      │          │          │   │
│ Resolume-out │            │            │     ●    │          │   │
│ BM-Sub0(Cap) │            │            │          │    ●     │   │
│ BM-Sub1(Cap) │            │            │          │          │   │
├─────────────────────────────────────────────────────────────────┤
│ Filtre : [Toutes ▾] [Kyber] [Spout] [Capture]      Recherche 🔍  │
└─────────────────────────────────────────────────────────────────┘
```

- Grille patchbay classique : **tap sur une cellule** crée/casse la route
  (même logique one-tap que la vue Mélangeur, pas de drag requis).
- Filtres par type de source en bas pour réduire la grille visible quand
  16 sources × destinations dépasse l'écran compact — évite un double scroll
  (horizontal + vertical) simultané.
- Une colonne `Mixer Ln` apparaît automatiquement par layer actif du
  mélangeur : router une source vers `Mixer L2` ici est **strictement
  équivalent** à choisir la même source via `Src: ▾` dans la vue Mélangeur —
  les deux vues écrivent dans la même table de routage (§2), à garder en tête
  comme contrainte d'implémentation (une seule source de vérité, deux entrées
  UI).

- **How (à trancher plus tard)** : soit une deuxième app web servie sur un
  port séparé par `kyfrogmix` (réutilise axum, cohérent avec l'existant), soit
  un client natif si la latence de preview vidéo l'exige. Documenter les deux
  dans la MR d'archi avant de coder.

## Backlog priorisé

### Phase A — fondations matrice
1. **Modèle `shared::matrix`** — `Source`/`Destination`/table de routage +
   persistance TOML + tests (pas de Windows requis, testable sur l'hôte Linux
   comme `config.rs`).
2. **Capture Blackmagic → Spout** — dépend des specs dans
   `Blackmagic_PCie_int.md` (à écrire). Bloquant pour toute source hardware.
3. **Kyber viewer → Spout local** — c'est l'Amélioration 2 de `CLAUDE.md`,
   prérequis pour que 3a alimente la matrice.

### Phase B — mélangeur
4. **`kyfrogmix` binaire + client OSC Resolume** — traduction config →
   commandes OSC, statut dans `StatusMap`.
5. **Sortie mélangeur → Kyber** — config `kycontroller` pointant sur le Spout
   sender de sortie Resolume (réutilise `gen.rs` tel quel).

### Phase C — interface
6. **Control surface dédiée** — vue matrice (patchbay) + vue mélangeur
   (PGM/PST, transitions). Décider web séparé vs natif après un prototype bas
   du Phase A/B.

### Ouvert / à trancher
- Granularité du contrôle Resolume : piloter des *clips* individuels ou
  seulement l'assignation de *layers* ? Impacte le mapping OSC.
- Limite du nombre de sources simultanées (perf Spout + réseau QUIC) — à
  mesurer une fois Phase A posée.
- MIDI/clavier pour la control surface : hors scope Phase C initiale, à
  documenter séparément si demandé.
- **Recherche/filtre sources (vue Mélangeur)** : le scroll horizontal par
  catégorie suffit-il à 16 sources sur écran compact, ou faut-il un champ de
  recherche texte comme celui déjà prévu dans la vue Routing ?
- **T-bar tactile** : la garder glissable (vrai slider tactile) ou se limiter
  aux boutons CUT/MIX/WIPE/STING (geste plus sûr au doigt, moins précis) ?
- **Panneau FX par layer** : le bouton `FX` ouvre-t-il un panneau dédié
  (rotation d'effets Resolume) dès la V1, ou reste-t-il hors scope de la
  Phase C initiale ?
