# KyberFrog / kyfrogmix — tâches de dépôt.

.PHONY: git-submodules

# Initialise/synchronise les sous-modules (third_party/kyctl, third_party/kyber-desktop —
# fork Kyber : kyclient/kycontroller). À lancer après un clone, ou après un
# changement de révision de sous-module.
#
# Pas de --recursive sur l'update : kyber-desktop a lui-même deux sous-modules
# (deps/winit, core/kysdk) sous des groupes GitLab différents de kyber-frog,
# auxquels tous les comptes n'ont pas accès. Le contenu principal de
# kyber-desktop (dont le script kyclient) n'en dépend pas.
git-submodules:
	git submodule sync --recursive
	git submodule update --init
