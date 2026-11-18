# FPC A900 on Kali Rolling

The procedure installs the distro's fprintd and places the custom libraries under `/opt/honor-mra-a900/lib`. A systemd drop-in applies that search path to fprintd. The tested stack uses Kali amd64 and an FPC `10a5:a900` with `FW:22.26.2.43`. The patched driver checks the sensor ID and its expected geometry before processing scans.

## Build as a regular user

Review APT's plan on a rolling system before approving development dependencies:

```bash
sudo apt-get install --no-install-recommends git curl unzip build-essential \
  meson ninja-build pkgconf libglib2.0-dev libgio-2.0-dev-bin \
  libgusb-dev libusb-1.0-0-dev binutils
bash fingerprint/build.sh
sha256sum -c fingerprint/stage.sha256
```

The build pins libfprint and the reviewed patches, validates the Lenovo ZIP and its x86-64 matcher, then builds and runs the libfprint tests locally. Build outputs appear under `fingerprint/stage`. The vendor matcher and build outputs are ignored by Git.

## Deploy for a sensor test

Check the target paths before installing. The commands below assume you are installing this drop-in for the first time.

```bash
sudo apt-get install --no-install-recommends fprintd libfprint-2-2
sudo install -d -m 0755 /opt/honor-mra-a900/lib /etc/systemd/system/fprintd.service.d
sudo install -m 0644 fingerprint/stage/libfprint-2.so.2.0.0 fingerprint/stage/libfpcbep.so /opt/honor-mra-a900/lib/
sudo ln -s libfprint-2.so.2.0.0 /opt/honor-mra-a900/lib/libfprint-2.so.2
sudo install -m 0644 fingerprint/10-honor-mra-a900.conf /etc/systemd/system/fprintd.service.d/10-honor-mra-a900.conf
sudo systemctl daemon-reload
sudo systemctl restart fprintd.service
fprintd-list "$USER"
```

In a GNOME terminal run `fprintd-enroll -f right-index-finger` or use the [Rust app](../README.md#fingerprint-app). Check that enrollment finishes. Run `fprintd-verify -f right-index-finger` with the registered finger and again with a different one. Expect `verify-match` and `verify-no-match`. Repeat after reboot. The system daemon activates on demand via D-Bus.

## GNOME login and unlock

After fingerprint tests pass, check that the distribution supplies a separate GDM fingerprint profile and save PAM config before adding the distro module. On the tested Kali system `/etc/pam.d/gdm-fingerprint` already references `pam_fprintd.so` and the package's global fingerprint profile defaults to disabled.

```bash
mkdir -p build
find /etc/pam.d -maxdepth 1 -type f -exec sha256sum {} + >build/pam-before.sha256
sudo install -d -m 0700 /var/backups/honor-mra-pam
sudo cp -a /etc/pam.d /var/backups/honor-mra-pam/
sudo apt-get install --no-install-recommends libpam-fprintd
sha256sum -c build/pam-before.sha256
```

Keep the global fingerprint profile disabled in `pam-auth-update` so that PAM callers such as `sudo` still use their existing authentication. Validate the GDM password path separately. The utility itself requires only fprintd and Polkit.

## Remove

Remove only profiles you intend to delete. The app can delete one selected print. `fprintd-delete "$USER"` deletes all prints for the current user.

1. If you installed `libpam-fprintd` for this setup, purge it and compare PAM files with the pre-installation checksums and backup.
2. Stop fprintd and remove the private driver artifacts:

   ```bash
   sudo systemctl stop fprintd.service
   sudo rm -f /etc/systemd/system/fprintd.service.d/10-honor-mra-a900.conf \
     /opt/honor-mra-a900/lib/libfprint-2.so.2 \
     /opt/honor-mra-a900/lib/libfprint-2.so.2.0.0 /opt/honor-mra-a900/lib/libfpcbep.so
   sudo rmdir /opt/honor-mra-a900/lib /opt/honor-mra-a900
   sudo systemctl daemon-reload
   ```

3. If both distro packages were added for this setup, review the plan for `sudo apt-get purge fprintd libfprint-2-2`. Otherwise the service will resume using the regular library. Check `/var/lib/fprint` and `/var/log/fpc` before removing test data. Keep or remove the PAM backup separately.

The observed hardware test, vendor archive digest and upstream references are in the [report](../docs/REPORT.ru.md).
