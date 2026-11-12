"""TinyWarp 构建和分发版本规则；公开版本与本地 dev.N 版本共享此入口。"""
import argparse
import json
from pathlib import Path
import plistlib
import re

ROOT = Path(__file__).resolve().parent.parent
NUMBER = r"(?:0|[1-9][0-9]*)"
TAG = re.compile(rf"v({NUMBER}\.{NUMBER}\.{NUMBER})(?:-dev\.([1-9][0-9]*))?")


def parse_tag(tag):
    if not isinstance(tag, str) or len(tag) > 128:
        raise ValueError("无效 TinyWarp 版本")
    match = TAG.fullmatch(tag)
    if not match:
        raise ValueError("版本必须为 vX.Y.Z 或 vX.Y.Z-dev.N")
    numbers = match[1].split(".") + ([match[2]] if match[2] else [])
    if any(int(number) > 2**64 - 1 for number in numbers):
        raise ValueError("版本数字超限")
    return match[1]


def load_config(path=ROOT / "tinywarp-version.json"):
    value = json.loads(Path(path).read_text())
    if set(value) != {"version", "build"}:
        raise ValueError("构建信息只允许 version 和 build")
    parse_tag("v" + value["version"])
    if "-" in value["version"] or type(value["build"]) is not int or not 0 < value["build"] <= 2**64 - 1:
        raise ValueError("产品版本应为 X.Y.Z，构建号应为正整数")
    return value


def bundle_version(app):
    app = Path(app)
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    tag = info.get("WarpVersion", "")
    base = parse_tag(tag)
    if info.get("CFBundleShortVersionString") != base:
        raise ValueError("macOS 产品版本与更新版本不一致")
    build = info.get("CFBundleVersion", "")
    if not isinstance(build, str) or not re.fullmatch(r"[1-9][0-9]*", build):
        raise ValueError("macOS 构建号必须为正整数")
    metadata = json.loads((app / "Contents/Resources/bundled/metadata/version.json").read_text())
    if metadata.get("warp_version") != tag:
        raise ValueError("资源版本与应用版本不一致")
    return tag


def stamp_plist(path, tag, config):
    base = parse_tag(tag)
    if base != config["version"]:
        raise ValueError("GIT_RELEASE_TAG 与 tinywarp-version.json 不一致")
    path = Path(path)
    value = plistlib.loads(path.read_bytes())
    value.update(CFBundleShortVersionString=base, CFBundleVersion=str(config["build"]))
    path.write_bytes(plistlib.dumps(value))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--development", type=int, help="本地 dev.N 序号")
    parser.add_argument("--field", choices=["tag", "version", "build"], default="tag")
    parser.add_argument("--check-tag")
    parser.add_argument("--stamp-plist", type=Path)
    parser.add_argument("--tag")
    args = parser.parse_args()
    try:
        config = load_config()
        tag = "v" + config["version"]
        if args.development is not None:
            tag += f"-dev.{args.development}"
        parse_tag(tag)
        if args.check_tag:
            if parse_tag(args.check_tag) != config["version"]:
                raise ValueError("版本标签与产品版本不一致")
            tag = args.check_tag
        if args.stamp_plist:
            if not args.tag:
                raise ValueError("写入 plist 必须明确提供 --tag")
            stamp_plist(args.stamp_plist, args.tag, config)
        else:
            print({"tag": tag, "version": config["version"], "build": config["build"]}[args.field])
    except (ValueError, TypeError, OSError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
