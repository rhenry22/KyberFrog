# Clé USB d'installation — Ubuntu 24.04 + drivers Blackmagic + logiciel maison

Prépare une clé USB qui installe **Ubuntu 24.04 Server** avec le même
assistant que l'ISO officielle (aucune étape ajoutée/retirée) et
préinstalle automatiquement, juste après le tronc commun de l'installeur :

- le driver **Blackmagic Desktop Video** (module DKMS, compatible DeckLink
  4K Pro, Mini Recorder, DeckLink Studio 2 — même paquet driver pour les
  trois cartes) ;
- le(s) `.deb` du logiciel maison.

Mécanisme : partition FAT32 `CIDATA` ajoutée en fin de clé (datasource
cloud-init NoCloud), lue par Subiquity. `interactive-sections: "*"` dans
`user-data` garde tout l'assistant interactif ; seuls des `late-commands`
s'exécutent en plus, après l'installation de base.

## Usage

1. Télécharger l'ISO `ubuntu-24.04.x-live-server-amd64.iso`
   (ubuntu.com/download/server).
2. Télécharger le driver **Desktop Video pour Linux** depuis le site
   Blackmagic (Support > Capture and Playback), en extraire les `.deb`
   (`desktopvideo_*.deb`, `desktopvideo-dkms_*.deb`), et les copier avec le(s)
   `.deb` du logiciel maison dans `installer/extra-debs/` (non versionnés,
   cf. `.gitignore` — paquets gated par licence / binaires volumineux).
3. Copier `VERSION` vers `VERSION.local` et adapter `UBUNTU_ISO`,
   `EXTRA_DEBS_DIR`, etc.
4. Identifier la clé cible avec `lsblk`, puis :

   ```sh
   ./installer/build-usb.sh /dev/sdX installer/VERSION.local
   ```

   Le script demande confirmation (retaper le device) avant tout `dd` —
   opération destructive, efface tout le disque.
5. Au boot sur la clé, sur l'entrée GRUB "Try or Install Ubuntu Server",
   presser `e` et ajouter `autoinstall` en fin de ligne kernel avant de
   démarrer (déclenche la lecture de `user-data`).

## Secure Boot

Le module DKMS Blackmagic n'est pas signé par Canonical. Si Secure Boot est
actif sur la machine cible, un écran bleu **MOK Management** apparaît au
premier reboot après l'install — valider avec le mot de passe défini dans
`VERSION` (`MOK_PASSWORD`), une seule fois. Alternative : désactiver Secure
Boot dans le BIOS pour l'éviter complètement.

## Limites connues

- Le chemin de montage de la partition `CIDATA` pendant l'installeur
  (`/cdrom` dans `late-commands`) est à vérifier une fois en conditions
  réelles (Ctrl+Alt+F2 pendant l'install) et à ajuster si Subiquity la monte
  ailleurs.
- Le driver Blackmagic est aujourd'hui exploité côté machine régie
  **Windows** (cf. `Blackmagic_PCie_int.md`) — cette clé sert à un poste
  Linux séparé ; l'intégration logicielle (FFmpeg `-f decklink`, pont
  `kyavserver`) décrite dans `Blackmagic_PCie_int.md` reste à confirmer côté
  Linux si ce poste doit s'intégrer à la matrice vidéo.
