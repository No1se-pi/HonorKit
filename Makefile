CC ?= cc
CFLAGS ?= -O2 -g
GESTURE_FLAGS := -std=c17 -D_GNU_SOURCE -Wall -Wextra -Wpedantic -Werror
HARDENING := -D_FORTIFY_SOURCE=3 -fstack-protector-strong -fPIE -pie -Wl,-z,relro,-z,now

.PHONY: gestures test-gestures
gestures: build/honor-mra-gesture-daemon build/honorkit-touchpad-config

build/honor-mra-gesture-daemon: gesture/gesture-daemon.c gesture/settings.h
	mkdir -p build
	$(CC) $(CFLAGS) $(GESTURE_FLAGS) $(HARDENING) $< $(LDFLAGS) -o $@

build/honorkit-touchpad-config: gesture/configure.c gesture/settings.h
	mkdir -p build
	$(CC) $(CFLAGS) $(GESTURE_FLAGS) $(HARDENING) $< $(LDFLAGS) -o $@

build/test-gestures: gesture/test-gestures.c gesture/gesture-daemon.c gesture/settings.h
	mkdir -p build
	$(CC) $(GESTURE_FLAGS) -O1 -g -fsanitize=undefined -fno-sanitize-recover=all $< -o $@

test-gestures: build/test-gestures
	./build/test-gestures

CARGO ?= cargo
USER_PREFIX ?= $(HOME)/.local
APPLICATIONS_DIR ?= $(if $(XDG_DATA_HOME),$(XDG_DATA_HOME),$(HOME)/.local/share)/applications
APP_DESKTOP := io.github.no1se.HonorKit.desktop
AUTOSTART_DIR ?= $(if $(XDG_CONFIG_HOME),$(XDG_CONFIG_HOME),$(HOME)/.config)/autostart
UNLOCK_DESKTOP := io.github.no1se.HonorKit.Unlock.desktop

.PHONY: app install-app uninstall-app
app:
	$(CARGO) build --locked --manifest-path fingerprint-app/Cargo.toml

install-app: app
	install -Dm 0755 fingerprint-app/target/debug/honorkit "$(USER_PREFIX)/bin/honorkit"
	install -Dm 0644 "fingerprint-app/data/$(APP_DESKTOP)" "$(APPLICATIONS_DIR)/$(APP_DESKTOP)"
	install -Dm 0644 "fingerprint-app/data/$(UNLOCK_DESKTOP)" "$(AUTOSTART_DIR)/$(UNLOCK_DESKTOP)"

uninstall-app:
	rm -f -- "$(USER_PREFIX)/bin/honorkit" "$(APPLICATIONS_DIR)/$(APP_DESKTOP)" "$(AUTOSTART_DIR)/$(UNLOCK_DESKTOP)"

SYSTEM_HELPER := /usr/local/libexec/honorkit-system
SYSTEM_DATA := /usr/local/share/honorkit
SYSTEM_POLICY := /usr/share/polkit-1/actions/io.github.no1se.HonorKit.system.policy

FRIEREN_FONTS := build/frieren/menu.pf2 build/frieren/small.pf2 build/frieren/title.pf2
DEJAVU := /usr/share/fonts/truetype/dejavu

.PHONY: grub-theme test-grub-theme install-grub-theme install-system-tools uninstall-system-tools
grub-theme: $(FRIEREN_FONTS)

test-grub-theme: grub-theme
	python3 system/test_frieren.py -v

build/frieren/menu.pf2: $(DEJAVU)/DejaVuSans-Bold.ttf Makefile
	mkdir -p $(@D)
	grub-mkfont -s 42 -o "$@" "$<"

build/frieren/small.pf2: $(DEJAVU)/DejaVuSans-Bold.ttf Makefile
	mkdir -p $(@D)
	grub-mkfont -s 30 -o "$@" "$<"

build/frieren/title.pf2: $(DEJAVU)/DejaVuSerif-Bold.ttf Makefile
	mkdir -p $(@D)
	grub-mkfont -s 70 -o "$@" "$<"

install-grub-theme: install-system-tools
	sudo "$(SYSTEM_HELPER)" install grub

install-system-tools: grub-theme
	sudo install -d -m 0755 /usr/local/libexec "$(SYSTEM_DATA)/frieren/icons"
	sudo install -m 0644 system/frieren/theme.txt system/frieren/*.png $(FRIEREN_FONTS) "$(SYSTEM_DATA)/frieren/"
	sudo install -m 0644 system/frieren/icons/*.png "$(SYSTEM_DATA)/frieren/icons/"
	sudo install -m 0644 system/90-honor-mra-audio.conf system/honor-mra-alc256-speakers.fw "$(SYSTEM_DATA)/"
	sudo install -m 0755 system/honorkit-system "$(SYSTEM_HELPER)"
	sudo install -m 0644 system/io.github.no1se.HonorKit.system.policy "$(SYSTEM_POLICY)"

uninstall-system-tools:
	sudo rm -f -- "$(SYSTEM_POLICY)" "$(SYSTEM_HELPER)" "$(SYSTEM_DATA)/90-honor-mra-audio.conf" "$(SYSTEM_DATA)/honor-mra-alc256-speakers.fw"
	sudo rm -f -- "$(SYSTEM_DATA)/frieren/frieren-menu.png" "$(SYSTEM_DATA)/frieren/frieren-menu.pf2"
	@if test -d "$(SYSTEM_DATA)/frieren/icons"; then sudo rm -f -- "$(SYSTEM_DATA)"/frieren/icons/*.png && sudo rmdir -- "$(SYSTEM_DATA)/frieren/icons"; fi
	sudo rm -f -- "$(SYSTEM_DATA)/frieren/theme.txt" "$(SYSTEM_DATA)"/frieren/*.png "$(SYSTEM_DATA)"/frieren/*.pf2
	sudo rmdir -- "$(SYSTEM_DATA)/frieren" "$(SYSTEM_DATA)"
