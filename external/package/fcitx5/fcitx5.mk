################################################################################
#
# fcitx5 (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

# Newest release (Buildroot 2026.08.1 has wayland-protocols 1.48).
FCITX5_VERSION = 5.1.23
# The release tarball, not GitHub's archive: it includes the yoga submodule (third_party/yoga).
FCITX5_SITE = https://download.fcitx-im.org/fcitx5/fcitx5
FCITX5_SOURCE = fcitx5-$(FCITX5_VERSION).tar.zst
FCITX5_LICENSE = LGPL-2.1+, Unicode-DFS-2016 (data)
FCITX5_LICENSE_FILES = LICENSES/LGPL-2.1-or-later.txt LICENSES/Unicode-DFS-2016.txt
FCITX5_INSTALL_STAGING = YES
FCITX5_DEPENDENCIES = host-pkgconf host-gettext extra-cmake-modules iso-codes json-for-modern-cpp \
	libxkbcommon xkeyboard-config expat cairo pango gdk-pixbuf librsvg libglib2 systemd \
	wayland wayland-protocols util-linux
FCITX5_CONF_OPTS = \
	-DENABLE_X11=OFF \
	-DENABLE_WAYLAND=ON \
	-DENABLE_DBUS=ON \
	-DUSE_SYSTEMD=ON \
	-DENABLE_KEYBOARD=ON \
	-DENABLE_ENCHANT=OFF \
	-DENABLE_EMOJI=OFF \
	-DENABLE_TEST=OFF \
	-DENABLE_TESTING_ADDONS=ON \
	-DENABLE_DOC=OFF \
	-DENABLE_XDGAUTOSTART=OFF \
	-DBUILD_SPELL_DICT=OFF \
	-DUSE_SYSTEM_PLASMA_WAYLAND_PROTOCOLS=OFF \
	-DENABLE_LIBUUID=ON

# Fcitx5 exports absolute target paths in its CMake files, which a cross build of an add-on
# would look for on the build machine. In staging only:
# - add-ons include Fcitx5's CMake helpers from FCITX_INSTALL_CMAKECONFIG_DIR: point it at the
#   copy beside the config file;
# - the exported include folders (INTERFACE_INCLUDE_DIRECTORIES "/usr/include/...") point into
#   staging.
# The FCITX_INSTALL_* destinations stay target paths: add-ons install there under DESTDIR.
define FCITX5_STAGING_CMAKECONFIG_DIR
	$(SED) 's#^set(FCITX_INSTALL_CMAKECONFIG_DIR "/usr/lib/cmake")#get_filename_component(FCITX_INSTALL_CMAKECONFIG_DIR "$${CMAKE_CURRENT_LIST_DIR}/.." ABSOLUTE)#' \
		$(STAGING_DIR)/usr/lib/cmake/Fcitx5Utils/Fcitx5UtilsConfig.cmake
	grep -q 'get_filename_component(FCITX_INSTALL_CMAKECONFIG_DIR' \
		$(STAGING_DIR)/usr/lib/cmake/Fcitx5Utils/Fcitx5UtilsConfig.cmake
	$(SED) 's#INTERFACE_INCLUDE_DIRECTORIES "/usr/include#INTERFACE_INCLUDE_DIRECTORIES "$(STAGING_DIR)/usr/include#' \
		$(STAGING_DIR)/usr/lib/cmake/Fcitx5*/*.cmake
endef
FCITX5_POST_INSTALL_STAGING_HOOKS += FCITX5_STAGING_CMAKECONFIG_DIR

$(eval $(cmake-package))
