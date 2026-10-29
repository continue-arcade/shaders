#!/usr/bin/env python3
"""通过真实 Station HTTP 和 Core JSON-RPC 在隔离空间验证 Shader Cart 分发。"""

import argparse
import base64
import hashlib
import json
import selectors
import subprocess
import time
import urllib.parse
import urllib.request
from pathlib import Path


def get(url):
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read()


def validate_public_key(public_key):
    """即使复用 Core 内置信任根，也核对独立提供的整份公开密钥。"""
    try:
        der = base64.b64decode(public_key["public_key_der"], validate=True)
        expected = "ed25519:" + hashlib.sha256(der).hexdigest()
        if len(der) != 44 or not der.startswith(bytes.fromhex("302a300506032b6570032100")):
            raise ValueError("不是 Ed25519 SPKI 公钥")
        if public_key["key_id"] != expected:
            raise ValueError("公钥 DER 与 key_id 不符")
    except (KeyError, TypeError, ValueError) as error:
        raise ValueError("独立 Station 公钥无效") from error


class Core:
    def __init__(self, binary, root):
        self.log = (root / "core.stderr.log").open("w")
        try:
            self.process = subprocess.Popen(
                [str(binary), "--space", str(root / "space"), "--create",
                 "--user-config-root", str(root / "user"),
                 "--device-config-root", str(root / "device"),
                 "serve", "--exit-on-eof"],
                stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=self.log, text=True, bufsize=1,
            )
        except BaseException:
            self.log.close()
            raise
        self.sequence = 0
        self.transcript = []
        self.selector = selectors.DefaultSelector()
        try:
            self.selector.register(self.process.stdout, selectors.EVENT_READ)
            ready = self.read()
            if ready.get("type") != "ready" or ready.get("context") is None:
                raise RuntimeError(f"隔离 Core 启动失败：{ready}")
        except BaseException:
            self.close(abort=True)
            raise

    def read(self):
        if not self.selector.select(timeout=60):
            raise TimeoutError("Core 在 60 秒内没有返回响应")
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError("Core 提前退出，请检查 core.stderr.log")
        return json.loads(line)

    def call(self, method, params=None):
        self.sequence += 1
        request = {"jsonrpc": "2.0", "id": self.sequence,
                   "method": method, "params": params}
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        response = self.read()
        self.transcript.append({"request": request, "response": response})
        if response.get("id") != self.sequence or "error" in response:
            raise RuntimeError(f"{method} 失败：{response}")
        return response["result"]

    def close(self, abort=False):
        if abort and self.process.poll() is None:
            self.process.terminate()
        try:
            self.process.stdin.close()
        except BrokenPipeError:
            pass
        try:
            self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=10)
            raise RuntimeError("Core 正常关闭超时")
        finally:
            self.selector.close()
            self.process.stdout.close()
            self.log.close()
        if not abort and self.process.returncode != 0:
            raise RuntimeError(f"Core 退出码：{self.process.returncode}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--station-url", required=True)
    parser.add_argument("--public-key", type=Path, required=True,
                        help="从本地 Station public-key 命令独立取得的公钥 JSON")
    parser.add_argument("--core", type=Path, required=True)
    parser.add_argument("--cart-tool", type=Path, required=True)
    parser.add_argument("--expected-cart", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    base = args.station_url.rstrip("/") + "/"
    parsed = urllib.parse.urlparse(base)
    if parsed.scheme != "http" or parsed.hostname not in {"localhost", "127.0.0.1", "::1"}:
        raise ValueError("本脚本仅验证明确指定的本地回环 Station")
    public_key = json.loads(args.public_key.read_text())
    validate_public_key(public_key)
    args.out.mkdir(parents=True, exist_ok=False)
    expected = hashlib.sha256(args.expected_cart.read_bytes()).hexdigest()
    cart_id = "org.continue.shaders.zfast-crt"
    version = "0.1.0-smoke.1"
    descriptor = json.loads(get(urllib.parse.urljoin(base, "station.json")))
    release = json.loads(get(urllib.parse.urljoin(
        base, f"api/v1/carts/{cart_id}/releases/{version}")))["release"]
    artifact = next(a for a in release["artifacts"] if a["state"] == "published")
    if artifact["bundle_sha256"].removeprefix("sha256:") != expected:
        raise AssertionError("Station 制品摘要与本地 Cart 不同")
    envelope = json.loads(get(urllib.parse.urljoin(base, "api/v1/index")))
    if envelope["key_id"] != public_key["key_id"]:
        raise AssertionError("Station 签名身份与独立取得的公钥不符")
    downloaded = get(urllib.parse.urljoin(
        base, f"api/v1/artifacts/{artifact['cart_artifact']}/download"))
    if hashlib.sha256(downloaded).hexdigest() != expected:
        raise AssertionError("HTTP 下载字节摘要不符")
    download_path = args.out / "downloaded.cart"
    download_path.write_bytes(downloaded)
    verification = json.loads(subprocess.check_output(
        [str(args.cart_tool), "verify", str(download_path)], text=True))
    core = Core(args.core, args.out)
    try:
        sources = core.call("carts.sources.list")["sources"]
        source = next((s for s in sources
                       if s["url"] == urllib.parse.urljoin(base, "station.json")), None)
        if source is None:
            source = core.call("carts.sources.add", {
                "url": base,
                "trustedKeys": {public_key["key_id"]: public_key["public_key_der"]},
            })
        opened = core.call("carts.station.open", {"sourceId": source["sourceId"]})
        context = {"sourceId": source["sourceId"], "contextId": opened["contextId"]}
        document = core.call("carts.station.browse", {
            "context": context, "cartId": cart_id, "cartRelease": version,
            "releases": False, "query": None, "page": 1,
        })["document"]
        if document["release"]["cart_release"] != version:
            raise AssertionError("正式 Core API 浏览的版本不符")
        sentinel = args.out / "space/saves/shader-smoke/user.txt"
        sentinel.parent.mkdir(parents=True, exist_ok=True)
        sentinel.write_text("preserve-user-data\n")
        task = core.call("carts.station.install", {
            "context": context, "cartId": cart_id,
            "cartRelease": version, "cartArtifact": artifact["cart_artifact"],
        })
        deadline = time.monotonic() + 60
        while True:
            state = core.call("carts.station.tasks.get", {
                "context": context, "taskId": task["taskId"],
            })
            if state["state"] != "running":
                if state["state"] != "completed":
                    raise AssertionError(f"正式安装任务失败：{state}")
                break
            if time.monotonic() > deadline:
                raise TimeoutError("正式安装任务超时")
            time.sleep(0.1)
        installed = core.call("carts.station.status.list", {
            "context": context, "cartIds": [cart_id],
        })["carts"]
        if len(installed) != 1 or installed[0]["state"] != "stored":
            raise AssertionError(f"安装登记不符：{installed}")
        if installed[0]["bundleSha256"] != expected:
            raise AssertionError("正式 Core 的安装摘要不符")
        artifact_root = args.out / "space/carts/station-artifacts" / expected
        manifests = list(artifact_root.rglob("continue.toml"))
        if not manifests:
            raise AssertionError("安装没有产生真实资源 manifest")
        core.call("carts.station.remove", {"context": context, "cartId": cart_id})
        remaining = core.call("carts.station.installations.list")["carts"]
        if any(row["cartId"] == cart_id for row in remaining):
            raise AssertionError("卸载后安装登记仍存在")
        if sentinel.read_text() != "preserve-user-data\n" or not artifact_root.is_dir():
            raise AssertionError("卸载改动了用户数据或不可变制品")
        report = {"passed": True, "station_id": descriptor["id"],
                  "cart_id": cart_id, "cart_release": version,
                  "cart_artifact": artifact["cart_artifact"],
                  "bundle_sha256": expected, "signing_key_id": envelope["key_id"],
                  "checks": ["real-station-http", "download-sha256", "continue-cart-verify",
                             "core-browse", "core-signed-index-install", "stored-registration",
                             "real-resource-unpack", "uninstall-preserves-user-data"],
                  "not_verified": ["shader-gpu-rendering", "cross-platform", "all-25-selections"],
                  "verification": verification}
        (args.out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps(report, ensure_ascii=False, indent=2))
    finally:
        (args.out / "core-transcript.json").write_text(
            json.dumps(core.transcript, ensure_ascii=False, indent=2) + "\n")
        core.close()


if __name__ == "__main__":
    main()
