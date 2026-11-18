#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only

from pathlib import Path
import runpy
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch


SYSTEM = Path(__file__).resolve().parent
HELPER = runpy.run_path(str(SYSTEM / "honorkit-system"))["install_grub"].__globals__
PACKAGE_NORMAL = HELPER["GRUB_PACKAGE_NORMAL"]
STOCK_CONFIG = b"""set default=0
set timeout=5
menuentry 'Kali' {
    linux /boot/vmlinuz root=UUID=example ro
    initrd /boot/initrd.img
}
menuentry 'Windows' {
    chainloader /EFI/Microsoft/Boot/bootmgfw.efi
}
"""


@unittest.skipUnless(
    PACKAGE_NORMAL.is_file() and HELPER["digest"](PACKAGE_NORMAL.read_bytes()) == HELPER["NORMAL_HASH"],
    "requires the supported Kali GRUB module",
)
class FrierenInstallerTest(unittest.TestCase):
    def setUp(self):
        folder = self.enterContext(tempfile.TemporaryDirectory())
        root = Path(folder)
        self.cfg = root / "grub.cfg"
        self.theme = root / "honorkit-frieren-theme"
        self.drop_in = root / "frieren.cfg"
        self.normal = root / "normal.mod"
        self.backup = root / "backup.mod"
        self.cfg.write_bytes(STOCK_CONFIG)
        self.original = PACKAGE_NORMAL.read_bytes()
        self.normal.write_bytes(self.original)
        self.expected_patch = HELPER["patched_normal"](self.original)
        sources = root / "data"
        shutil.copytree(SYSTEM / "frieren", sources / "frieren")
        for font in ("menu.pf2", "small.pf2", "title.pf2"):
            shutil.copyfile(SYSTEM.parent / "build" / "frieren" / font, sources / "frieren" / font)
        header = (
            "### BEGIN /etc/grub.d/00_header ###\n"
            "insmod gfxmenu\nterminal_output gfxterm\n"
            f"set theme=($root){self.theme}/theme.txt\n"
        ).encode()
        for font in ("menu.pf2", "small.pf2", "title.pf2"):
            header += f"loadfont ($root){self.theme}/{font}\n".encode()
        self.generated_config = header + b"### END /etc/grub.d/00_header ###\n" + STOCK_CONFIG
        self.enterContext(patch.dict(HELPER, {
            "DATA": sources,
            "GRUB_CFG": self.cfg,
            "GRUB_THEME": self.theme,
            "GRUB_DROP_IN": self.drop_in,
            "GRUB_NORMAL": self.normal,
            "GRUB_NORMAL_BACKUP": self.backup,
            "GRUB_BACKUPS": root,
            "check_grub_efi": lambda: None,
            "run": self.run_grub_command,
        }))

    def run_grub_command(self, *args):
        if args[0] == "/usr/sbin/grub-mkconfig":
            args[2].write_bytes(self.generated_config if self.drop_in.exists() else STOCK_CONFIG)
        else:
            subprocess.run(args, check=True)

    def test_install_and_remove_restore_managed_module(self):
        HELPER["install_grub"]()
        HELPER["install_grub"]()
        self.assertEqual(HELPER["grub_status"](), "installed")
        self.assertEqual(self.cfg.read_bytes(), self.generated_config)
        self.assertEqual(self.backup.read_bytes(), self.original)
        self.assertEqual(self.normal.read_bytes(), self.expected_patch)
        HELPER["remove_grub"]()
        self.assertEqual(self.cfg.read_bytes(), STOCK_CONFIG)
        self.assertEqual(self.normal.read_bytes(), self.original)
        self.assertFalse(self.backup.exists())
        self.assertFalse(self.theme.exists())

    def test_remove_preserves_preexisting_patch(self):
        self.normal.write_bytes(self.expected_patch)
        HELPER["install_grub"]()
        self.assertFalse(self.backup.exists())
        HELPER["remove_grub"]()
        self.assertEqual(self.normal.read_bytes(), self.expected_patch)

    def test_remove_preserves_module_from_newer_package(self):
        HELPER["install_grub"]()
        updated = b"updated module"
        self.normal.write_bytes(updated)
        self.assertEqual(HELPER["grub_status"](), "repair")
        HELPER["remove_grub"]()
        self.assertEqual(self.normal.read_bytes(), updated)

    def test_failed_candidate_restores_original_state(self):
        self.generated_config = self.generated_config.replace(b"set timeout=5", b"set timeout=0")
        with self.assertRaises(ValueError):
            HELPER["install_grub"]()
        self.assertEqual(self.cfg.read_bytes(), STOCK_CONFIG)
        self.assertEqual(self.normal.read_bytes(), self.original)
        self.assertFalse(self.drop_in.exists())
        self.assertFalse(self.backup.exists())
        self.assertFalse(self.theme.exists())

    def test_rejects_unknown_module(self):
        self.normal.write_bytes(b"unknown module")
        with self.assertRaises(ValueError):
            HELPER["install_grub"]()
        self.assertEqual(self.cfg.read_bytes(), STOCK_CONFIG)
        self.assertFalse(self.drop_in.exists())
        self.assertFalse(self.backup.exists())

    def test_core_with_embedded_normal_is_detected(self):
        for modules, embeds in [((), False), (("normal",), True)]:
            with self.subTest(modules=modules):
                image = self.cfg.with_suffix(".efi")
                subprocess.run([
                    "grub-mkimage", "-O", "x86_64-efi", "-p", "/boot/grub", "-o", image,
                    "part_gpt", "ext2", *modules,
                ], check=True)
                self.assertEqual(HELPER["embeds_normal"](image.read_bytes()), embeds)


if __name__ == "__main__":
    unittest.main()
