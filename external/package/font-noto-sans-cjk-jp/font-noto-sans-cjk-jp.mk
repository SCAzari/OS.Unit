################################################################################
#
# font-noto-sans-cjk-jp (toolkit spike, UNIT-OS-WP-0010)
#
################################################################################

FONT_NOTO_SANS_CJK_JP_VERSION = Sans2.004
FONT_NOTO_SANS_CJK_JP_SITE = https://raw.githubusercontent.com/notofonts/noto-cjk/$(FONT_NOTO_SANS_CJK_JP_VERSION)/Sans/OTF/Japanese
FONT_NOTO_SANS_CJK_JP_SOURCE = NotoSansCJKjp-Regular.otf
FONT_NOTO_SANS_CJK_JP_EXTRA_DOWNLOADS = NotoSansCJKjp-Bold.otf \
	https://raw.githubusercontent.com/notofonts/noto-cjk/$(FONT_NOTO_SANS_CJK_JP_VERSION)/LICENSE
FONT_NOTO_SANS_CJK_JP_LICENSE = OFL-1.1
FONT_NOTO_SANS_CJK_JP_LICENSE_FILES = LICENSE

define FONT_NOTO_SANS_CJK_JP_EXTRACT_CMDS
	cp $(FONT_NOTO_SANS_CJK_JP_DL_DIR)/NotoSansCJKjp-Regular.otf \
		$(FONT_NOTO_SANS_CJK_JP_DL_DIR)/NotoSansCJKjp-Bold.otf \
		$(FONT_NOTO_SANS_CJK_JP_DL_DIR)/LICENSE $(@D)/
endef

define FONT_NOTO_SANS_CJK_JP_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0644 $(@D)/NotoSansCJKjp-Regular.otf \
		$(TARGET_DIR)/usr/share/fonts/noto-cjk/NotoSansCJKjp-Regular.otf
	$(INSTALL) -D -m 0644 $(@D)/NotoSansCJKjp-Bold.otf \
		$(TARGET_DIR)/usr/share/fonts/noto-cjk/NotoSansCJKjp-Bold.otf
endef

$(eval $(generic-package))
