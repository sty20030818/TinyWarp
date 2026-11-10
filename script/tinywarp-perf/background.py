#!/usr/bin/env python3
"""测量已获 UI 授权的隔离控制实例；不激活窗口或安装系统 CLI。"""

import argparse
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import time

from baseline import command, process_tree, summarize


class Background:
    def __init__(self, repository, state_path, output):
        self.work = repository.resolve() / '.note/l3a'
        self.state = json.loads(state_path.read_text())
        self.app = Path(self.state['app']).resolve()
        self.home = Path(self.state['home']).resolve()
        self.pid = int(self.state['pid'])
        self.exe = self.app / 'Contents/MacOS/warp-oss'
        self.output = output.resolve()
        self.output.mkdir(mode=0o700)
        info = plistlib.loads((self.app / 'Contents/Info.plist').read_bytes())
        assert info['CFBundleIdentifier'] == self.state['app_id']
        assert self.state['app_id'].startswith('dev.warp.TinyWarpL3A')
        assert self.home.parent == Path('/private/tmp')
        assert self.home.name.startswith('twp-api-home-')
        assert Path(self.state['discovery']).resolve().parent == Path('/private/tmp')
        assert Path(process_tree(self.pid)[self.pid]).resolve() == self.exe
        self.env = dict(os.environ, HOME=str(self.home), CFFIXED_USER_HOME=str(self.home),
                        WARP_LOCAL_CONTROL_DISCOVERY_DIR=self.state['discovery'])
        version = self.api('app', 'version')
        assert version['app_id'] == self.state['app_id']
        assert version['instance_id'] == self.state['instance_id']
        windows = self.api('window', 'list')['windows']
        assert len(windows) == 1
        self.window = windows[0]['window_id']
        self.native('hide')
        self.assert_hidden()

    def native(self, op):
        result = command([str(self.work / 'control'), str(self.app), op])
        return json.loads(result) if result.strip() else None

    def api(self, *args):
        result = subprocess.run([str(self.exe), '--warpctrl', '--output-format', 'json',
                                 *args, '--pid', str(self.pid)], env=self.env, cwd=self.home,
                                capture_output=True, text=True, timeout=15)
        if result.returncode:
            raise RuntimeError(f'控制操作失败：{args[:2]}，退出码 {result.returncode}')
        data = json.loads(result.stdout)
        assert data.get('instance_id', self.state['instance_id']) == self.state['instance_id']
        return data

    def tabs(self):
        return self.api('tab', 'list', '--window', self.window)['tabs']

    def panes(self, tab):
        return self.api('pane', 'list', '--window', self.window, '--tab', tab)['panes']

    def create(self):
        return self.api('tab', 'create', '--type', 'terminal', '--window', self.window)['tab']['id']

    def close(self, tab):
        self.api('tab', 'close', '--window', self.window, '--tab', tab)

    def shells(self):
        return {pid for pid, path in process_tree(self.pid).items()
                if Path(path).name in {'zsh', '-zsh'}}

    def ready(self):
        marker = self.home / 'prompt-ready'
        pids = {int(line.split()[0]) for line in marker.read_text().splitlines()}
        return self.shells() & pids

    def wait(self, check, timeout=90):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            os.kill(self.pid, 0)
            if check():
                return
            time.sleep(0.15)
        raise TimeoutError('后台测量前置条件超时')

    def assert_hidden(self):
        state = self.native('info')
        assert state['pid'] == self.pid and state['hidden'] and not state['active']
        assert state['frontmost_pid'] != self.pid
        return state

    def sample(self, name):
        self.assert_hidden()
        samples, previous_cpu = [], {}
        cumulative = {'application': 0, 'children': 0}
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline or len(samples) < 2:
            state = self.assert_hidden()
            paths = process_tree(self.pid)
            metrics = json.loads(command([str(self.work / 'metrics'), *map(str, sorted(paths))]))
            row = {'time': time.monotonic(), 'hidden': state['hidden'],
                   'application': dict(rss=0, footprint=0, cpu_ns=0),
                   'children': dict(rss=0, footprint=0, cpu_ns=0)}
            for metric in metrics:
                pid = metric['pid']
                group = 'application' if Path(paths[pid]).resolve() == self.exe else 'children'
                for key in ['rss', 'footprint']:
                    row[group][key] += metric[key]
                if pid in previous_cpu:
                    cumulative[group] += metric['cpu_ns'] - previous_cpu[pid]
                previous_cpu[pid] = metric['cpu_ns']
            for group in cumulative:
                row[group]['cpu_ns'] = cumulative[group]
            samples.append(row)
            time.sleep(1)
        self.assert_hidden()
        (self.run / (name + '-samples.json')).write_text(json.dumps(samples))
        result = summarize(samples)
        (self.run / (name + '.json')).write_text(json.dumps(result, indent=2))
        print(json.dumps({'round': self.number, 'scenario': name, 'result': result}), flush=True)

    def cycle(self, number):
        self.number = number
        self.run = self.output / f'round-{number}'
        self.run.mkdir(mode=0o700)
        tabs = self.tabs()
        base = tabs[0]['tab_id']
        for tab in tabs[1:]:
            self.close(tab['tab_id'])
        self.wait(lambda: len(self.shells()) == 1)
        self.wait(lambda: len(self.ready()) == 1)
        assert len(self.panes(base)) == 1
        time.sleep(5)
        self.sample('idle')
        added = []
        for count in range(2, 7):
            added.append(self.create())
            self.wait(lambda: len(self.ready()) == count)
        for count in [7, 8]:
            self.api('pane', 'split', '--direction', 'right', '--window', self.window,
                     '--tab', added[-1])
            self.wait(lambda: len(self.ready()) == count)
        assert len(self.tabs()) == 6
        assert sum(len(self.panes(t['tab_id'])) for t in self.tabs()) == 8
        time.sleep(5)
        self.sample('six-tabs-eight-panes')
        for tab in added:
            self.close(tab)
        release_started = time.monotonic()
        self.wait(lambda: len(self.shells()) == 1)
        released = time.monotonic() - release_started
        time.sleep(5)
        self.sample('before-churn')
        for _ in range(20):
            previous = self.shells()
            tab = self.create()
            self.wait(lambda: bool(self.ready() - previous))
            self.close(tab)
        self.sample('after-churn-immediate')
        self.wait(lambda: len(self.shells()) == 1)
        time.sleep(5)
        self.sample('after-churn-settled')
        previous = self.shells()
        tab = self.create()
        self.wait(lambda: len(self.ready() - previous) == 1)
        target = (self.ready() - previous).pop()
        for name in ['output-done', 'fixture-response']:
            (self.home / name).unlink(missing_ok=True)
        start = time.monotonic()
        assert target in self.shells()
        os.kill(target, signal.SIGUSR1)
        self.wait(lambda: (self.home / 'output-done').exists())
        output_seconds = time.monotonic() - start
        time.sleep(3)
        self.sample('output-100000')
        start = time.monotonic()
        assert target in self.shells()
        os.kill(target, signal.SIGUSR2)
        self.wait(lambda: (self.home / 'fixture-response').exists())
        timings = {'output_to_shell_marker_seconds': output_seconds,
                   'fixture_signal_response_seconds': time.monotonic() - start,
                   'multi_close_release_seconds': released,
                   'poll_sleep_seconds': 0.15}
        (self.run / 'timings.json').write_text(json.dumps(timings, indent=2))
        self.close(tab)
        self.wait(lambda: len(self.shells()) == 1)
        (self.run / 'complete').touch()
        print(json.dumps({'round_complete': number}), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--repository', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--state', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    measurement = Background(args.repository, args.state, args.output)
    for number in range(1, 4):
        measurement.cycle(number)
    pids = [pid for pid, path in process_tree(measurement.pid).items()
            if Path(path).resolve() == measurement.exe]
    (measurement.output / 'per-process-final.json').write_text(
        command([str(measurement.work / 'metrics'), *map(str, pids)]))
    for pid in pids:
        result = subprocess.run(['/usr/bin/vmmap', '-summary', str(pid)],
                                capture_output=True, text=True, timeout=30)
        (measurement.output / f'vmmap-{pid}.txt').write_text(result.stdout + result.stderr)
    (measurement.output / 'complete').touch()
