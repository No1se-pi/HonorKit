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
