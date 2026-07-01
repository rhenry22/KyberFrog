# Intégration Blackmagic DeckLink 4K Pro (PCIe)

Prérequis de la Phase A / item 2 de `VIDEO_MATRIX_MIXER.md` : faire entrer une
carte de capture/sortie DeckLink 4K Pro comme source **et** destination de la
matrice vidéo, au même titre que Kyber et Spout local.

## Contexte matériel

- **Carte** : Blackmagic DeckLink 4K Pro, 4 sous-devices, chacun configurable
  indépendamment en capture ou en sortie (I/O simultané).
- **État** : driver **Desktop Video déjà installé** et carte **fonctionnelle**
  sur la machine régie (testée via les outils Blackmagic, ex. Media Express).
  Pas de travail de prérequis matériel à prévoir dans ce chantier — on part
  directement sur l'intégration logicielle.
- **Usage visé** : les 4 sous-devices exploitables, sans figer un mapping
  fixe (ex: "2 capture + 2 sortie") — la config doit permettre d'assigner
  n'importe quel sous-device à un rôle capture ou sortie, comme n'importe
  quelle autre source/destination de la matrice.

## Choix d'architecture

### FFmpeg (`libavdevice` decklink) plutôt que bindings SDK directs
On pilote la carte via **FFmpeg compilé avec le support DeckLink**
(`-f decklink` en entrée et en sortie), pas via des bindings Rust maison sur
le SDK C++ Blackmagic.

- **Why** : cohérent avec le principe déjà adopté pour le mélangeur
  ("déléguer à un moteur existant" plutôt que réimplémenter — cf.
  `VIDEO_MATRIX_MIXER.md` §Mélangeur). FFmpeg gère déjà l'énumération des
  sous-devices, la négociation de format et le threading capture/sortie ;
  écrire des bindings FFI sur le SDK COM-like de Blackmagic serait un travail
  Win32 substantiel pour un résultat que FFmpeg couvre déjà.
- **Trade-off accepté** : latence et contrôle un cran moins fins qu'un accès
  SDK direct (pas de contrôle frame-exact, pas d'accès aux features avancées
  du SDK type genlock fin). Acceptable pour un usage régie VJ, à revisiter si
  un besoin de sync broadcast précis apparaît.
- **How** : vérifier que le binaire FFmpeg utilisé par le projet (celui du
  fork Kyber, ou un binaire système) est bien compilé avec
  `--enable-decklink` — sinon `-f decklink` échoue silencieusement à
  l'énumération des devices. `ffmpeg -f decklink -list_devices 1 -i dummy`
  doit lister les 4 sous-devices une fois validé.

### Pont capture → Spout : extension de `kyavserver`
Le flux capturé par FFmpeg doit rejoindre le bus interne commun de la matrice
(Spout local, cf. `VIDEO_MATRIX_MIXER.md` §3c). Plutôt qu'un nouveau binaire
pont, on **étend `kyavserver`** (composant du fork Kyber) pour qu'il accepte
un flux DeckLink/FFmpeg en entrée, en plus de son rôle actuel de publication
Spout pinnée.

- **Why réutiliser `kyavserver`** : c'est déjà le composant du fork qui sait
  faire du Spout output avec pinning d'id (`spout_sender` = CRC-32 IEEE, cf.
  `CLAUDE.md`). Un second binaire ferait doublon sur cette responsabilité et
  imposerait de réimplémenter le pinning Spout ailleurs.
- **How** : `kyavserver` gagne un mode d'entrée alternatif (pipe FFmpeg ou
  named pipe Windows) en plus de son entrée actuelle, en sortie il republie
  en Spout exactement comme aujourd'hui. Modification côté **fork Kyber**
  (pas dans ce repo KyberFrog) — à porter dans `kymedia`/`txproto` selon où
  vit `kyavserver` aujourd'hui.
- **Chaîne complète (capture)** :
  `DeckLink sous-device → ffmpeg -f decklink → pipe → kyavserver (nouveau mode) → Spout sender → matrice`

### Sortie (DeckLink comme destination)
Symétrique : une source de la matrice (Spout local, viewer Kyber, ou sortie
du mélangeur Resolume) peut être routée vers un sous-device DeckLink en
sortie via `ffmpeg -f decklink` consommant un flux Spout (ou directement la
sortie `kyavserver`/Resolume selon ce qui est le plus direct — **point ouvert**,
cf. ci-dessous).

## Supervision côté KyberFrog

Chaque paire "sous-device DeckLink + rôle (capture/sortie)" actif devient un
processus de plus supervisé par le `Manager` existant (`supervisor.rs`),
suivant le pattern déjà en place pour `kycontroller`/`kyclient` :

- Nouvelle variante `Key::Capture(sub_device_id)` / `Key::Output(sub_device_id)`
  dans l'enum `Key` typé (pas de string matching, cf. convention du projet).
- `Spec` dédié : binaire = ffmpeg (ou wrapper), args générés à partir de la
  config (sous-device, format, rôle), même politique de restart/backoff que
  les autres enfants.
- Statut exposé dans le `StatusMap` commun, visible dans la future control
  surface (`VIDEO_MATRIX_MIXER.md` §5).

## Modèle de config (`shared`)

Nouveau type dans `shared/src/matrix.rs` (à côté de `Source`/`Destination`
introduits par `VIDEO_MATRIX_MIXER.md` Phase A item 1) :

```toml
[[capture.decklink_device]]
sub_device = 0
role = "capture"   # ou "output"
format = "1080p59.94"   # à valider contre ce que la carte/FFmpeg négocie
```

- **Why un tableau flexible plutôt que 4 champs fixes** : respecte la
  décision "mapping libre, pas figé" — un sous-device peut changer de rôle
  sans migration de schéma.

## Backlog (rattaché à Phase A de `VIDEO_MATRIX_MIXER.md`)

1. **Valider le binaire FFmpeg** utilisé par le projet est compilé avec
   `--enable-decklink` ; sinon, l'ajouter au pipeline de build (image Docker
   `kyber/debian-win64:local` ou binaire embarqué séparé — à trancher, la
   carte étant Windows-only ce n'est pertinent que côté target Windows).
2. **Extension `kyavserver`** (fork Kyber, hors ce repo) : entrée alternative
   FFmpeg/pipe en plus du mode actuel.
3. **Modèle `capture.decklink_device`** dans `shared` + validation (un
   sous-device ne peut pas être capture ET sortie en même temps).
4. **Supervision** : nouveau `Spec`/`Key` dans `supervisor.rs` pour les
   process ffmpeg de capture/sortie.
5. **Test end-to-end** : un sous-device en capture → Spout → visible dans
   Resolume ou re-émis en Kyber ; un sous-device en sortie recevant la sortie
   du mélangeur.

## Points ouverts

- **Sortie DeckLink** : consommer directement la sortie Resolume (si Resolume
  peut driver du SDI nativement) ou repasser par Spout → ffmpeg → decklink ?
  Impacte la latence et la nécessité ou non de toucher `kyavserver` pour la
  sortie aussi.
- **Formats/résolutions supportés** : à valider empiriquement avec
  `ffmpeg -f decklink -list_formats 1` sur la machine régie une fois le
  binaire FFmpeg confirmé compatible.
- **Nommage des sous-devices** : FFmpeg les énumère par index runtime, pas par
  identifiant stable — vérifier si Blackmagic expose un id stable (numéro de
  port physique) pour éviter qu'un remap matériel casse la config.
