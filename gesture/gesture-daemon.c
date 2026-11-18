/* SPDX-License-Identifier: GPL-3.0-only
 * Copyright (c) 2025 MadhiasM.
 * Based on honor-magicbook-art-touchpad-gestures by MadhiasM.
 */
#include "settings.h"

#include <errno.h>
#include <fcntl.h>
#include <linux/hidraw.h>
#include <linux/input.h>
#include <linux/uinput.h>
#include <poll.h>
#include <signal.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/signalfd.h>
#include <time.h>
#include <unistd.h>

#define GESTURE_REPORT 0x0e
#define GESTURE_REPORT_SIZE 9
#define MRA_GESTURE_REPORT_SIZE 31

struct gesture {
    uint8_t type;
    uint8_t direction;
    unsigned short key;
    unsigned short modifier;
    const char *name;
};

static const struct gesture gestures[] = {
    {0x03, 1, KEY_BRIGHTNESSUP, 0, "brightness-up"},
    {0x03, 2, KEY_BRIGHTNESSDOWN, 0, "brightness-down"},
    {0x04, 1, KEY_VOLUMEUP, 0, "volume-up"},
    {0x04, 2, KEY_VOLUMEDOWN, 0, "volume-down"},
    {0x0a, 3, KEY_V, KEY_LEFTMETA, "notifications"},
    {0x06, 0, KEY_SYSRQ, 0, "screenshot-ui"},
    {0x07, 0, KEY_SYSRQ, KEY_LEFTSHIFT, "screenshot"},
    {0x08, 0, KEY_H, KEY_LEFTMETA, "minimize"},
    {0x09, 0, KEY_F4, KEY_LEFTALT, "close"},
};

static const struct gesture *decode(const uint8_t *report, size_t size)
{
    if ((size != GESTURE_REPORT_SIZE && size != MRA_GESTURE_REPORT_SIZE) ||
        report[0] != GESTURE_REPORT)
        return NULL;

    for (size_t i = 0; i < sizeof(gestures) / sizeof(gestures[0]); ++i) {
        const struct gesture *gesture = &gestures[i];
        if (report[1] == gesture->type &&
            (gesture->direction == 0 || report[2] == gesture->direction))
            return gesture;
    }
    return NULL;
}

static unsigned load_gesture_settings(const char *path)
{
    int fd = open(path, O_RDONLY | O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK);
    if (fd < 0)
        return G_DEFAULT;
    char text[12];
    ssize_t length = read(fd, text, sizeof(text));
    close(fd);
    if (length < 2 || length == sizeof(text) || text[length - 1] != '\n')
        return G_DEFAULT;
    text[length - 1] = '\0';
    unsigned value;
    return parse_setting(text, G_ALL, &value) ? value : G_DEFAULT;
}

static const struct gesture *configured_gesture(const struct gesture *event, unsigned settings)
{
    bool swap = settings & G_SWAP_EDGES;
    unsigned group;
    switch (event->type) {
    case 0x03:
        group = swap ? G_VOLUME : G_BRIGHTNESS;
        if (swap)
            event += 2;
        break;
    case 0x04:
        group = swap ? G_BRIGHTNESS : G_VOLUME;
        if (swap)
            event -= 2;
        break;
    case 0x06:
    case 0x07:
        group = G_SCREENSHOT;
        break;
    case 0x08:
    case 0x09:
        group = G_WINDOWS;
        break;
    default:
        group = G_NOTIFICATIONS;
        break;
    }
    return settings & group ? event : NULL;
}

static int setup_uinput(int fd)
{
    struct uinput_setup setup = {
        .id = {.bustype = BUS_VIRTUAL, .vendor = HONORKIT_VENDOR,
               .product = HONORKIT_PRODUCT, .version = 1},
        .name = "HONOR MagicBook Art Gesture Keys",
    };

    if (ioctl(fd, UI_SET_EVBIT, EV_KEY) < 0)
        return -1;
    for (size_t i = 0; i < sizeof(gestures) / sizeof(gestures[0]); ++i) {
        const struct gesture *gesture = &gestures[i];
        if (ioctl(fd, UI_SET_KEYBIT, gesture->key) < 0 ||
            (gesture->modifier && ioctl(fd, UI_SET_KEYBIT, gesture->modifier) < 0))
            return -1;
    }
    if (ioctl(fd, UI_DEV_SETUP, &setup) < 0)
        return -1;
    return ioctl(fd, UI_DEV_CREATE);
}

