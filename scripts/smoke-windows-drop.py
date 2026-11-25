"""Regression test for disk-backed drops on Windows, run with an isolated ClippO build."""
import json
import ctypes
from ctypes import wintypes
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


def exercise_fullscreen(playwright, pid):
    user32, gdi = ctypes.windll.user32, ctypes.windll.gdi32
    user32.FindWindowExW.argtypes = [wintypes.HWND, wintypes.HWND, wintypes.LPCWSTR, wintypes.LPCWSTR]
    user32.FindWindowExW.restype = wintypes.HWND
    user32.SetWindowPos.argtypes = [wintypes.HWND, wintypes.HWND, ctypes.c_int, ctypes.c_int,
                                    ctypes.c_int, ctypes.c_int, wintypes.UINT]
    gdi.CreateRectRgn.restype = wintypes.HANDLE
    user32.GetWindowRgn.argtypes = [wintypes.HWND, wintypes.HANDLE]
    gdi.DeleteObject.argtypes = [wintypes.HANDLE]
    def find_window(pid, class_name=None, title=None):
        hwnd = None
        while True:
            hwnd = user32.FindWindowExW(None, hwnd, class_name, title)
            assert hwnd, "Missing test window"
            owner = wintypes.DWORD()
            user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
            if owner.value == pid and user32.IsWindowVisible(hwnd):
                return hwnd

    hwnd = find_window(pid, title="ClippO")
    region = gdi.CreateRectRgn(0, 0, 0, 0)

    def expect_mask(hidden):
        for _ in range(60):
            if (user32.GetWindowRgn(hwnd, region) == 1) == hidden:  # NULLREGION
                return
            time.sleep(.1)
        raise AssertionError(f"Expected fullscreen mask to be {hidden}")

    browser = playwright.chromium.launch(channel="msedge", headless=False)
    try:
        rect = wintypes.RECT()
        assert user32.GetWindowRect(hwnd, ctypes.byref(rect))
        page = browser.new_page(no_viewport=True)
        page.set_content('<video></video><button onclick="document.querySelector(\'video\').requestFullscreen()">Play fullscreen</button>')
        cdp = page.context.new_cdp_session(page)
        window_id = cdp.send("Browser.getWindowForTarget")["windowId"]
        def bounds(**values):
            cdp.send("Browser.setWindowBounds", {"windowId": window_id, "bounds": values})
        bounds(windowState="normal")
        bounds(left=rect.left - 100, top=rect.top + 40, width=600, height=400)
        bounds(windowState="maximized")
        page.bring_to_front()
        processes = browser.new_browser_cdp_session().send("SystemInfo.getProcessInfo")["processInfo"]
        browser_pid = next(p["id"] for p in processes if p["type"] == "browser")
        fixture = find_window(browser_pid, class_name="Chrome_WidgetWin_1")
        # Keep the isolated browser visible even on a busy developer desktop.
        assert user32.SetWindowPos(fixture, -1, 0, 0, 0, 0, 0x0013)
        for _ in range(2):
            page.get_by_role("button", name="Play fullscreen").click()
            # CDP can emulate DOM fullscreen without resizing the native window.
            bounds(windowState="fullscreen")
            expect_mask(True)
            page.evaluate("document.exitFullscreen()")
            bounds(windowState="normal")
            bounds(windowState="maximized")
            expect_mask(False)
        page.get_by_role("button", name="Play fullscreen").click()
        bounds(windowState="fullscreen")
        expect_mask(True)
        browser.close()
        # Closing the fullscreen browser must return the drop as well.
        expect_mask(False)
    finally:
        browser.close()
        gdi.DeleteObject(region)


def exercise_fan_click(page):
    # Keep the smoke test independent of the runner's default image viewer.
    page.evaluate("""() => {
        const fetch = window.fetch;
        window.fanCommands = [];
        window.restoreFanFetch = () => { window.fetch = fetch; };
        window.fetch = (url, init) => {
            const command = /\\/(open_screenshot|drag_screenshot)$/.exec(String(url));
            if (command) {
                window.fanCommands.push({cmd: command[1], id: JSON.parse(init.body).id});
                return Promise.resolve(new Response('null', {
                    headers: {'Tauri-Response': 'ok', 'Content-Type': 'application/json'}
                }));
            }
            return fetch(url, init);
        };
    }""")
    card = page.locator(".fan-card").first
    card.click()
    card.press("Enter")
    box = card.bounding_box()
    x, y = box["x"] + box["width"] / 2, box["y"] + box["height"] / 2
    page.mouse.move(x, y)
    page.mouse.down()
    page.mouse.move(x + 12, y)
    page.mouse.up()
    commands = page.evaluate("""async () => {
        window.restoreFanFetch();
        await window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
            event: 'screenshot-drag-ended', payload: null
        });
        return window.fanCommands;
    }""")
    assert [c["cmd"] for c in commands] == ["open_screenshot", "open_screenshot", "drag_screenshot"], commands
    assert len({c["id"] for c in commands}) == 1


