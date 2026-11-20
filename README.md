# ClippO

English · [Deutsch](README.de.md)

ClippO is a floating desktop widget for compressing screenshots, images and PDFs on Windows and Linux. Drop a file onto the widget or paste a screenshot with **Ctrl + V**. Drop a WAV recording to convert it to MP3. ClippO shows the original size, output size and savings. Files are processed locally.

## Install

Choose a package from [GitHub Releases](https://github.com/Hubertoink/ClippO/releases/latest).

| Platform | Download | How to start |
| --- | --- | --- |
| Windows x64 | `*-setup.exe` | Run the installer for your Windows account. |
| Windows x64, portable | `*-portable.zip` | Extract the whole archive and run `clippo.exe` next to `resources`. |
| Ubuntu 24.04 LTS, x86-64 | `*_amd64.deb` | In the download folder, run `sudo apt install ./ClippO_0.8.1_amd64.deb`. Open **ClippO** from the application menu. |

Both Windows editions require the Microsoft Edge WebView2 Runtime. On Ubuntu, APT installs the runtime dependencies, including WebKitGTK, ImageMagick, qpdf and Ghostscript.

ClippO starts in German on German-language systems and in English elsewhere. Choose **Preferences → General → Language → English** to switch immediately. In German, choose **Einstellungen → Allgemein → Sprache → English**. Your selection is saved for the next launch.

## Use the widget

- Press **Ctrl + Shift + O** to show or hide ClippO. **Ctrl + O** opens a file picker. The tray menu can reopen or quit the app.
- Enable **Preferences → Appearance → Top-edge teardrop** to dock ClippO at the top center of the current screen. Drag a file onto the teardrop to unfold its actions, or click it to open the widget. The minus button and **Ctrl + Shift + O** collapse it again. On Windows and X11, the resting teardrop disappears during fullscreen on its monitor and returns when you leave fullscreen.
- Drop a PNG, JPEG, WebP, AVIF, HEIC or PDF to compress it. When dropping images, you can also resize or convert them. The resize preview shows the original and target width and height for each image.
- Drop or browse for a mono or stereo WAV file to convert it locally to MP3. Choose **Save copy** to save `<name>-clippo.mp3` beside the WAV. Balanced uses a target bitrate of 192 kbit/s, Smart 256 and Maximum 96. Set a custom bitrate under **Preferences → Compression**. A file-size limit can lower the bitrate. The decoder accepts PCM WAV and 32-bit IEEE float WAV at 8–384 kHz. WAV-specific metadata is not transferred.
- Choose **Balanced**, **Smart**, **Maximum** or **Custom**. Enable **Under 1 MB** to target 999,999 bytes. ClippO reports the actual output size when it finishes.
- Save a copy next to the original, set an auto-save folder or copy the result to the clipboard. Click **Saved to** to open the file manager. You can replace the original when the output is smaller and has the same format.
- On Windows, ClippO can detect clipboard screenshots, prepare an Outlook draft and copy a file for Teams. On Linux, paste a clipboard image or a copied file with **Ctrl + V**. Folder detection watches the **Screenshots** subfolder in your Pictures directory.
- Linux window positioning, global shortcuts, clipboard access and tray visibility depend on the desktop session. If the shortcut is unavailable, launch ClippO again from the application menu to show the existing window.

## Updates

ClippO checks for a newer stable release on startup. Choose **Download** to fetch and verify the signed update, then **Install** when your compression job is finished. On Ubuntu, approve the package installation and restart ClippO when prompted. **Later** dismisses the notice until the next launch. Portable Windows builds link to the release ZIP. Windows versions before 0.4.0 need one manual upgrade.

## Build from source

ClippO uses Rust, Tauri 2, React and TypeScript. Frontend development requires Node.js 22.12 or newer. Run `npm ci` first.

| Command | Purpose |
| --- | --- |
| `npm run dev` | Start the browser preview. |
| `npm run build` | Type-check and build the frontend. |
| `npm run desktop` / `npm run desktop:build` | Run or package the Windows app with MSVC, Visual Studio C++ Build Tools and the Windows SDK. The script downloads checksum-verified Windows codecs. |
| `npm run tauri -- dev` / `npm run tauri -- build --bundles deb` | Run or package the native Ubuntu app. Install the build dependencies listed in the [Linux workflow](.github/workflows/linux-package.yml). |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml` | Test resizing, original-file replacement and conflict-free copy names. On Windows, `scripts/desktop.ps1 test` also configures the build environment. |

The Linux workflow builds the Debian package, installs it with APT and starts the packaged widget on a virtual X11 display. It pastes PNG, PDF and WAV files and checks their outputs. The codec regression test independently decodes the generated MP3 audio.

### Publish updates

Build the Windows installer with `TAURI_SIGNING_PRIVATE_KEY` pointing to the private key matching the public key in `src-tauri/tauri.conf.json`. Keep a secure backup of that key. Download the tested Linux package into `release/` and sign it with the same key. From PowerShell in the project directory:

```powershell
$version = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
$keyPath = Join-Path $env:LOCALAPPDATA 'ClippO/signing/updater.key'
$env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
.\scripts\desktop.ps1 build
$env:TAURI_SIGNING_PRIVATE_KEY = $null
$deb = "release/ClippO_${version}_amd64.deb"
npm exec -- tauri signer sign -f $keyPath --app-version $version $deb
.\scripts\package-release.ps1 -Signed -LinuxPackage $deb
```

Publish the installer, portable ZIP, Debian package, both detached signatures and the combined `latest.json` in one release tagged `v<version>`. Mark the release as latest once all six assets are present. [Tauri updater](https://v2.tauri.app/plugin/updater/)

## Implementation

The React widget sends file paths to background Rust workers. Each job works on a private copy. Image processing uses image-rs, mozjpeg, oxipng, libwebp and ImageMagick. Audio conversion streams WAV samples through the bundled [LAME MP3 encoder](https://lame.sourceforge.io/). The PDF pipeline uses lopdf and qpdf, with Ghostscript for downsampling suitable documents. Signed and encrypted PDFs are rejected. Already optimized files may show 0% savings. The input limit is 256 MiB per file.

New copies receive a unique name. Before replacing an original, ClippO checks its contents against the input used for compression. Temporary results stay in the app cache for clipboard and attachment actions. Startup cleanup removes result directories older than 24 hours.

## License

ClippO is released under the [MIT License](LICENSE). Its libraries, fonts and compression tools retain their respective licenses.