static int emit(int fd, const struct gesture *gesture)
{
    struct input_event events[6] = {0};
    size_t count = 0;

    if (gesture->modifier)
        events[count++] = (struct input_event){.type = EV_KEY, .code = gesture->modifier, .value = 1};
    events[count++] = (struct input_event){.type = EV_KEY, .code = gesture->key, .value = 1};
    events[count++] = (struct input_event){.type = EV_SYN, .code = SYN_REPORT};
    events[count++] = (struct input_event){.type = EV_KEY, .code = gesture->key};
    if (gesture->modifier)
        events[count++] = (struct input_event){.type = EV_KEY, .code = gesture->modifier};
    events[count++] = (struct input_event){.type = EV_SYN, .code = SYN_REPORT};

    size_t bytes = count * sizeof(events[0]);
    ssize_t written;
    do {
        written = write(fd, events, bytes);
    } while (written < 0 && errno == EINTR);
    if (written >= 0 && (size_t)written != bytes) {
        errno = EIO;
        return -1;
    }
    return written < 0 ? -1 : 0;
}

static int run(int hid, int signals, int output, const char *settings_path)
{
    struct pollfd sources[] = {{.fd = signals, .events = POLLIN},
                               {.fd = hid, .events = POLLIN}};
    uint8_t report[64];
    unsigned settings = G_DEFAULT;
    time_t loaded_at = -1;

    for (;;) {
        if (poll(sources, 2, -1) < 0) {
            if (errno == EINTR)
                continue;
            return -1;
        }
        if (sources[0].revents & POLLIN)
            return 0;
        if ((sources[0].revents | sources[1].revents) & (POLLERR | POLLHUP | POLLNVAL)) {
            errno = ENODEV;
            return -1;
        }
        if (!(sources[1].revents & POLLIN))
            continue;

        ssize_t length = read(hid, report, sizeof(report));
        if (length < 0) {
            if (errno == EINTR || errno == EAGAIN)
                continue;
            return -1;
        }
        if (length == 0) {
            errno = ENODEV;
            return -1;
        }

        const struct gesture *gesture = decode(report, (size_t)length);
        if (!gesture)
            continue;
        if (output < 0) {
            puts(gesture->name);
            fflush(stdout);
        } else {
            struct timespec now;
            if (settings_path && clock_gettime(CLOCK_MONOTONIC, &now) == 0 && now.tv_sec != loaded_at) {
                settings = load_gesture_settings(settings_path);
                loaded_at = now.tv_sec;
            }
            gesture = configured_gesture(gesture, settings);
            if (gesture && emit(output, gesture) < 0)
                return -1;
        }
    }
}

int main(int argc, char **argv)
{
    bool monitor = argc == 2 && strcmp(argv[1], "--monitor") == 0;
    if (argc != 1 && !monitor) {
        fprintf(stderr, "Usage: %s [--monitor]\n", argv[0]);
        return 2;
    }

    sigset_t mask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGINT);
    sigaddset(&mask, SIGTERM);
    if (sigprocmask(SIG_BLOCK, &mask, NULL) < 0) {
        perror("sigprocmask");
        return 1;
    }
    int signals = signalfd(-1, &mask, SFD_CLOEXEC);
    if (signals < 0) {
        perror("signalfd");
        return 1;
    }

    int hid = open(HONORKIT_TOUCHPAD, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
    if (hid < 0) {
        perror(HONORKIT_TOUCHPAD);
        return 1;
    }
    struct hidraw_devinfo info;
    if (ioctl(hid, HIDIOCGRAWINFO, &info) < 0) {
        perror("HIDIOCGRAWINFO");
        return 1;
    }
    if (info.bustype != BUS_I2C || info.vendor != HONORKIT_VENDOR || info.product != HONORKIT_PRODUCT) {
        fprintf(stderr, "Unexpected HID device at %s\n", HONORKIT_TOUCHPAD);
        return 1;
    }

    int output = -1;
    if (!monitor) {
        output = open("/dev/uinput", O_WRONLY | O_NONBLOCK | O_CLOEXEC);
        if (output < 0 || setup_uinput(output) < 0) {
            perror("uinput setup");
            return 1;
        }
    }

    int result = run(hid, signals, output, HONORKIT_SETTINGS);
    if (result < 0)
        perror("gesture input");
    if (output >= 0) {
        ioctl(output, UI_DEV_DESTROY);
        close(output);
    }
    close(hid);
    close(signals);
    return result < 0 ? 1 : 0;
}
