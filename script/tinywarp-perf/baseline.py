#!/usr/bin/env python3
"""在独立 macOS 应用包和 HOME 中建立 GUI 终端资源基线。"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import signal
import statistics
import subprocess
import time

from fixtures import SETTINGS, RC


def command(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs).stdout


def process_tree(root_pid):
    rows = {}
    for line in command(['/bin/ps', '-Ao', 'pid=,ppid=,comm=']).splitlines():
        fields = line.split(None, 2)
        if len(fields) == 3:
            rows[int(fields[0])] = (int(fields[1]), fields[2])
    owned = {root_pid}
    while True:
        expanded = owned | {pid for pid, (parent, _) in rows.items() if parent in owned}
        if expanded == owned:
            break
        owned = expanded
    return {pid: rows[pid][1] for pid in owned if pid in rows}


def summarize(samples):
    result = {}
    for group in ['application', 'children']:
        rows = [x[group] for x in samples]
        elapsed = samples[-1]['time'] - samples[0]['time']
        result[group] = {
            'rss_median_bytes': statistics.median(x['rss'] for x in rows),
            'rss_sampled_peak_bytes': max(x['rss'] for x in rows),
            'footprint_median_bytes': statistics.median(x['footprint'] for x in rows),
            'footprint_sampled_peak_bytes': max(x['footprint'] for x in rows),
            'cpu_percent_one_core': (rows[-1]['cpu_ns'] - rows[0]['cpu_ns']) / (elapsed * 1e7),
        }
    return result


class Baseline:
    def __init__(self, repository, seconds):
        self.repository = repository.resolve()
        self.work = self.repository / '.note/l3a'
        self.app = self.repository / '.note/candidates/L3A/release/bundle/osx/TinyWarp.app'
        self.seconds = seconds
        self.pid = None
        self.procedure_version = 6
        self.home = None
        self.run = None

    def control(self, operation, text=None):
        args = [str(self.work / 'control'), str(self.app), operation]
        if operation in {'new-tab', 'focus-input', 'split', 'close'}:
            self.wait_foreground()
        if text is not None:
            args.append(text)
        result = subprocess.run(args, capture_output=True, text=True)
        if result.returncode:
            (self.run / 'control-error.json').write_text(json.dumps({'operation': operation, 'exit': result.returncode, 'details': result.stdout}))
            raise RuntimeError('测试菜单前置条件失败，详见 control-error.json')
        return json.loads(result.stdout) if result.stdout.strip() else None

    def shells(self):
        return {pid for pid, path in process_tree(self.pid).items() if Path(path).name in {'zsh', '-zsh'}}

    def ready_shells(self):
        marker = self.home / 'prompt-ready'
        ready = {int(line.split()[0]) for line in marker.read_text().splitlines()} if marker.exists() else set()
        return self.shells() & ready

    def wait(self, check, timeout=45):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError('隔离应用提前退出，请检查本轮独立日志')
            if check():
                return
            time.sleep(0.15)
        raise TimeoutError('测量前置状态未达到，保留本轮记录')

    def wait_foreground(self):
        if not self.control('info')['active']:
            self.control('activate')
        self.wait(lambda: (lambda info: info['active'] and len(info['windows']) == 1)(self.control('info')), timeout=300)

    def sample(self, label, seconds=None):
        for attempt in range(1, 4):
            self.wait_foreground()
            pin = self.control('display-info')
            (self.run / (label + '-display.json')).write_text(json.dumps(pin))
            time.sleep(3)
            result = self.sample_once(label, seconds)
            if result is not None:
                return result
            print(json.dumps({'discarded_sampling_window': label, 'attempt': attempt}), flush=True)
        raise RuntimeError('连续三次采样失去前台，保留本轮状态后停止')

    def sample_once(self, label, seconds=None):
        start_window = self.control('info')
        (self.run / (label + '-window-start.json')).write_text(json.dumps(start_window))
        if not start_window['active'] or not start_window['windows']:
            return None
        samples = []
        previous_cpu = {}
        cumulative_cpu = {'application': 0, 'children': 0}
        deadline = time.monotonic() + (seconds or self.seconds)
        while time.monotonic() < deadline or len(samples) < 2:
            paths = process_tree(self.pid)
            metrics = json.loads(command([str(self.work / 'metrics'), *map(str, sorted(paths))]))
            data = {'time': time.monotonic(), 'application': {'rss': 0, 'footprint': 0, 'cpu_ns': 0}, 'children': {'rss': 0, 'footprint': 0, 'cpu_ns': 0}}
            for metric in metrics:
                group = 'application' if Path(paths[metric['pid']]).name == 'warp-oss' else 'children'
                for key in ['rss', 'footprint']:
                    data[group][key] += metric[key]
                if metric['pid'] in previous_cpu:
                    cumulative_cpu[group] += metric['cpu_ns'] - previous_cpu[metric['pid']]
                previous_cpu[metric['pid']] = metric['cpu_ns']
            for group in cumulative_cpu:
                data[group]['cpu_ns'] = cumulative_cpu[group]
            samples.append(data)
            time.sleep(1)
        end_window = self.control('info')
        (self.run / (label + '-window-end.json')).write_text(json.dumps(end_window))
        expected_pin = json.loads((self.run / (label + '-display.json')).read_text())
        def pinned(state):
            if len(state['windows']) != 1:
                return False
            bounds = state['windows'][0]['bounds']
            left, top = expected_pin['display_x'], expected_pin['display_y']
            return (abs(bounds['Width'] - expected_pin['target_width']) <= 1
                    and abs(bounds['Height'] - expected_pin['target_height']) <= 1
                    and bounds['X'] >= left and bounds['Y'] >= top
                    and bounds['X'] + bounds['Width'] <= left + expected_pin['display_width']
                    and bounds['Y'] + bounds['Height'] <= top + expected_pin['display_height'])
        if not end_window['active'] or not end_window['windows'] or not pinned(start_window) or not pinned(end_window):
            print(json.dumps({'foreground_validation_failed': end_window}), flush=True)
            return None
        (self.run / (label + '-window-state.json')).write_text(json.dumps({'start': start_window, 'end': end_window}))
        (self.run / (label + '-samples.json')).write_text(json.dumps(samples))
        result = summarize(samples)
        (self.run / (label + '.json')).write_text(json.dumps(result, indent=2))
        print(json.dumps({'scenario': label, 'result': result}), flush=True)
        return result

    def cycle(self, number):
        self.run = self.work / f'run-{number}'
        self.run.mkdir(mode=0o700)
        self.home = self.run / 'home'
        self.home.mkdir(mode=0o700)
        config = self.home / '.warp-oss'
        config.mkdir(mode=0o700)
        settings = SETTINGS
        (config / 'settings.toml').write_text(settings)
        rc = RC
        (self.home / '.zshrc').write_text(rc)
        (self.home / '.zprofile').touch()
        (self.home / '.zshenv').touch()
        (self.home / '.zlogin').touch()
        assert str(self.app / 'Contents/MacOS/warp-oss') not in {x.strip() for x in command(['/bin/ps', '-Ao', 'comm=']).splitlines()}, '旧测试实例尚未退出，不修改包身份'
        info_path = self.app / 'Contents/Info.plist'
        info = plistlib.loads(info_path.read_bytes())
        info['CFBundleIdentifier'] = f'dev.warp.TinyWarpL3ARun{number}Epoch{time.time_ns()}'
        info['CFBundleName'] = 'TinyWarp Perf'
        info['CFBundleDisplayName'] = 'TinyWarp Perf'
        info['CFBundleURLTypes'] = []
        plistlib.dump(info, info_path.open('wb'))
        command(['/usr/bin/codesign', '--force', '--deep', '--sign', '-', '--entitlements', str(self.repository / 'script/Debug-Entitlements.plist'), str(self.app)])
        command(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(self.app)])
        environment = dict(os.environ, HOME=str(self.home), CFFIXED_USER_HOME=str(self.home), ZDOTDIR=str(self.home), SHELL='/bin/zsh', TMPDIR=str(self.run))
        environment.pop('WARP_DATA_PROFILE', None)
        environment.pop('WARP_INTEGRATION', None)
        output = (self.run / 'application-output.log').open('wb')
        started = time.monotonic()
        self.process = subprocess.Popen(['/usr/bin/sandbox-exec', '-f', str(self.work / 'isolation.sb'), str(self.app / 'Contents/MacOS/warp-oss')], env=environment, cwd=self.home, stdout=output, stderr=subprocess.STDOUT)
        self.pid = self.process.pid
        try:
            self.wait(lambda: (self.home / 'prompt-ready').exists())
            startup = time.monotonic() - started
            self.control('activate')
            self.wait(lambda: bool(self.control('info')['windows']))
            (self.run / 'runtime.json').write_text(json.dumps({'procedure_version': self.procedure_version, 'startup_prompt_ready_seconds': startup, 'pid': self.pid, 'window': self.control('info'), 'fixture_settings_sha256': hashlib.sha256(settings.encode()).hexdigest()}, indent=2))
            self.wait(lambda: len(self.shells()) == 1)
            proof = command(['/usr/sbin/lsof', '-p', str(self.pid), '-Fn'])
            assert str(self.home) in proof
            forbidden = [str(Path.home() / name) + '/' for name in ['.warp-oss', '.warp-preview', 'Library/Application Support/dev.warp.WarpOss']]
            assert not any(p in proof for p in forbidden)
            (self.run / 'isolation-observed.json').write_text(json.dumps({'own_home_files_open': True, 'protected_files_open': False, 'bundle_id': info['CFBundleIdentifier']}))
            print('等待 TinyWarp Perf 保持前台；点一下测试窗口即可，不用回聊天确认。', flush=True)
            self.wait(lambda: self.control('info')['active'], timeout=180)
            time.sleep(5)
            self.sample('idle')
            for count in range(2, 7):
                self.control('new-tab')
                self.wait(lambda: len(self.ready_shells()) == count)
                time.sleep(0.5)
            for count in [7, 8]:
                self.control('focus-input')
                time.sleep(0.3)
                self.control('split')
                self.wait(lambda: len(self.ready_shells()) == count)
                time.sleep(0.5)
            time.sleep(5)
            self.sample('six-tabs-eight-panes')
            for _ in range(7):
                self.control('close')
                time.sleep(0.4)
            release_started = time.monotonic()
            self.wait(lambda: len(self.shells()) == 1, timeout=90)
            multi_close_release_seconds = time.monotonic() - release_started
            time.sleep(5)
            self.sample('before-churn')
            for _ in range(20):
                previous = self.shells()
                self.control('new-tab')
                self.wait(lambda: bool(self.ready_shells() - previous))
                time.sleep(0.5)
                self.control('close')
                time.sleep(0.4)
            self.sample('after-churn-immediate')
            # 默认撤销关闭窗口为60秒，等待超过该窗口再判断资源是否归还。
            self.wait(lambda: len(self.shells()) == 1, timeout=90)
            time.sleep(5)
            self.sample('after-churn-settled')
            previous_output_shells = self.shells()
            self.control('new-tab')
            self.wait(lambda: len(self.ready_shells()) == 2)
            time.sleep(1)
            target_shell = (self.shells() - previous_output_shells).pop()
            output_started = time.monotonic()
            os.kill(target_shell, signal.SIGUSR1)
            self.wait(lambda: (self.home / 'output-done').exists())
            output_seconds = time.monotonic() - output_started
            time.sleep(3)
            self.sample('output-100000')
            response_started = time.monotonic()
            os.kill(target_shell, signal.SIGUSR2)
            self.wait(lambda: (self.home / 'fixture-response').exists())
            response_seconds = time.monotonic() - response_started
            (self.run / 'timings.json').write_text(json.dumps({'startup_prompt_ready_seconds': startup, 'output_to_shell_marker_seconds': output_seconds, 'fixture_signal_response_seconds': response_seconds, 'multi_close_release_seconds': multi_close_release_seconds, 'poll_sleep_seconds': 0.15, 'timing_excludes_final_render_ack': True, 'keyboard_and_scroll_latency_measured': False}, indent=2))
            if number == 3:
                app_pids = [pid for pid, path in process_tree(self.pid).items() if Path(path).name == 'warp-oss']
                (self.run / 'per-process-final.json').write_text(command([str(self.work / 'metrics'), *map(str, app_pids)]))
                for helper_pid in app_pids:
                    if helper_pid != self.pid:
                        helper_map = subprocess.run(['/usr/bin/vmmap', '-summary', str(helper_pid)], capture_output=True, text=True, timeout=30)
                        (self.run / 'helper-vmmap.txt').write_text(helper_map.stdout + helper_map.stderr)
                        (self.run / 'helper-vmmap-result.json').write_text(json.dumps({'exit': helper_map.returncode}))
                for name, args in [('vmmap', ['/usr/bin/vmmap', '-summary', str(self.pid)]), ('sample', ['/usr/bin/sample', str(self.pid), '2', '5', '-file', str(self.run / 'sample.txt')])]:
                    result = subprocess.run(args, capture_output=True, text=True, timeout=30)
                    (self.run / (name + '-result.json')).write_text(json.dumps({'exit': result.returncode}))
                    (self.run / (name + '-output.txt')).write_text(result.stdout + result.stderr)
            (self.run / 'complete').touch()
        finally:
            if self.process.poll() is None:
                self.control('quit')
                self.process.wait(timeout=30)
            output.close()
        print(json.dumps({'cycle_complete': number}), flush=True)


def self_test():
    data = [{'time': i + 1, 'application': {'rss': 10 + i, 'footprint': 20 + i, 'cpu_ns': i * 10_000_000}, 'children': {'rss': 1, 'footprint': 2, 'cpu_ns': 0}} for i in range(3)]
    result = summarize(data)
    assert result['application']['rss_median_bytes'] == 11
    assert result['application']['footprint_sampled_peak_bytes'] == 22
    assert result['application']['cpu_percent_one_core'] == 1
    assert result['children']['cpu_percent_one_core'] == 0
    print('summary_self_test_passed')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repository', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--cycle', type=int, choices=[1, 2, 3])
    parser.add_argument('--seconds', type=int, default=15)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    if args.self_test:
        self_test()
    else:
        assert args.cycle is not None and args.seconds >= 10
        os.umask(0o077)
        baseline = Baseline(args.repository, args.seconds)
        assert baseline.app.is_dir() and baseline.app.resolve() == baseline.app
        assert baseline.work.is_dir() and baseline.work.resolve() == baseline.work
        baseline.cycle(args.cycle)
