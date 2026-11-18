# HonorKit

Touchpad and biometric support for the HONOR MagicBook Art 14 (MRA-XXX), with a small Rust/GTK 4 app for managing fingerprints through fprintd.

| Component | Hardware or interface | Tested on |
| --- | --- | --- |
| Touchpad gestures | I²C HID `35cc:0104` | Kali Rolling amd64, kernel `6.19.14+kali-amd64`, GNOME Wayland |
| Fingerprint driver | FPC `10a5:a900`, `L:0002 FW:22.26.2.43` | Kali `libfprint 1.94.10` base and `fprintd 1.94.5-4+b2` |
| Fingerprint app | Standard fprintd D-Bus API | GNOME Wayland and GTK 4 |

## Fingerprint app

The app lists fingerprints for the current user, guides enrollment with an animated fingerprint and progress, and deletes one selected fingerprint after confirmation. It uses the desktop's existing Polkit authorization and respects GTK's animation setting. Run it as a regular user in your graphical session.

On Kali, install `libgtk-4-dev`, `pkgconf` and `make`. Use Rust 1.92 or later and GTK 4.10 or later. Then build HonorKit and add its icon to the current user's GNOME application menu:

```bash
make install-app
```

Open **HonorKit** from GNOME or type `honorkit` in a graphical session. The command opens the same window. For a one-off run from source, use `cargo run --locked --manifest-path fingerprint-app/Cargo.toml`. Run `make uninstall-app` to remove the user-level launcher and executable.

The interface is in Russian. The enrollment animation runs inside HonorKit. GNOME's login and lock screens use their existing interface.

[Русское руководство пользователя](docs/USAGE.ru.md) · [Troubleshooting](docs/TROUBLESHOOTING.md) · [Architecture and security boundaries](docs/ARCHITECTURE.md)

## Touchpad

`make gestures test-gestures` builds the C daemon and checks the gesture mappings, both observed HID report lengths, and shutdown behavior. The daemon reads the OEM reports and emits keyboard events. udev starts the systemd service when the matching touchpad appears.

[Touchpad installation and removal](gesture/INSTALL.md)

## Fingerprint driver

Start by checking your normal distribution support with `lsusb` and `fprintd-list "$USER"`. The custom build is for the FPC A900 and was tested on the firmware above. It uses a private library directory for fprintd. Lenovo's matcher is downloaded from Lenovo and checked against pinned SHA-256 values at build time. It is closed-source and excluded from this repository.

[Fingerprint driver build, installation, GNOME integration and removal](fingerprint/INSTALL.md)

## Test notes and credits

[Hardware validation report in Russian](docs/REPORT.ru.md). It includes the physical gesture and fingerprint results, reboot checks, limitations and upstream references.

- The gesture daemon derives from [MadhiasM/honor-magicbook-art-touchpad-gestures](https://github.com/MadhiasM/honor-magicbook-art-touchpad-gestures), GPL-3.0. [HonorControl](https://github.com/ZachAR3/HonorControl) provided a second implementation and HID descriptor reference.
- The fingerprint patches derive from [libfprint MR 396](https://gitlab.freedesktop.org/libfprint/libfprint/-/merge_requests/396), [cityji's FPC A900 driver](https://github.com/cityji/honor-magickbookpro-fingerprint-driver), and [benjaminjamesxyz/fingerprint-sensor](https://github.com/benjaminjamesxyz/fingerprint-sensor), LGPL-2.1-or-later as applicable to the libfprint sources and patches.
- The Rust app and gesture changes use GPL-3.0. See [licenses](licenses).
