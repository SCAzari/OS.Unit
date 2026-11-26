################################################################################
#
# anthy-unicode (UNIT-OS-WP-0009)
#
################################################################################

ANTHY_UNICODE_VERSION = 1.0.0.20260213
ANTHY_UNICODE_SITE = https://github.com/fujiwarat/anthy-unicode/releases/download/$(ANTHY_UNICODE_VERSION)
ANTHY_UNICODE_SOURCE = anthy-unicode-$(ANTHY_UNICODE_VERSION).tar.xz
ANTHY_UNICODE_LICENSE = LGPL-2.1+ (library), GPL-2.0+ (dictionary data)
ANTHY_UNICODE_LICENSE_FILES = COPYING
ANTHY_UNICODE_INSTALL_STAGING = YES
ANTHY_UNICODE_CONF_OPTS = -Demacs=disabled
HOST_ANTHY_UNICODE_CONF_OPTS = -Demacs=disabled

# The dictionary is built by running the package's own tools, which a cross build cannot do
# (patch 0001 leaves them out of it). Build the dictionary on the host and install it with the
# target library: it is the same for x86-64 host and target.
ANTHY_UNICODE_DEPENDENCIES = host-anthy-unicode
define ANTHY_UNICODE_INSTALL_HOST_DICTIONARY
	$(INSTALL) -D -m 0644 $(HOST_DIR)/share/anthy-unicode/anthy.dic \
		$(TARGET_DIR)/usr/share/anthy-unicode/anthy.dic
	$(INSTALL) -D -m 0644 $(HOST_DIR)/share/anthy-unicode/zipcode.t \
		$(TARGET_DIR)/usr/share/anthy-unicode/zipcode.t
endef
ANTHY_UNICODE_POST_INSTALL_TARGET_HOOKS += ANTHY_UNICODE_INSTALL_HOST_DICTIONARY

$(eval $(meson-package))
$(eval $(host-meson-package))
