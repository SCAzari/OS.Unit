################################################################################
#
# fcitx5-qt (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

FCITX5_QT_VERSION = 5.1.16
FCITX5_QT_SITE = $(call github,fcitx,fcitx5-qt,$(FCITX5_QT_VERSION))
FCITX5_QT_LICENSE = LGPL-2.1+ (libraries), BSD-3-Clause (input module)
FCITX5_QT_LICENSE_FILES = LICENSES/LGPL-2.1-or-later.txt LICENSES/BSD-3-Clause.txt
FCITX5_QT_DEPENDENCIES = fcitx5 extra-cmake-modules qt6base qt6wayland
FCITX5_QT_CONF_OPTS = \
	-DENABLE_QT4=OFF \
	-DENABLE_QT5=OFF \
	-DENABLE_QT6=ON \
	-DENABLE_X11=OFF \
	-DENABLE_QT6_WAYLAND_WORKAROUND=OFF \
	-DBUILD_ONLY_PLUGIN=ON \
	-DCMAKE_INSTALL_QT6PLUGINDIR=/usr/lib/qt6/plugins

$(eval $(cmake-package))
