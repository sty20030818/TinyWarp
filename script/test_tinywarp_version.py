import json
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest

from tinywarp_version import ROOT, bundle_version, load_config, parse_tag, stamp_plist


class TinyWarpVersionTests(unittest.TestCase):
    def test_tag_policy(self):
        for tag in ["v0.1.0", "v0.10.0", "v0.1.0-dev.1"]:
            self.assertTrue(parse_tag(tag))
        for tag in ["0.1.0", "v0.01.0", "v0.1.0-dev.0", "v0.1.0-dev.01", "v0.1.0-beta.1", "v0.1.0+build.1", "v0.1.0/../escape", "v18446744073709551616.0.0"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                parse_tag(tag)

    def test_config_rejects_invalid_builds_and_extra_fields(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "version.json"
            for value in [{"version": "0.1.0", "build": False}, {"version": "0.1.0", "build": 0}, {"version": "0.1.0-dev.1", "build": 1}, {"version": "0.1.0", "build": 1, "extra": 2}]:
                path.write_text(json.dumps(value))
                with self.subTest(value=value), self.assertRaises(ValueError):
                    load_config(path)

    def bundle(self, home, tag):
        app = Path(home) / "TinyWarp.app"
        path = app / "Contents/Info.plist"
        path.parent.mkdir(parents=True)
        path.write_bytes(plistlib.dumps(dict(CFBundleIdentifier="dev.warp.WarpOss", WarpVersion=tag, CFBundleShortVersionString="0.0.0", CFBundleVersion="0", Preserved=True)))
        metadata = app / "Contents/Resources/bundled/metadata/version.json"
        metadata.parent.mkdir(parents=True)
        metadata.write_text(json.dumps({"warp_version": tag}))
        return app, path

    def test_numeric_macos_fields_and_metadata_agree(self):
        with tempfile.TemporaryDirectory() as temporary:
            app, path = self.bundle(temporary, "v0.1.0-dev.1")
            stamp_plist(path, "v0.1.0-dev.1", {"version": "0.1.0", "build": 1})
            self.assertEqual(bundle_version(app), "v0.1.0-dev.1")
            value = plistlib.loads(path.read_bytes())
            self.assertEqual(value["CFBundleShortVersionString"], "0.1.0")
            self.assertEqual(value["CFBundleVersion"], "1")
            self.assertTrue(value["Preserved"])
            value["CFBundleVersion"] = "1-dev"
            path.write_bytes(plistlib.dumps(value))
            with self.assertRaises(ValueError):
                bundle_version(app)

    @unittest.skipUnless(sys.platform == "darwin", "需要 macOS plutil")
    def test_tinywarp_plist_and_original_warp_channel_are_separate(self):
        version = load_config()["version"]
        for tag, features, expected in [(f"v{version}-dev.1", "tinywarp_gui", version), ("v0.2026.11.12.00.30.stable_02", "gui", "0.2026.11.12.00.30.02")]:
            with tempfile.TemporaryDirectory() as temporary:
                app, path = self.bundle(temporary, tag)
                value = plistlib.loads(path.read_bytes())
                del value["WarpVersion"]
                path.write_bytes(plistlib.dumps(value))
                env = dict(os.environ, GIT_RELEASE_TAG=tag, WARP_CHANNEL="oss", FEATURES=features, WARP_PLIST_PATH=str(path), WARP_PLIST_NO_FILE_TYPES="1", WARP_SCHEME_NAME="")
                subprocess.run([str(ROOT / "script/update_plist")], cwd=ROOT, env=env, check=True, capture_output=True)
                self.assertEqual(plistlib.loads(path.read_bytes())["CFBundleShortVersionString"], expected)

    def test_publisher_rejects_dev_before_loading_a_private_key(self):
        with tempfile.TemporaryDirectory() as temporary:
            app, path = self.bundle(temporary, "v0.1.0-dev.1")
            stamp_plist(path, "v0.1.0-dev.1", {"version": "0.1.0", "build": 1})
            output = Path(temporary) / "release"
            result = subprocess.run([str(ROOT / "script/tinywarp-release"), str(app), "--output", str(output), "--key", str(Path(temporary) / "missing-key")], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("开发包", result.stderr)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
