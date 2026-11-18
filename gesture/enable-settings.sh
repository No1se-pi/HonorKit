#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

daemon=/usr/local/libexec/honor-mra-gesture-daemon
backup=/var/backups/honorkit-gesture-daemon.pre-settings
helper=/usr/local/libexec/honorkit-touchpad-config
policy=io.github.no1se.HonorKit.touchpad.policy
policy_path=/usr/share/polkit-1/actions/$policy
unit=honor-mra-gestures.service

if [[ ! -x "$daemon" ]]; then
    printf '%s\n' 'Install the gesture service before upgrading its settings support.' >&2
    exit 1
fi
for destination in "$backup" "$helper" "$policy_path" "$daemon.new"; do
    if [[ -e "$destination" || -L "$destination" ]]; then
        printf 'Destination already exists: %s\n' "$destination" >&2
        exit 1
    fi
done
test -x build/honor-mra-gesture-daemon
test -x build/honorkit-touchpad-config
sudo -v
sudo -n install -m 0755 -- "$daemon" "$backup"

rollback() {
    code=$?
    if ((code != 0)); then
        sudo -n install -m 0755 -- "$backup" "$daemon.new" &&
            sudo -n mv -fT -- "$daemon.new" "$daemon" || true
        sudo -n systemctl restart "$unit" || true
        sudo -n unlink "$helper" 2>/dev/null || true
        sudo -n unlink "$policy_path" 2>/dev/null || true
    fi
}
trap rollback EXIT

sudo -n install -m 0755 -- build/honor-mra-gesture-daemon "$daemon.new"
sudo -n install -m 0755 -- build/honorkit-touchpad-config "$helper"
sudo -n install -m 0644 -- "gesture/$policy" "$policy_path"
sudo -n mv -fT -- "$daemon.new" "$daemon"
sudo -n systemctl restart "$unit"
systemctl is-active "$unit"
