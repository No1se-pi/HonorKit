/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef HONORKIT_SETTINGS_H
#define HONORKIT_SETTINGS_H

#include <stdbool.h>

#define HONORKIT_SETTINGS "/etc/honorkit-gestures.conf"
#define HONORKIT_TOUCHPAD "/dev/honor-mra-touchpad"
#define HONORKIT_VENDOR 0x35cc
#define HONORKIT_PRODUCT 0x0104

enum gesture_settings {
    G_BRIGHTNESS = 1 << 0,
    G_VOLUME = 1 << 1,
    G_SCREENSHOT = 1 << 2,
    G_WINDOWS = 1 << 3,
    G_NOTIFICATIONS = 1 << 4,
    G_SWAP_EDGES = 1 << 5,
    G_DEFAULT = G_BRIGHTNESS | G_VOLUME | G_SCREENSHOT | G_WINDOWS | G_NOTIFICATIONS,
    G_ALL = G_DEFAULT | G_SWAP_EDGES
};

static inline bool parse_setting(const char *text, unsigned max, unsigned *result)
{
    unsigned number = 0;
    if (*text < '0' || *text > '9')
        return false;
    while (*text >= '0' && *text <= '9') {
        number = number * 10 + (unsigned)(*text++ - '0');
        if (number > max)
            return false;
    }
    if (*text != '\0')
        return false;
    *result = number;
    return true;
}
#endif
