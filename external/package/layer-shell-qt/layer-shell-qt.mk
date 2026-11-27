################################################################################
#
# layer-shell-qt (UNIT-OS-WP-0009)
#
################################################################################

# 6.5.5 is the newest release that builds with Buildroot 2026.02's Qt 6.9.1 (6.6 needs Qt 6.10).
LAYER_SHELL_QT_VERSION = 6.5.5
LAYER_SHELL_QT_SITE = https://download.kde.org/stable/plasma/$(LAYER_SHELL_QT_VERSION)
LAYER_SHELL_QT_SOURCE = layer-shell-qt-$(LAYER_SHELL_QT_VERSION).tar.xz
LAYER_SHELL_QT_LICENSE = LGPL-2.1-only or LGPL-3.0-only (library), BSD-3-Clause (build files), MIT (protocol)
LAYER_SHELL_QT_LICENSE_FILES = \
	LICENSES/LGPL-2.1-only.txt \
	LICENSES/LGPL-3.0-only.txt \
	LICENSES/LicenseRef-KDE-Accepted-LGPL.txt \
	LICENSES/BSD-3-Clause.txt \
	LICENSES/MIT.txt
LAYER_SHELL_QT_INSTALL_STAGING = YES
LAYER_SHELL_QT_DEPENDENCIES = \
	extra-cmake-modules \
	host-wayland \
	libxkbcommon \
	qt6base \
	qt6declarative \
	qt6wayland \
	wayland \
	wayland-protocols
# Buildroot's Qt keeps its plugins and QML modules under lib/qt6 (INSTALL_ARCHDATADIR).
LAYER_SHELL_QT_CONF_OPTS = \
	-DBUILD_TESTING=OFF \
	-DQT_HOST_PATH=$(HOST_DIR) \
	-DKDE_INSTALL_QTPLUGINDIR=lib/qt6/plugins \
	-DKDE_INSTALL_QMLDIR=lib/qt6/qml

$(eval $(cmake-package))
