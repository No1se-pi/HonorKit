#!/bin/bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
unit=honor-mra-gestures.service
rule=70-honor-mra-touchpad.rules
module=honor-mra-uinput.conf
binary=/usr/local/libexec/honor-mra-gesture-daemon
unit_path=/etc/systemd/system/$unit
rule_path=/etc/udev/rules.d/$rule
module_path=/etc/modules-load.d/$module

matches=()
for device in /sys/class/hidraw/hidraw*; do
    if grep -Fxq 'HID_ID=0018:000035CC:00000104' "$device/device/uevent"; then
        matches+=("$(readlink -f -- "$device")")
    fi
done
if (("${#matches[@]}" != 1)); then
    printf 'Expected one HONOR touchpad, found %s\n' "${#matches[@]}" >&2
    exit 1
fi
for destination in "$binary" "$unit_path" "$rule_path" "$module_path"; do
    if [[ -e "$destination" || -L "$destination" ]]; then
        printf 'Destination already exists: %s\n' "$destination" >&2
        exit 1
    fi
done

sudo -v
installed=()
rollback_on_error() {
    result=$?
    if ((result != 0 && ${#installed[@]} > 0)); then
        sudo -n systemctl stop "$unit" || true
        sudo -n rm -f -- "${installed[@]}" || true
        sudo -n systemctl daemon-reload || true
        sudo -n udevadm control --reload-rules || true
        sudo -n udevadm trigger --action=change --settle "${matches[0]}" || true
    fi
}
trap rollback_on_error EXIT

install_file() {
    sudo -n install -o root -g root -m "$1" -- "$2" "$3"
    installed+=("$3")
}

install_file 0755 build/honor-mra-gesture-daemon "$binary"
install_file 0644 "gesture/$unit" "$unit_path"
install_file 0644 "gesture/$rule" "$rule_path"
install_file 0644 "gesture/$module" "$module_path"
sudo -n modprobe uinput
sudo -n systemctl daemon-reload
sudo -n udevadm control --reload-rules
sudo -n udevadm trigger --action=add --settle "${matches[0]}"
sudo -n systemctl start "$unit"
systemctl is-active "$unit"
