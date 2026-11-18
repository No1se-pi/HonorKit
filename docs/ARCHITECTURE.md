# HonorKit architecture

HonorKit provides a Rust/GTK 4 application, a small C gesture daemon, and a
reproducible build for the FPC A900 libfprint driver. The three can be used
independently.

## Desktop application

The graphical application discovers the default reader through
`net.reactivated.Fprint.Manager.GetDefaultDevice` on the system D-Bus. The
device object's API covers listing, registration, and individual deletion.
An empty account name resolves to the app's account. GNOME's usual Polkit
agent mediates permission to modify saved fingerprints.

| Operation | D-Bus sequence |
| --- | --- |
| Refresh | `GetDefaultDevice` → `ListEnrolledFingers` |
| Enroll | `Claim` → check selected slot → subscribe to `EnrollStatus` → `EnrollStart` → `EnrollStop` → `Release` |
| Delete one | Confirm selection → `Claim` → `DeleteEnrolledFinger` → `Release` |

The UI waits asynchronously and disables competing operations during a claim.
A completed enrollment signal determines whether the app shows success.
Cancel and window close stop active registration, then release the claim.
The drawing code renders a generic fingerprint with Cairo. It never sees a
raw scan or a template. The stage count and status come from the daemon.
[Official fprintd device API](https://fprint.freedesktop.org/fprintd-dev/Device.html)

## Touchpad

udev matches I²C HID `35cc:0104` and exposes `/dev/honor-mra-touchpad`.
Systemd allows the gesture service to open that device for reading and
`/dev/uinput` for writing. The C daemon checks the device identity and maps
known vendor reports to a fixed set of keyboard events. GNOME handles the
resulting keys as brightness, volume, windows, notifications, and screenshots.
Ordinary pointer and multitouch events still go through the desktop's
normal input stack.

## Fingerprint driver

The Kali package supplies the system D-Bus service and default libfprint.
The service's drop-in selects the private libfprint and Lenovo matcher under
`/opt/honor-mra-a900/lib`. The pinned source and patches build the library
without the distro's unrelated drivers. The closed matcher processes images
on the host. Its vendor archive and extracted ELF are hashed before they
are used. The tested init geometry is `64 × 176`, HWID `0x331`.
The distro's fprintd service sandbox remains enabled.

The Rust app executes as the current user. The gesture daemon's only input
and output device access is the matched touchpad and uinput. The private
driver runs under the distro fprintd service. The package-supplied GDM PAM
profile is a separate, optional integration for login and unlock.

## Unlock feedback and touchpad settings

The per-user watcher observes `LockedHint` in the current graphical
logind session and a successful fprintd `VerifyStatus`. It correlates the
two events in a bounded interval and displays a short GTK card after the
session unlocks. GNOME and PAM still perform authentication. A standalone
verification in an already unlocked session does not trigger the card.
The watcher starts from an XDG autostart entry on GNOME.
[logind session API](https://www.freedesktop.org/software/systemd/man/latest/org.freedesktop.login1.html)

Touchpad settings use a small privileged executable. Polkit asks for
active-session administrator authentication. The executable accepts only a
gesture mask or one of three haptic levels. It writes the gesture mask
atomically under `/etc`. The existing daemon keeps its read-only hidraw
access. Haptic writes verify DMI model and HID identity and send one
fixed-format report. There is no firmware readback for that setting.
