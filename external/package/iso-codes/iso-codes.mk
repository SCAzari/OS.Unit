################################################################################
#
# iso-codes (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

ISO_CODES_VERSION = 4.20.1
ISO_CODES_SITE = https://salsa.debian.org/iso-codes-team/iso-codes/-/archive/v$(ISO_CODES_VERSION)
ISO_CODES_SOURCE = iso-codes-v$(ISO_CODES_VERSION).tar.gz
ISO_CODES_LICENSE = LGPL-2.1+
ISO_CODES_LICENSE_FILES = LICENSES/LGPL-2.1-or-later.txt
ISO_CODES_DEPENDENCIES = host-python3 host-gettext
ISO_CODES_INSTALL_STAGING = YES

# Only the JSON data is needed; translations are left out to keep the image small.
define ISO_CODES_REMOVE_TRANSLATIONS
	find $(TARGET_DIR)/usr/share/locale -name 'iso_*.mo' -delete 2>/dev/null || true
	rm -rf $(TARGET_DIR)/usr/share/xml/iso-codes
endef
ISO_CODES_POST_INSTALL_TARGET_HOOKS += ISO_CODES_REMOVE_TRANSLATIONS

$(eval $(meson-package))
