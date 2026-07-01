# Build Ubuntu 24.04 — AppImage

Contrepartie Linux de `packaging/build-installer.sh` (Windows/NSIS). Produit
un `KyberFrog-<ver>-x86_64.AppImage` : un seul fichier exécutable, sans
installation système — pour itérer les versions en dev, il suffit de
remplacer le fichier (pas de désinstallation, pas de droits root).

> **Limite connue** : ce paquet contient uniquement le binaire `kyberfrog`
> (supervisor + web UI). Il n'embarque **pas** `kycontroller`/`kyavserver`/
> `kyclient` (binaires du fork Kyber) — un build Linux existe côté CI du fork
> (`kyclient-build-debian-trixie-x86_64` dans `third_party/kyctl`), mais sa
> compilation locale est bloquée par un accès GitLab manquant au groupe
> `core` (dépendances natives de `kyclient`) ; voir le détail et les
> prochaines pistes dans `LINUX_INTEGRATION.md`. Tant que ce n'est pas
> débloqué, l'émission/réception réelle ne fonctionnera pas sur Ubuntu ; la
> web UI, le supervisor et l'édition de config restent testables seuls. Le
> tray (Win32-only) et Spout (Windows-only) sont déjà en no-op stub sur Linux
> — cf. `CLAUDE.md` §"Cross-platform module pattern".

## Packages Ubuntu 24.04 nécessaires

Contrairement au build Windows (cross-compilé via l'image Docker MinGW), ce
build est **natif** : on compile directement sur la machine Ubuntu, pas de
cross-compilation.

```sh
sudo apt update
sudo apt install -y \
    build-essential \
    pkg-config \
    curl \
    git \
    ca-certificates \
    file \
    desktop-file-utils \
    libfuse2t64
```

- **`build-essential`** : `cc`/linker requis par `cargo` pour lier le binaire
  natif (`x86_64-unknown-linux-gnu`), même si le workspace n'a pas de
  dépendances C directes.
- **`pkg-config`** : présent par précaution — aucune dépendance du workspace
  ne requiert de lib système aujourd'hui (`Cargo.lock` ne référence ni
  OpenSSL ni rustls/native-tls), mais des crates futures (TLS, OSC/audio pour
  le mélangeur vidéo) pourraient en avoir besoin.
- **`curl`, `git`, `ca-certificates`** : installer `rustup`/le toolchain Rust
  et laisser `build.rs` faire son `git describe` pour la version (cf.
  `kyberfrog/build.rs`).
- **`file`, `desktop-file-utils`** : utilisés par `appimagetool` pour valider
  l'AppDir (fichier `.desktop`, détection de type MIME du binaire).
- **`libfuse2t64`** : requis pour **exécuter** un AppImage sur Ubuntu 24.04
  (qui n'a plus FUSE2 par défaut) — nécessaire à la fois pour lancer
  `appimagetool` (lui-même distribué en AppImage) et pour lancer le
  `.AppImage` produit. Sur une machine où l'AppImage final doit tourner sans
  ce paquet, `KyberFrog.AppImage --appimage-extract-and-run` contourne le
  besoin de FUSE.

**Rust toolchain** (pas via `apt rustc`, trop ancien) :

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup target add x86_64-unknown-linux-gnu   # target par défaut sur Ubuntu x86_64, déjà présent normalement
```

**Node.js** (pour builder la web UI React, `ui/`) — via nvm ou le binaire
officiel nodejs.org, pas le paquet `apt nodejs` (souvent obsolète) :

```sh
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
```

**`appimagetool`** (téléchargé une fois, mis en cache à côté du script) :

```sh
curl -L -o packaging/linux/.appimagetool-x86_64.AppImage \
    https://github.com/AppImage/appimagetool/releases/latest/download/appimagetool-x86_64.AppImage
chmod +x packaging/linux/.appimagetool-x86_64.AppImage
```

## Build

```sh
bash packaging/linux/build-appimage.sh
```

Produit `dist/KyberFrog-<ver>-x86_64.AppImage`. Pour itérer :

```sh
chmod +x dist/KyberFrog-<ver>-x86_64.AppImage
./dist/KyberFrog-<ver>-x86_64.AppImage
```

Rebuild + relance = juste refaire les deux lignes ci-dessus avec le nouveau
fichier ; l'ancien peut rester à côté (utile pour comparer deux versions).

## À faire avant le premier build réussi

- **Icône PNG manquante** : seul `kyberfrog/assets/kyberfrog.ico` existe
  aujourd'hui (utilisé par l'installeur Windows). `build-appimage.sh` détecte
  l'absence de `kyberfrog/assets/kyberfrog.png` et continue sans icône
  (avertissement) — exporter un PNG 256×256 depuis le `.ico` pour un rendu
  propre dans les launchers Linux.
- **Vérifier les binaires du fork Kyber sous Linux** (cf. limite connue
  ci-dessus) avant de considérer ce paquet comme un vrai déploiement
  fonctionnel plutôt qu'un shell UI/supervisor.
