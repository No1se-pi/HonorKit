/* SPDX-License-Identifier: GPL-3.0-only */
#include "settings.h"

#include <errno.h>
#include <fcntl.h>
#include <linux/hidraw.h>
#include <linux/input.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <unistd.h>

static int save_gestures(unsigned value)
{
    char path[] = "/etc/.honorkit-gestures.XXXXXX";
    int fd = mkstemp(path);
    if (fd < 0)
        return -1;

    int result = 0;
    if (fchmod(fd, 0644) < 0 || dprintf(fd, "%u\n", value) < 0 || fsync(fd) < 0)
        result = -1;
    int error = errno;
    if (close(fd) < 0 && result == 0) {
        result = -1;
        error = errno;
    }
    if (result == 0 && rename(path, HONORKIT_SETTINGS) == 0)
        return 0;
    if (result == 0)
        error = errno;
    unlink(path);
    errno = error;
    return -1;
}

static int send_haptics(unsigned level)
{
    FILE *product = fopen("/sys/class/dmi/id/product_name", "re");
    if (!product)
        return -1;
    char model[32];
    bool supported = fgets(model, sizeof(model), product) && strcmp(model, "MRA-XXX\n") == 0;
    fclose(product);
    if (!supported) {
        errno = ENODEV;
        return -1;
    }

    int fd = open(HONORKIT_TOUCHPAD, O_RDWR | O_CLOEXEC | O_NONBLOCK);
    if (fd < 0)
        return -1;
    struct hidraw_devinfo device;
    int result = ioctl(fd, HIDIOCGRAWINFO, &device);
    if (result == 0 && (device.bustype != BUS_I2C ||
                        device.vendor != HONORKIT_VENDOR || device.product != HONORKIT_PRODUCT)) {
        errno = ENODEV;
        result = -1;
    }
    if (result == 0) {
        const uint8_t report[9] = {0x0e, 0x02, (uint8_t)level};
        ssize_t written;
        do {
            written = write(fd, report, sizeof(report));
        } while (written < 0 && errno == EINTR);
        if (written >= 0 && (size_t)written != sizeof(report))
            errno = EIO;
        if (written != sizeof(report))
            result = -1;
    }
    int error = errno;
    close(fd);
    errno = error;
    return result;
}

int main(int argc, char **argv)
{
    unsigned value;
    bool gestures = argc == 3 && strcmp(argv[1], "gestures") == 0;
    bool haptics = argc == 3 && strcmp(argv[1], "haptics") == 0;
    if ((!gestures && !haptics) || !parse_setting(argv[2], gestures ? G_ALL : 2, &value)) {
        fputs("Usage: honorkit-touchpad-config gestures 0..63 | haptics 0..2\n", stderr);
        return 2;
    }
    if (geteuid() != 0) {
        fputs("Administrator authentication is required.\n", stderr);
        return 1;
    }

    if ((gestures ? save_gestures(value) : send_haptics(value)) < 0) {
        perror("honorkit-touchpad-config");
        return 1;
    }
    return 0;
}
