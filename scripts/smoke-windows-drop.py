"""Regression test for disk-backed drops on Windows, run with an isolated ClippO build."""
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import wave
from pathlib import Path

from playwright.sync_api import expect, sync_playwright


def connect(app, port):
    for _ in range(100):
        if app.poll() is not None:
            raise RuntimeError("Test instance exited before its WebView was ready")
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/version", timeout=.5) as response:
                return json.load(response)["webSocketDebuggerUrl"]
        except (urllib.error.URLError, TimeoutError):
            time.sleep(.1)
    raise RuntimeError("Timed out waiting for WebView2")


def exercise(page, folder):
    page.locator(".widget").wait_for()
    page.evaluate("""localStorage.setItem('clippo.preferences', JSON.stringify({
        language:'en', watch:false, teardrop:true, motion:'Off'
    }))""")
    page.reload()
    handle = page.get_by_role("button", name="Open ClippO", exact=True)
    expect(handle).to_be_visible()
    expect(page.locator(".widget")).to_have_css("width", "64px")
    client = page.context.new_cdp_session(page)

    def drag(event, file, x, y):
        data = {"items": [], "files": [str(file)], "dragOperationsMask": 1}
        client.send("Input.dispatchDragEvent", {"type": event, "x": x, "y": y, "data": data})

    for number in (1, 2):
        source = folder / f"Aufnahme ä {number}.WAV"
        with wave.open(str(source), "wb") as wav:
            wav.setparams((1, 2, 44100, 0, "NONE", ""))
            wav.writeframes(bytes(88200))
        drag("dragEnter", source, 32, 12)
        expect(page.locator(".widget")).to_have_css("width", "420px")
        expect(page.get_by_text("Convert to MP3", exact=True)).to_be_visible()
        drag("dragOver", source, 180, 100)
        drag("drop", source, 180, 100)
        output = source.with_name(source.stem + "-clippo.mp3")
        expect(page.get_by_text(output.name, exact=True)).to_be_visible()
        page.get_by_role("button", name="Save copy", exact=True).click()
        expect(page.locator(".notice-location")).to_contain_text(output.name)
        assert 0 < output.stat().st_size < source.stat().st_size
    page.get_by_role("button", name="Collapse to teardrop").click()
    expect(handle).to_be_visible()
    expect(page.locator(".widget")).to_have_css("width", "64px")
    image = folder / "Picture.png"
    shutil.copyfile(Path(__file__).resolve().parents[1] / "src-tauri/icons/icon.png", image)
    drag("dragEnter", image, 32, 12)
    expect(page.locator('[data-drop-action="convert"]')).to_be_visible()
    drag("dragOver", image, 160, 100)
    drag("dragOver", image, -40, -40)
    expect(handle).to_be_visible()
    drag("dragEnter", image, 32, 12)
    resize = page.locator('[data-drop-action="resize"]')
    expect(resize).to_be_visible()
    expect(page.locator(".widget")).to_have_css("width", "420px")
    box = resize.bounding_box()
    x, y = box["x"] + box["width"] / 2, box["y"] + box["height"] / 2
    drag("dragOver", image, x, y)
    drag("drop", image, x, y)
    expect(page.get_by_text("Resize image", exact=True)).to_be_visible()


def main():
    exe = Path(sys.argv[1]).resolve(strict=True)
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        folder = Path(tmp)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        env = dict(os.environ, WEBVIEW2_USER_DATA_FOLDER=str(folder / "profile"),
                   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=f"--remote-debugging-port={port} --remote-debugging-address=127.0.0.1")
        with subprocess.Popen([exe], env=env) as app:
            try:
                endpoint = connect(app, port)
                with sync_playwright() as playwright:
                    browser = playwright.chromium.connect_over_cdp(endpoint)
                    exercise(browser.contexts[0].pages[0], folder)
            finally:
                app.terminate()
                app.wait(timeout=10)


if __name__ == "__main__":
    main()
