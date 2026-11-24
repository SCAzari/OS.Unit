################################################################################
#
# fcitx5-anthy (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

FCITX5_ANTHY_VERSION = 5.1.10
FCITX5_ANTHY_SITE = $(call github,fcitx,fcitx5-anthy,$(FCITX5_ANTHY_VERSION))
FCITX5_ANTHY_LICENSE = GPL-2.0+
FCITX5_ANTHY_LICENSE_FILES = LICENSES/GPL-2.0-or-later.txt
FCITX5_ANTHY_DEPENDENCIES = fcitx5 anthy-unicode extra-cmake-modules host-gettext
FCITX5_ANTHY_CONF_OPTS = -DENABLE_TEST=OFF

$(eval $(cmake-package))
