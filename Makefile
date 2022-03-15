prefix ?= /usr
sysconfdir ?= /etc
exec_prefix = $(prefix)
bindir = $(exec_prefix)/bin
libdir = $(exec_prefix)/lib
libexecdir = $(exec_prefix)/libexec
includedir = $(prefix)/include
datarootdir = $(prefix)/share
datadir = $(datarootdir)

SRC = Cargo.toml Cargo.lock Makefile $(shell find src -type f -wholename '*src/*.rs')

.PHONY: all clean distclean install uninstall update

BIN=hp-vendor

TARGET = debug
DEBUG ?= 0
ifeq ($(DEBUG),0)
	TARGET = release
	ARGS += --release
endif

VENDOR ?= 0
ifneq ($(VENDOR),0)
	ARGS += --frozen
endif

all: target/release/$(BIN)

clean:
	cargo clean

distclean:
	rm -rf .cargo vendor vendor.tar.xz

install: all
	install -D -m 0755 "target/release/$(BIN)" "$(DESTDIR)$(libexecdir)/$(BIN)"
	install -D -m 0644 "$(BIN).service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN).service"
	install -D -m 0644 "$(BIN)-daily.service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.service"
	install -D -m 0644 "$(BIN)-daily.timer" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.timer"
	install -D -m 0644 "$(BIN)-upload.service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-upload.service"
	install -D -m 0644 "$(BIN)-upload.timer" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-upload.timer"

uninstall:
	rm -f "$(DESTDIR)$(libexecdir)/$(BIN)"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN).service"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.service"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.timer"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily-upload.service"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily-upload.timer"

update:
	cargo update

vendor:
	rm .cargo -rf
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar cf vendor.tar vendor
	rm -rf vendor

vendor-check:
ifeq ($(VENDOR),1)
	rm vendor -rf && tar xf vendor.tar
endif

target/release/$(BIN): $(SRC) vendor-check
	cargo build $(ARGS)

	cargo build $(ARGS)
