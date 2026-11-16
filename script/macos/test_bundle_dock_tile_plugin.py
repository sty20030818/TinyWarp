#!/usr/bin/env python3
"""检查 Dock 插件包；可传真实应用，或无参创建最小原生包 fixture。"""
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


def check_cases(app):
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


def create_fixture(root):
    app = root / "TinyWarp CI Fixture.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    write_info(
        contents / "Info.plist",
        {
            "CFBundleIdentifier": "dev.warp.TinyWarp.IconPackageCI",
            "CFBundleExecutable": "warp-oss",
        },
    )
    source = root / "fixture.c"
    source.write_text("int main(void) { return 0; }\n")
    subprocess.run(
        ["/usr/bin/xcrun", "clang", str(source), "-o", str(contents / "MacOS/warp-oss")],
        check=True,
    )
    invoke(app)
    return app


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("app", type=Path, nargs="?", help="已有真实应用包；省略时仅验证最小原生包")
    app = parser.parse_args().app
    if app is not None:
        check_cases(app.resolve(strict=True))
    else:
        print("使用最小原生包验证插件与资源，不构建或运行 TinyWarp 产品。", flush=True)
        with tempfile.TemporaryDirectory(prefix="tinywarp-icon-ci-") as temporary:
            check_cases(create_fixture(Path(temporary)))


if __name__ == "__main__":
    main()
