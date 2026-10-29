#!/usr/bin/env python3
"""登记驱动的单家族草稿候选；只创建，不覆盖任何已有 tag 或 Release。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
import urllib.parse

SLUG = re.compile(r'[a-z0-9]+(?:-[a-z0-9]+)*', re.ASCII)


def plan(root, family, out, revision):
    if not SLUG.fullmatch(family) or len(family) > 64:
        raise ValueError('非法家族 slug')
    if not re.fullmatch(r'[0-9a-f]{40}', revision):
        raise ValueError('候选必须指向准确的 40 位源码 revision')
    registry = tomllib.loads((root / 'registry' / (family + '.toml')).read_text())
    report_path = out / (family + '-report.json')
    report = json.loads(report_path.read_text())
    for key in ['family', 'version', 'cart_id', 'resource_id', 'tag_prefix', 'release_notes']:
        if report[key] != registry[key]:
            raise ValueError('报告与登记不符: ' + key)
    version = registry['version']
    # SemVer 已经由 Rust build 严格验证；此处进一步禁止路径、选项和稳定版误发。
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+-[0-9A-Za-z.-]+(?:\+[0-9A-Za-z.-]+)?', version):
        raise ValueError('本工作流只创建 prerelease 版本候选')
    if registry['family'] != family or registry['tag_prefix'] != family + '/':
        raise ValueError('候选家族或 tag 前缀不符')
    if registry['cart_id'] != 'org.continue.shaders.' + family or registry['resource_id'] != 'org.continue.shader.' + family:
        raise ValueError('候选身份不符')
    tag = family + '/v' + version
    if report['tag'] != tag:
        raise ValueError('候选 tag 不符')
    notes = 'docs/releases/' + family + '-' + version + '.md'
    if registry['release_notes'] != notes or not (root / notes).read_text().strip():
        raise ValueError('缺少当前家族版本更新说明')
    cart = out / (family + '-' + version + '.cart')
    if report['bundle_sha256'] != 'sha256:' + hashlib.sha256(cart.read_bytes()).hexdigest():
        raise ValueError('候选 Cart 摘要不符')
    return [tag, str(cart), str(report_path), str(root / notes), '--draft', '--prerelease',
            '--target', revision, '--title', family + ' ' + version,
            '--notes-file', str(root / notes)]


def require_absent(endpoint):
    result = subprocess.run(['gh', 'api', endpoint], text=True, capture_output=True)
    if result.returncode == 0:
        raise RuntimeError('已存在，拒绝覆盖: ' + endpoint)
    if '(HTTP 404)' not in result.stderr:
        raise RuntimeError('无法确认发布目标不存在: ' + result.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--family', required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    release_args = plan(Path('.'), args.family, args.out, args.revision)
    repo = os.environ['GH_REPO']
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', repo):
        raise ValueError('非法 GH_REPO')
    tag = release_args[0]
    require_absent('repos/' + repo + '/git/ref/tags/' + urllib.parse.quote(tag, safe='/'))
    require_absent('repos/' + repo + '/releases/tags/' + urllib.parse.quote(tag, safe=''))
    subprocess.run(['gh', 'release', 'create', *release_args], check=True)


if __name__ == '__main__':
    main()
