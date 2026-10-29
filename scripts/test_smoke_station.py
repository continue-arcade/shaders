"""分发冒烟自身的关键失败路径，使用真实子进程与固定公开密钥。"""

import base64
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import smoke_station


class SmokeFailureTests(unittest.TestCase):
    def test_public_key_identity_must_match_supplied_der(self):
        der = bytes.fromhex("302a300506032b6570032100") + bytes(range(32))
        valid = {"key_id": "ed25519:" + hashlib.sha256(der).hexdigest(),
                 "public_key_der": base64.b64encode(der).decode()}
        smoke_station.validate_public_key(valid)
        for bad in [
            {**valid, "public_key_der": base64.b64encode(der[:-1] + b"!").decode()},
            {**valid, "public_key_der": "invalid-base64"},
            {**valid, "key_id": "ed25519:" + "0" * 64},
        ]:
            with self.assertRaises(ValueError):
                smoke_station.validate_public_key(bad)

    @unittest.skipUnless(os.name == "posix", "此故障进程 fixture 使用 POSIX shebang")
    def test_failed_ready_reaps_the_actual_child_process(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "bad-core"
            binary.write_text(f"#!{sys.executable}\nimport time\nprint('not-json', flush=True)\ntime.sleep(30)\n")
            binary.chmod(0o755)
            children = []
            popen = subprocess.Popen

            def start(*args, **kwargs):
                child = popen(*args, **kwargs)
                children.append(child)
                return child

            try:
                with patch("smoke_station.subprocess.Popen", side_effect=start):
                    with self.assertRaises(ValueError):
                        smoke_station.Core(binary, root)
                self.assertEqual(len(children), 1)
                self.assertIsNotNone(children[0].poll(), "握手失败后留下了 Core 子进程")
            finally:
                for child in children:
                    if child.poll() is None:
                        child.terminate()
                        child.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()

class FamilyMetadataTests(unittest.TestCase):
    def test_expected_metadata_comes_from_each_actual_cart(self):
        import zipfile
        with tempfile.TemporaryDirectory() as directory:
            for family in ['dot', 'sharp-bilinear']:
                path = Path(directory) / (family + '.cart')
                with zipfile.ZipFile(path, 'w') as archive:
                    archive.writestr('cartfile.toml', f'id="org.continue.shaders.{family}"\nversion="0.1.0-smoke.1"\n')
                    archive.writestr('continue.toml', f'id="org.continue.shader.{family}"\nversion="0.1.0-smoke.1"\n[shader.variants.standard]\nparameters="parameters/standard.json"\n')
                    archive.writestr('parameters/standard.json', '{"parameters": [{"id":"original_id","variant_default":0.5,"source_initial":0.25,"min":0,"max":1,"step":0.1}]}')
                    archive.writestr('ui.json', '{"parameters": {"original_id":{"label":{"en":"Control","zh-CN":"参数"},"description":{"en":"Meaning","zh-CN":"说明"}}}}')
                expected = smoke_station.expected_metadata(path)
                self.assertEqual(expected['cart_id'], 'org.continue.shaders.'+family)
                self.assertEqual(expected['resource_id'], 'org.continue.shader.'+family)
                self.assertEqual(expected['variants']['standard'][0]['id'], 'original_id')
