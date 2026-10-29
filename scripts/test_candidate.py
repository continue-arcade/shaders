"""候选发布的输入、跨家族混入和已有 tag 拒绝测试。"""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import subprocess


class CandidateTests(unittest.TestCase):
    def module(self):
        import candidate
        return candidate

    def test_family_input_rejected_before_files_or_processes(self):
        candidate = self.module()
        for family in ["../dot", "dot;echo hacked", "dot\n", "Dot", "--help", ""]:
            with self.assertRaises(ValueError):
                candidate.plan(Path('/missing'), family, Path('/missing'), 'a' * 40)

    def test_plan_uses_exact_family_artifact_digest_and_notes(self):
        candidate = self.module()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'registry').mkdir(); (root / 'dist').mkdir()
            notes = 'docs/releases/dot-0.1.0-smoke.1.md'
            (root / notes).parent.mkdir(parents=True); (root / notes).write_text('Dot 冒烟')
            (root / 'registry/dot.toml').write_text('family="dot"\nversion="0.1.0-smoke.1"\ntag_prefix="dot/"\ncart_id="org.continue.shaders.dot"\nresource_id="org.continue.shader.dot"\nrelease_notes="'+notes+'"\n')
            cart = root / 'dist/dot-0.1.0-smoke.1.cart'; cart.write_bytes(b'cart')
            report = dict(family='dot', version='0.1.0-smoke.1', tag_prefix='dot/', tag='dot/v0.1.0-smoke.1', cart_id='org.continue.shaders.dot', resource_id='org.continue.shader.dot', release_notes=notes, bundle_sha256='sha256:'+hashlib.sha256(b'cart').hexdigest())
            report_path = root / 'dist/dot-report.json'; report_path.write_text(json.dumps(report))
            args = candidate.plan(root, 'dot', root/'dist', 'a'*40)
            self.assertEqual(args[0], 'dot/v0.1.0-smoke.1')
            cart.write_bytes(b'tampered')
            with self.assertRaises(ValueError): candidate.plan(root, 'dot', root/'dist', 'a'*40)
            cart.write_bytes(b'cart'); report['cart_id']='org.continue.shaders.zfast-crt'; report_path.write_text(json.dumps(report))
            with self.assertRaises(ValueError): candidate.plan(root, 'dot', root/'dist', 'a'*40)

    def test_existing_release_or_tag_and_network_errors_are_not_overwritten(self):
        candidate = self.module()
        for code, stderr in [(0,''), (1,'network failure'), (1,'HTTP 403')]:
            with patch('candidate.subprocess.run', return_value=subprocess.CompletedProcess([],code,'',stderr)):
                with self.assertRaises(RuntimeError): candidate.require_absent('repos/a/b/git/ref/tags/dot/v1.0.0')
        with patch('candidate.subprocess.run', return_value=subprocess.CompletedProcess([],1,'','gh: Not Found (HTTP 404)')):
            candidate.require_absent('repos/a/b/git/ref/tags/dot/v1.0.0')
