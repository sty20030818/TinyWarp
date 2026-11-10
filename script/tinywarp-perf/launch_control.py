#!/usr/bin/env python3
"""复制测量包、隔离数据并等待 UI 授权，完成后台三轮后正常退出。"""

import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time

from fixtures import SETTINGS, RC


def launch(candidate, binary, work):
    assert candidate.is_dir() and binary.is_file()
    work.mkdir(mode=0o700)
    temporary = Path(tempfile.mkdtemp(prefix='twp-api-', dir='/private/tmp'))
    home = Path(tempfile.mkdtemp(prefix='twp-api-home-', dir='/private/tmp'))
    discovery = Path(tempfile.mkdtemp(prefix='twp-api-discovery-', dir='/private/tmp'))
    app = temporary / 'TinyWarp.app'
    shutil.copytree(candidate, app, symlinks=True)
    shutil.copy2(binary, app / 'Contents/MacOS/warp-oss')
    info_path = app / 'Contents/Info.plist'
    info = plistlib.loads(info_path.read_bytes())
    info.update(CFBundleIdentifier=f'dev.warp.TinyWarpL3AControl{time.time_ns()}',
                CFBundleName='TinyWarp Perf Control', CFBundleDisplayName='TinyWarp Perf Control',
                CFBundleURLTypes=[])
    info_path.write_bytes(plistlib.dumps(info))
    repository = Path(__file__).resolve().parents[2]
    subprocess.run(['/usr/bin/codesign', '--force', '--deep', '--sign', '-', '--entitlements',
                    str(repository / 'script/Debug-Entitlements.plist'), str(app)], check=True)
    subprocess.run(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    config = home / '.warp-oss'
    config.mkdir(mode=0o700)
    (config / 'settings.toml').write_text(SETTINGS)
    (home / '.zshrc').write_text(RC)
    for name in ['.zshenv', '.zprofile', '.zlogin']:
        (home / name).touch()
    real_home = Path.home()
    protected = [real_home / p for p in ['.warp-oss', '.warp-preview', '.warp',
                 'Library/Application Support/dev.warp.WarpOss',
                 'Library/Application Support/dev.warp.Warp-Preview',
                 'Library/Preferences/dev.warp.WarpOss.plist',
                 'Library/Preferences/dev.warp.Warp-Preview.plist', 'Library/Group Containers']]
    profile = work / 'isolation.sb'
    profile.write_text('(version 1)\n(allow default)\n(deny file*\n' +
                       '\n'.join(' (subpath ' + json.dumps(str(p)) + ')' for p in protected) + ')\n')
    env = dict(os.environ, HOME=str(home), CFFIXED_USER_HOME=str(home), ZDOTDIR=str(home),
               SHELL='/bin/zsh', TMPDIR=str(home), WARP_LOCAL_CONTROL_DISCOVERY_DIR=str(discovery))
    for key in ['WARP_DATA_PROFILE', 'WARP_INTEGRATION']:
        env.pop(key, None)
    with (work / 'application.log').open('wb') as log:
        process = subprocess.Popen(['/usr/bin/sandbox-exec', '-f', str(profile),
                                    str(app / 'Contents/MacOS/warp-oss'),
                                    'warposs://settings?q=warpctrl'], env=env, cwd=home,
                                   stdout=log, stderr=subprocess.STDOUT)
        try:
            state = dict(pid=process.pid, app=str(app), home=str(home),
                         discovery=str(discovery), app_id=info['CFBundleIdentifier'])
            state_path = work / 'state.json'
            state_path.write_text(json.dumps(state, indent=2))
            print('请在 TinyWarp Perf Control 设置中启用 warpctrl CLI，无需安装命令。', flush=True)
            deadline = time.monotonic() + 600
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError('测试包提前退出')
                records = [json.loads(p.read_text()) for p in discovery.glob('*.json')]
                records = [r for r in records if r.get('pid') == process.pid
                           and r.get('app_id') == state['app_id']
                           and r.get('endpoint') and r.get('credential_broker')]
                if len(records) == 1:
                    state['instance_id'] = records[0]['instance_id']
                    state_path.write_text(json.dumps(state, indent=2))
                    break
                time.sleep(0.5)
            else:
                raise TimeoutError('等待 UI 授权超时')
            subprocess.run([sys.executable, '-B', str(Path(__file__).with_name('background.py')),
                            '--state', str(state_path), '--output', str(work / 'results')], check=True)
        finally:
            if process.poll() is None:
                subprocess.run([str(repository / '.note/l3a/control'), str(app), 'quit'],
                                capture_output=True, timeout=10, check=True)
                process.wait(timeout=30)
            print('测试包已退出，保留临时样本和测量记录。', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--work', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    launch(args.app.resolve(), args.binary.resolve(), args.work.resolve())