def exercise(page, folder, playwright, pid):
    page.locator(".widget").wait_for()
    page.evaluate("""localStorage.setItem('clippo.preferences', JSON.stringify({
        language:'en', watch:false, teardrop:true, motion:'Off'
    }))""")
    page.reload()
    handle = page.get_by_role("button", name="Open ClippO", exact=True)
    expect(handle).to_be_visible()
    expect(handle).to_have_css("height", "24px")
    assert handle.evaluate("el => getComputedStyle(el, '::before').height") == "20px"
    expect(page.locator(".widget")).to_have_css("width", "64px")
    # The narrow WebView must present a clear frame before its left edge moves.
    handle.evaluate("""el => el.addEventListener('click', () => {
        window.clearedBeforeResize = false;
        function sample() {
            if (innerHeight > 40) return;
            if (!document.querySelector('.teardrop-handle')?.getClientRects().length &&
                !document.querySelector('.titlebar')?.getClientRects().length) {
                window.clearedBeforeResize = true;
                return;
            }
            requestAnimationFrame(sample);
        }
        requestAnimationFrame(sample);
    }, {once: true})""")
    handle.click()
    panel = page.locator(".dock-expanded .widget")
    expect(panel).to_be_visible()
    expect(panel).to_have_css("width", "420px")
    assert page.evaluate("window.clearedBeforeResize")
    page.get_by_role("button", name="Collapse to teardrop").click()
    expect(handle).to_be_visible()
    expect(page.locator(".widget")).to_have_css("width", "64px")
    exercise_fullscreen(playwright, pid)
    # Send the folder watcher's event with a real PNG and exercise the native encoder.
    page.evaluate("""() => {
        const settings = JSON.parse(localStorage.getItem('clippo.preferences'));
        localStorage.setItem('clippo.preferences', JSON.stringify({...settings, watch:true}));
    }""")
    page.reload()
    expect(handle).to_be_visible()
    capture = folder / "Capture.png"
    shutil.copyfile(Path(__file__).resolve().parents[1] / "src-tauri/icons/icon.png", capture)
    page.evaluate("""path => window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {
        event:'capture', payload:{path, wasHidden:false}
    })""", str(capture))
    prompt = page.get_by_role("button", name="Compress screenshot", exact=True)
    expect(prompt).to_be_visible()
    expect(page.locator(".widget")).to_have_css("width", "100px")
    prompt.click()
    expect(page.locator(".handle-message")).to_contain_text("%")
    expect(page.locator(".handle-message")).to_contain_text("saved")
    expect(page.locator(".widget")).to_have_css("height", "24px")
    expect(page.locator(".widget")).to_have_css("width", "64px", timeout=5000)
    # The hosted runner has its own clipboard. Keep local runs off the user's clipboard.
    if os.environ.get("CLIPPO_TEST_CLIPBOARD") == "1":
        page.evaluate("""() => {
            const settings = JSON.parse(localStorage.getItem('clippo.preferences'));
            localStorage.setItem('clippo.preferences', JSON.stringify({...settings, auto:true}));
        }""")
        page.reload()
        expect(handle).to_be_visible()
        subprocess.run(["powershell", "-NoProfile", "-STA", "-Command",
                        "Add-Type -AssemblyName System.Windows.Forms,System.Drawing\n"
                        "$image = [Drawing.Bitmap]::new($env:CLIPPO_TEST_PNG)\n"
                        "[Windows.Forms.Clipboard]::SetImage($image)\n$image.Dispose()"],
                       env={**os.environ, "CLIPPO_TEST_PNG": str(capture)}, check=True)
        expect(page.locator(".handle-message")).to_contain_text("%", timeout=10000)
        expect(page.locator(".widget")).to_have_css("width", "100px")
        expect(page.locator(".widget")).to_have_css("width", "64px", timeout=5000)
        # Some capture tools supply only the registered PNG format, without a DIB.
        # Use a second image so deduplication and real arrival in the fan are observable.
        png = Path(__file__).resolve().parents[1] / "public/clippo.png"
        subprocess.run(["powershell", "-NoProfile", "-STA", "-Command",
                        "$ErrorActionPreference = 'Stop'\n"
                        "Add-Type -AssemblyName System.Windows.Forms\n"
                        "$data = [Windows.Forms.DataObject]::new()\n"
                        "$png = [IO.MemoryStream]::new([IO.File]::ReadAllBytes($env:CLIPPO_TEST_PNG))\n"
                        "$data.SetData('PNG', $false, $png)\n"
                        "[Windows.Forms.Clipboard]::SetDataObject($data, $true)"],
                       env={**os.environ, "CLIPPO_TEST_PNG": str(png)}, check=True)
        expect(page.locator(".handle-message")).to_contain_text("%", timeout=20000)
        expect(page.locator(".widget")).to_have_css("width", "64px", timeout=5000)
        handle.click()
        expect(page.locator(".fan-card img")).to_have_count(2)
        expect(page.locator(".widget")).to_have_css("width", "420px")
        exercise_fan_click(page)
        page.get_by_role("button", name="Preferences", exact=True).click()
        page.get_by_role("button", name="Appearance", exact=True).click()
        fan = page.get_by_role("switch", name="Screenshot fan in the teardrop")
        expect(fan).to_be_checked()
        fan.click()
        page.get_by_role("button", name="Back", exact=True).click()
        expect(page.locator(".fan-card")).to_have_count(0)
        page.get_by_role("button", name="Collapse to teardrop").click()
        expect(handle).to_be_visible()
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
    resize = page.locator('[data-drop-action="resize"]')
    expect(page.locator(".widget")).to_have_css("width", "420px")
    target = resize.element_handle()
    target.wait_for_element_state("stable")
    # Explorer sends drag-over events continuously while the window expands.
    active = page.locator('[data-drop-action="resize"].active')
    for _ in range(60):
        box = target.bounding_box()
        x, y = box["x"] + box["width"] / 2, box["y"] + box["height"] / 2
        drag("dragOver", image, x, y)
        if active.is_visible():
            break
        time.sleep(.05)
    expect(active).to_be_visible()
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
                    exercise(browser.contexts[0].pages[0], folder, playwright, app.pid)
            finally:
                app.terminate()
                app.wait(timeout=10)


if __name__ == "__main__":
    main()
