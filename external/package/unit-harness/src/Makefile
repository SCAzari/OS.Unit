# unit-harness: build and install (UNIT-OS-WP-0002)
CFLAGS ?= -O2
CFLAGS += -Wall -Wextra
BIN = unit-evstamp unit-inject
SCRIPTS = unit-stats unit-pair unit-trace unit-harness
all: $(BIN)
unit-evstamp: unit-evstamp.c
	$(CC) $(CFLAGS) -o $@ $<
unit-inject: unit-inject.c
	$(CC) $(CFLAGS) -o $@ $<
install: all
	install -d $(DESTDIR)/usr/bin $(DESTDIR)/usr/lib/unit-harness $(DESTDIR)/usr/libexec/unit $(DESTDIR)/usr/lib/systemd/system
	install -m 0755 $(BIN) $(SCRIPTS) $(DESTDIR)/usr/bin/
	install -m 0644 lib.sh trace-filter.awk $(DESTDIR)/usr/lib/unit-harness/
	install -m 0755 unit-harness-boot $(DESTDIR)/usr/libexec/unit/unit-harness-boot
	install -m 0644 unit-harness.service $(DESTDIR)/usr/lib/systemd/system/unit-harness.service
clean:
	rm -f $(BIN)
.PHONY: all install clean
