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

DEBUG ?= 0
ifeq ($(DEBUG),0)
	ARGS += "--release"
	TARGET = release
endif

VENDORED ?= 0
ifeq ($(VENDORED),1)
	ARGS += "--frozen"
endif

all: target/release/$(BIN)

clean:
	cargo clean

distclean:
	rm -rf .cargo vendor vendor.tar.xz

install: all
	install -D -m 0755 "target/release/$(BIN)" "$(DESTDIR)$(libexecdir)/$(BIN)"
	install -D -m 0644 "$(BIN).service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN).service"
	install -D -m 0644 "$(BIN).service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.service"
	install -D -m 0644 "$(BIN)-daily.service" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.service"
	install -D -m 0644 "$(BIN)-daily.timer" "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.timer"

uninstall:
	rm -f "$(DESTDIR)$(libexecdir)/$(BIN)"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN).service"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.service"
	rm -f "$(DESTDIR)$(libdir)/systemd/system/$(BIN)-daily.timer"

update:
	cargo update

vendor:
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar pcfJ vendor.tar.xz vendor
	rm -rf vendor

target/release/$(BIN): $(SRC)
ifeq ($(VENDORED),1)
	tar pxf vendor.tar.xz
endif
	cargo build --features disable-model-check $(ARGS)
