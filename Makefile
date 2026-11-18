CC ?= cc
CFLAGS ?= -O2 -g
GESTURE_FLAGS := -std=c17 -D_GNU_SOURCE -Wall -Wextra -Wpedantic -Werror
HARDENING := -D_FORTIFY_SOURCE=3 -fstack-protector-strong -fPIE -pie -Wl,-z,relro,-z,now

.PHONY: gestures test-gestures
gestures: build/honor-mra-gesture-daemon

build/honor-mra-gesture-daemon: gesture/gesture-daemon.c
	mkdir -p build
	$(CC) $(CFLAGS) $(GESTURE_FLAGS) $(HARDENING) $< $(LDFLAGS) -o $@

build/test-gestures: gesture/test-gestures.c gesture/gesture-daemon.c
	mkdir -p build
	$(CC) $(GESTURE_FLAGS) -O1 -g -fsanitize=undefined -fno-sanitize-recover=all $< -o $@

test-gestures: build/test-gestures
	./build/test-gestures

CARGO ?= cargo
USER_PREFIX ?= $(HOME)/.local
APPLICATIONS_DIR ?= $(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share)/applications
APP_DESKTOP := io.github.no1se.HonorKit.desktop

.PHONY: app install-app uninstall-app
app:
	$(CARGO) build --locked --manifest-path fingerprint-app/Cargo.toml

install-app: app
	install -Dm 0755 fingerprint-app/target/debug/honorkit "$(USER_PREFIX)/bin/honorkit"
	install -Dm 0644 "fingerprint-app/data/$(APP_DESKTOP)" "$(APPLICATIONS_DIR)/$(APP_DESKTOP)"

uninstall-app:
	rm -f -- "$(USER_PREFIX)/bin/honorkit" "$(APPLICATIONS_DIR)/$(APP_DESKTOP)"
