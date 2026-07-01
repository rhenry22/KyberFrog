#!/usr/bin/env bash
# build-usb.sh — prépare une clé USB Ubuntu Server 24.04 (autoinstall, wizard 100% inchangé)
# avec préinstallation des drivers Blackmagic DeckLink + logiciel maison.
#
# Usage : ./build-usb.sh <USB_DEVICE> [chemin/vers/VERSION]   (VERSION par défaut: <dossier du script>/VERSION)
# Exemple : ./installer/build-usb.sh /dev/sdb installer/VERSION.local
#
# ATTENTION : ce script écrit directement sur USB_DEVICE avec dd (destructif, efface tout
# le disque) puis le partitionne. Vérifie le device avec `lsblk` avant de lancer.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ $# -lt 1 ]]; then
    echo "Usage : $0 <USB_DEVICE> [chemin/vers/VERSION]" >&2
    echo "Exemple : $0 /dev/sdb $SCRIPT_DIR/VERSION.local" >&2
    exit 1
fi

USB_DEVICE="$1"
VERSION_FILE="${2:-$SCRIPT_DIR/VERSION}"

if [[ ! -f "$VERSION_FILE" ]]; then
    echo "Erreur : fichier de version introuvable : $VERSION_FILE" >&2
    exit 1
fi

# shellcheck disable=SC1090
source "$VERSION_FILE"

# --- Validation des paramètres requis ---------------------------------------
required_vars=(UBUNTU_ISO EXTRA_DEBS_DIR CIDATA_LABEL CIDATA_SIZE_MIB INSTANCE_ID MOK_PASSWORD)
missing=()
for v in "${required_vars[@]}"; do
    if [[ -z "${!v:-}" ]]; then
        missing+=("$v")
    fi
done
if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Erreur : paramètre(s) manquant(s) dans $VERSION_FILE : ${missing[*]}" >&2
    exit 1
fi

if [[ ! -f "$UBUNTU_ISO" ]]; then
    echo "Erreur : ISO introuvable : $UBUNTU_ISO" >&2
    exit 1
fi

if [[ ! -d "$EXTRA_DEBS_DIR" ]] || ! compgen -G "$EXTRA_DEBS_DIR/*.deb" > /dev/null; then
    echo "Erreur : aucun .deb trouvé dans $EXTRA_DEBS_DIR" >&2
    exit 1
fi

if [[ ! -b "$USB_DEVICE" ]]; then
    echo "Erreur : $USB_DEVICE n'est pas un périphérique bloc" >&2
    exit 1
fi

# --- Confirmation destructive ------------------------------------------------
echo "=== Cible sélectionnée ==="
lsblk -o NAME,SIZE,MODEL,TRAN "$USB_DEVICE"
echo
echo "Ceci va EFFACER TOUT LE CONTENU de $USB_DEVICE."
read -rp "Tape exactement '$USB_DEVICE' pour confirmer : " confirm
if [[ "$confirm" != "$USB_DEVICE" ]]; then
    echo "Confirmation invalide, abandon." >&2
    exit 1
fi

# --- 1. Écrit l'ISO stock sur la clé ----------------------------------------
echo "==> Écriture de l'ISO sur $USB_DEVICE..."
sudo dd if="$UBUNTU_ISO" of="$USB_DEVICE" bs=4M status=progress conv=fsync
sync
sudo partprobe "$USB_DEVICE"

# --- 2. Ajoute une partition FAT32 CIDATA dans l'espace libre restant -------
echo "==> Création de la partition $CIDATA_LABEL (${CIDATA_SIZE_MIB} MiB)..."
sudo parted -s "$USB_DEVICE" unit MiB mkpart primary fat32 "-${CIDATA_SIZE_MIB}" 100%
sudo parted -s "$USB_DEVICE" name 3 "$CIDATA_LABEL"
sudo partprobe "$USB_DEVICE"

# Détermine le nom de la 3e partition (gère /dev/sdX3 et /dev/nvme0n1p3)
if [[ "$USB_DEVICE" =~ [0-9]$ ]]; then
    CIDATA_PART="${USB_DEVICE}p3"
else
    CIDATA_PART="${USB_DEVICE}3"
fi
sleep 1
sudo mkfs.vfat -n "$CIDATA_LABEL" "$CIDATA_PART"

# --- 3. Génère user-data / meta-data à partir des paramètres du VERSION ----
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

cat > "$WORKDIR/meta-data" <<EOF
instance-id: ${INSTANCE_ID}
EOF

cat > "$WORKDIR/user-data" <<EOF
#cloud-config
autoinstall:
  version: 1

  # Garde TOUTES les étapes de l'assistant identiques à une install standard.
  interactive-sections:
    - "*"

  late-commands:
    - cp -r /cdrom/extra-debs /target/opt/extra-debs
    - curtin in-target -- apt-get update
    - curtin in-target -- apt-get install -y dkms mokutil build-essential linux-headers-generic
    - curtin in-target -- bash -c "apt-get install -y /opt/extra-debs/*.deb || apt-get -f install -y"
    - curtin in-target -- rm -rf /opt/extra-debs
    - >
      curtin in-target -- bash -c '
      if [ -f /var/lib/dkms/mok.key ] && ! mokutil --test-key /var/lib/dkms/mok.pub >/dev/null 2>&1; then
        echo -e "${MOK_PASSWORD}\n${MOK_PASSWORD}" | mokutil --import /var/lib/dkms/mok.pub;
      fi'
EOF

# --- 4. Copie tout sur la partition CIDATA ----------------------------------
MNT="$(mktemp -d)"
sudo mount "$CIDATA_PART" "$MNT"
sudo cp "$WORKDIR/user-data" "$WORKDIR/meta-data" "$MNT/"
sudo mkdir -p "$MNT/extra-debs"
sudo cp "$EXTRA_DEBS_DIR"/*.deb "$MNT/extra-debs/"
sync
sudo umount "$MNT"
rmdir "$MNT"

echo
echo "==> Clé USB prête : $USB_DEVICE"
echo "Au boot, sur l'entrée GRUB 'Try or Install Ubuntu Server', appuyer sur 'e' et ajouter"
echo "'autoinstall' en fin de ligne kernel avant de démarrer."
echo "Vérifier pendant l'install (Ctrl+Alt+F2) que /cdrom pointe bien vers la partition ${CIDATA_LABEL}"
echo "et ajuster le chemin dans late-commands si Subiquity la monte ailleurs."
