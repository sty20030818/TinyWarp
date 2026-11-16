#!/usr/bin/env python3
"""使用实际应用包检查 Dock 插件安装和缺失资源的失败行为。"""
import argparse
import os
import plistlib
from pathlib import Path
import shutil
import subprocess
import tempfile

HELPER = Path(__file__).with_name("bundle_dock_tile_plugin")
PLUGIN = "WarpDockTilePlugin.docktileplugin"


def invoke(app, *args, error=None):
    result = subprocess.run([str(HELPER), str(app), *map(str, args)], capture_output=True, text=True)
    if error is None:
        assert result.returncode == 0, result.stderr
    else:
        assert result.returncode != 0 and error in result.stderr, result.stderr


def write_info(path, info):
    path.write_bytes(plistlib.dumps(info, sort_keys=False))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path)
    app = parser.parse_args().app.resolve(strict=True)
    invoke(app, "--check")
    with tempfile.TemporaryDirectory(prefix="tinywarp-icon-package-test-") as temporary:
        fixture = Path(temporary) / "TinyWarp Icon Fixture.app"
        contents = fixture / "Contents"
        (contents / "MacOS").mkdir(parents=True)
        info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
        info["CFBundleIdentifier"] = "dev.warp.TinyWarp.IconPackageFixture"
        info.pop("NSDockTilePlugIn")
        write_info(contents / "Info.plist", info)
        executable = "Contents/MacOS/" + info["CFBundleExecutable"]
        os.link(app / executable, fixture / executable)
        invoke(fixture, app / "Contents/PlugIns" / PLUGIN)
        invoke(fixture, app / "Contents/PlugIns" / PLUGIN, error="目标插件已存在")
        metadata_path = contents / "PlugIns" / PLUGIN / "Contents/Info.plist"
        metadata = plistlib.loads(metadata_path.read_bytes())
        assert metadata["MainAppBundleIdentifier"] == info["CFBundleIdentifier"]
        metadata["NSPrincipalClass"] = "WrongEntryClass"
        write_info(metadata_path, metadata)
        invoke(fixture, "--check", error="入口类不存在")
        metadata["NSPrincipalClass"] = plistlib.loads(
            (app / "Contents/PlugIns" / PLUGIN / "Contents/Info.plist").read_bytes()
        )["NSPrincipalClass"]
        write_info(metadata_path, metadata)
        neon = contents / "PlugIns" / PLUGIN / "Contents/Resources/neon.png"
        neon.unlink()
        invoke(fixture, "--check", error="neon.png")
    print("5 项实际打包检查通过：完整包、预构建安装、重复安装拒绝、入口类错误、缺失图标。")


if __name__ == "__main__":
    main()
