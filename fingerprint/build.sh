#!/usr/bin/env bash
set -euo pipefail

cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
project=$PWD
if (( EUID == 0 )); then
    printf '%s\n' 'Build this driver as a regular user.' >&2
    exit 1
fi

mkdir -p build
work=$(mktemp -d "$project/build/fpc-a900.XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
mkdir "$work/vendor"

libfprint_commit=0c97a47d8ef405cd577b87058c1e89cae9d242e7
git clone --depth 1 --branch v1.94.10 \
    https://gitlab.freedesktop.org/libfprint/libfprint.git "$work/source"
test "$(git -C "$work/source" rev-parse HEAD)" = "$libfprint_commit"

archive="$work/r1slm02w.zip"
curl --fail --location --silent --show-error --proto '=https' --retry 2 \
    -o "$archive" https://download.lenovo.com/pccbbs/mobiles/r1slm02w.zip
printf '%s  %s\n' e725be53da4a6235b9621289386c44ecacb8d03a52c2d56d69904525fe6f7f3f "$archive" | sha256sum -c -
blob="$work/vendor/libfpcbep.so"
unzip -p "$archive" FPC_driver_linux_27.26.23.39/install_fpc/libfpcbep.so >"$blob"
printf '%s  %s\n' 5daa63bad068a3f97b5bb4d83de4d9fcceed6b4d8f1070ed3a00916abfbf664a "$blob" | sha256sum -c -

git -C "$work/source" apply --include='libfprint/drivers/fpcmoh/*' \
    "$project/fingerprint/patches/upstream-mr396.patch"
for patch in "$project"/fingerprint/patches/0*.patch; do
    git -C "$work/source" apply "$patch"
done
git -C "$work/source" apply "$project/fingerprint/port-v1.94.10.patch"
git -C "$work/source" apply "$project/fingerprint/limit-a900.patch"

export LD_LIBRARY_PATH="$work/vendor"
link_args="-L$work/vendor -Wl,-rpath-link,$work/vendor,-z,relro,-z,now"
meson setup "$work/obj" "$work/source" --wrap-mode=nodownload --prefix=/opt/honor-mra-a900 \
    -Ddrivers=fpcmoh -Dgtk-examples=false -Ddoc=false -Dintrospection=false \
    -Dudev_hwdb=disabled -Dudev_rules=disabled -Dinstalled-tests=false \
    -Db_lundef=true -Db_pie=true "-Dc_link_args=$link_args" "-Dcpp_link_args=$link_args"
meson compile -C "$work/obj"
meson test -C "$work/obj" --print-errorlogs
DESTDIR="$work/staged" meson install -C "$work/obj" --no-rebuild
lib=$(find "$work/staged" -type f -name libfprint-2.so.2.0.0 -print -quit)
test -n "$lib"
readelf -dW "$lib" >"$work/dynamic"
if grep -Eq '\((RPATH|RUNPATH)\)' "$work/dynamic"; then
    printf '%s\n' 'Refusing to stage a library with an embedded loader path.' >&2
    exit 1
fi

install -d fingerprint/stage
install -m 0644 "$lib" "$blob" fingerprint/stage/
ln -sfn libfprint-2.so.2.0.0 fingerprint/stage/libfprint-2.so.2
sha256sum fingerprint/stage/{libfprint-2.so.2.0.0,libfpcbep.so} >fingerprint/stage.sha256
