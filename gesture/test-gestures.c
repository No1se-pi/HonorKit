#define main gesture_daemon_main
#include "gesture-daemon.c"
#undef main

#include <assert.h>
#include <sys/socket.h>
#include <sys/wait.h>

int main(void)
{
    static const struct {
        uint8_t type, direction;
        unsigned short key, modifier;
    } expected[] = {
        {3, 1, KEY_BRIGHTNESSUP, 0}, {3, 2, KEY_BRIGHTNESSDOWN, 0},
        {4, 1, KEY_VOLUMEUP, 0}, {4, 2, KEY_VOLUMEDOWN, 0},
        {10, 3, KEY_V, KEY_LEFTMETA}, {6, 0, KEY_SYSRQ, 0},
        {7, 0, KEY_SYSRQ, KEY_LEFTSHIFT}, {8, 0, KEY_H, KEY_LEFTMETA},
        {9, 0, KEY_F4, KEY_LEFTALT},
    };

    int stream[2];
    assert(pipe(stream) == 0);
    uint8_t report[9] = {0x0e};

    for (size_t i = 0; i < sizeof(expected) / sizeof(expected[0]); ++i) {
        report[1] = expected[i].type;
        report[2] = expected[i].direction;
        const struct gesture *gesture = decode(report, sizeof(report));
        assert(gesture && gesture->key == expected[i].key && gesture->modifier == expected[i].modifier);
        uint8_t mra_report[MRA_GESTURE_REPORT_SIZE] = {0};
        memcpy(mra_report, report, sizeof(report));
        assert(decode(mra_report, sizeof(mra_report)) == gesture);
        assert(!decode(mra_report, sizeof(mra_report) - 1));
        assert(emit(stream[1], gesture) == 0);

        struct input_event events[6];
        size_t key_down = gesture->modifier ? 1 : 0;
        size_t count = gesture->modifier ? 6 : 4;
        size_t bytes = count * sizeof(events[0]);
        assert(read(stream[0], events, bytes) == (ssize_t)bytes);
        assert(events[key_down].type == EV_KEY && events[key_down].code == gesture->key &&
               events[key_down].value == 1);
        assert(events[key_down + 1].type == EV_SYN && events[key_down + 1].code == SYN_REPORT);
        assert(events[key_down + 2].type == EV_KEY && events[key_down + 2].code == gesture->key &&
               events[key_down + 2].value == 0);
        assert(events[count - 1].type == EV_SYN && events[count - 1].code == SYN_REPORT);
        if (gesture->modifier) {
            assert(events[0].type == EV_KEY && events[0].code == gesture->modifier && events[0].value == 1);
            assert(events[4].type == EV_KEY && events[4].code == gesture->modifier && events[4].value == 0);
        }
    }
    assert(!decode(report, 0) && !decode(report, 8));
    report[1] = 3;
    report[2] = 3;
    assert(!decode(report, sizeof(report)));
    report[1] = 0xff;
    assert(!decode(report, sizeof(report)));
    report[0] = 0x01;
    report[1] = 6;
    assert(!decode(report, sizeof(report)));
    assert(emit(-1, &gestures[0]) == -1 && errno == EBADF);

    int hid[2];
    assert(socketpair(AF_UNIX, SOCK_SEQPACKET, 0, hid) == 0);
    sigset_t mask;
    sigemptyset(&mask);
    sigaddset(&mask, SIGTERM);
    assert(sigprocmask(SIG_BLOCK, &mask, NULL) == 0);
    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        close(hid[1]);
        int signals = signalfd(-1, &mask, SFD_CLOEXEC);
        assert(signals >= 0);
        _exit(run(hid[0], signals, stream[1]) == 0 ? 0 : 1);
    }

    alarm(5);
    const uint8_t volume_report[MRA_GESTURE_REPORT_SIZE] = {0x0e, 4, 1};
    assert(write(hid[1], report, sizeof(report)) == sizeof(report));
    assert(write(hid[1], volume_report, sizeof(volume_report)) == sizeof(volume_report));
    struct input_event events[4];
    assert(read(stream[0], events, sizeof(events)) == sizeof(events));
    assert(events[0].code == KEY_VOLUMEUP);
    assert(kill(child, SIGTERM) == 0);
    int status;
    assert(waitpid(child, &status, 0) == child && WIFEXITED(status) && WEXITSTATUS(status) == 0);

    close(hid[1]);
    int signals = signalfd(-1, &mask, SFD_CLOEXEC);
    assert(signals >= 0);
    assert(run(hid[0], signals, stream[1]) == -1 && errno == ENODEV);
    alarm(0);
    puts("PASS: 9 gesture mappings, invalid reports, evdev frames, I/O error, idle SIGTERM, disconnect");
}
