# Troubleshooting HonorKit

## No scanner or no service

Start here as the account that will use GNOME:

```bash
lsusb -d 10a5:a900
fprintd-list "$USER"
systemctl status fprintd.service
journalctl -b -u fprintd.service --no-pager
```

| Result | What it tells you |
| --- | --- |
| `lsusb` lists `10a5:a900` | The kernel can enumerate the USB reader. The product string shows its firmware. |
| `fprintd-list` shows no device | Check stock support and the service journal. A USB listing alone does not imply libfprint support. |
| The reader is shown with no enrolled prints | The hardware is visible to fprintd. Use HonorKit or `fprintd-enroll` to add a finger. |
| HonorKit reports that fprintd is unavailable | Check the package, D-Bus activation, and the driver drop-in. If you see this only in a private test bus, start HonorKit from your regular GNOME session. |
| The scanner is busy | Close another enrollment dialog or verify command, then reload HonorKit. |
| The service becomes inactive while idle | fprintd normally exits after inactivity. A later D-Bus call launches it again. |

## The private driver fails to load

See which override systemd resolved with `systemctl cat fprintd.service`. Check
the loader paths and vendor dependency after starting the service:

```bash
systemctl show fprintd.service -p Environment -p MainPID
readelf -dW /opt/honor-mra-a900/lib/libfprint-2.so.2
pid=$(systemctl show fprintd.service -p MainPID --value)
if [ "$pid" -gt 0 ]; then
    sudo grep -E 'honor-mra-a900|libfpcbep' "/proc/$pid/maps"
fi
```

A boot may number the USB bus or device differently. Use the vendor and product
IDs as identity. The build rejects a changed Lenovo archive digest. If an update
breaks the custom ABI, revert the drop-in using the
[removal instructions](../fingerprint/INSTALL.md#remove) and rebuild against a
reviewed compatible release.

## Enrollment and verification

- After a scan, fully lift the finger before the next touch. Keep a steady touch
  and vary the angle between samples.
- A completed enrollment writes a template. Verify with the enrolled finger and
  a distinct finger to distinguish a real match from a UI success message.
- A mismatch can happen on the correct finger. Retry with the same finger
  centered on the reader. Repeated failures warrant checking the journal and
  testing against the recorded firmware and driver revision.
- If GNOME offers password sign-in but no fingerprint prompt, first test
  `fprintd-verify`. Then check that GDM has its separate PAM module and that
  GNOME fingerprint authentication is enabled. Keep the global PAM option
  disabled if sudo must continue using password authentication.

## Touchpad gestures

```bash
systemctl status honor-mra-gestures.service
journalctl -b -u honor-mra-gestures.service --no-pager
readlink -f /dev/honor-mra-touchpad
udevadm info --query=property --name=/dev/honor-mra-touchpad
```

The udev rule activates a static unit. Seeing `static` in systemctl is expected.
On the tested laptop the HID descriptor advertised report `0x0e` as 9 bytes,
while real gesture reports were 31 bytes. The daemon accepts both lengths.

If the virtual device exists but no actions occur, the optional read-only
`sudo python3 -I -B gesture/diagnose.py` counts HID report IDs, lengths and
the emitted virtual key codes for 35 seconds. Run it while making the gestures.
It reads the specific touchpad and its virtual keyboard. If there are OEM
reports but no virtual keys, inspect the daemon mapping. If virtual keys
arrive, inspect GNOME's shortcuts and the active desktop session.

The [hardware report](REPORT.ru.md) contains observed results and
[gesture installation](../gesture/INSTALL.md) contains the rollback.
