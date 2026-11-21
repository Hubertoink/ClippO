# ClippO

[English](README.md) · Deutsch

Ein schwebendes Desktop-Widget für Bilder, Screenshots und PDFs. Mit **Strg + Umschalt + O** anzeigen oder minimieren, eine Datei aus dem Dateimanager ablegen oder mit **Strg + V** einfügen. ClippO zeigt die tatsächliche Dateigröße und Ersparnis, sobald die lokale Verarbeitung abgeschlossen ist.

## Starten

Für Omarchy Quattro auf x86-64 lade das Arch-Paket aus den [GitHub-Releases](https://github.com/Hubertoink/ClippO/releases/latest) herunter und installiere es im Downloadordner mit `sudo pacman -U ./clippo-0.9.1-1-x86_64.pkg.tar.zst`.

**Omarchy / Arch Linux (x86-64)** · Installiere `base-devel` und führe im ausgecheckten Repository `cd packaging/arch && makepkg -si` als normaler Benutzer aus. Führe danach einmal `clippo-omarchy-install` aus, um das native Omarchy-Leisten-Widget zu aktivieren. Es öffnet als verankertes Quickshell-Panel, übernimmt das aktive Omarchy-Design live und bleibt beim Workspace-Wechsel verfügbar. App-Launcher und `clippo panel` schalten dasselbe Panel um.

Die Sprache folgt beim ersten Start der Systemsprache. Unter **Einstellungen → Allgemein → Sprache** kannst du zwischen Deutsch und Englisch wechseln.

Für Ubuntu 24.04 auf x86-64 lade die Datei `ClippO_0.9.1_amd64.deb` aus den [GitHub-Releases](https://github.com/Hubertoink/ClippO/releases/latest) herunter. Führe im Downloadordner `sudo apt install ./ClippO_0.9.1_amd64.deb` aus und starte ClippO über das Anwendungsmenü. Das Paket nutzt ImageMagick, qpdf und Ghostscript aus den Ubuntu-Paketquellen. **Strg + V** verarbeitet ein kopiertes Bild oder eine Datei. Die Ordnererkennung überwacht den Unterordner `Screenshots` im Bilderverzeichnis. Die automatische Zwischenablageerkennung und die Outlook-/Teams-Aktionen stehen unter Windows bereit. Tastenkürzel und Fensterpositionierung hängen unter Linux von der Desktop-Sitzung ab.

Den Windows-x64-Installer findest du unter [GitHub Releases](https://github.com/Hubertoink/ClippO/releases/latest). Ab Version 0.3.2 enthält das Setup kein WebView2-Installationspaket und installiert die Laufzeit nicht nach. Die Microsoft WebView2 Runtime muss bereits auf dem Rechner vorhanden sein; das gilt auch für die portable ZIP. Ein neuer Release-Build schreibt den Installer nach `src-tauri/target/release/bundle/nsis/`. Das Widget erscheint am Bildschirmrand. Schließen oder die Minus-Schaltfläche blendet es aus. Über das Tray-Symbol lässt es sich wieder öffnen oder vollständig beenden.

Für die Entwicklung werden Node.js 22.12 oder neuer, Rust mit MSVC-Toolchain, Visual Studio C++ Build Tools, das Windows SDK und WebView2 benötigt. Das Startskript lädt die festgelegten Codec-Versionen beim ersten Build herunter und prüft ihre SHA-256-Prüfsummen.

```powershell
npm ci
npm run desktop
```

| Befehl | Ergebnis |
| --- | --- |
| `npm run desktop:build` | Release-App und NSIS-Installer für den aktuellen Benutzer |
| `npm run dev` | Lokale Browservorschau auf Port 1420 |
| `npm run build` | TypeScript-Prüfung und Produktionsbuild des Frontends |
| `powershell -ExecutionPolicy Bypass -File scripts/desktop.ps1 test` | Rust-Regressionstests für Dateiaustausch und wiederholtes Speichern |

## Bedienung

- **WAV in MP3 umwandeln.** Eine Mono- oder Stereo-WAV ablegen oder auswählen und anschließend **Kopie speichern** anklicken. Die Datei heißt `<Name>-clippo.mp3`. Ausgewogen verwendet 192 kbit/s, Smart 256 und Maximal 96. Unter **Einstellungen → Komprimierung** lässt sich eine eigene Bitrate wählen. Ein Größenlimit kann die Bitrate senken. Unterstützt werden PCM-WAV und 32-Bit-Float-WAV mit 8 bis 384 kHz. Das Original bleibt erhalten. WAV-spezifische Metadaten werden nicht übertragen.
- **Ablegen und Einfügen.** Der Drop-Bereich zeigt eine Schätzung. Während der Verarbeitung aktualisiert der native Encoder die Anzeige mit gemessenen Größen. Bei mehreren Dateien lässt sich das gewünschte Ergebnis über den Dateinamen auswählen.
- **Drop-Aktionen.** Beim Ziehen eines Bildes erscheinen drei Ziele. Komprimieren startet sofort. Größe ändern zeigt die ursprüngliche und die neue Breite × Höhe in Pixeln. Die Zielmaße folgen dem Prozentregler. Bei mehreren Bildern lässt sich die Vorschau pro Datei auswählen. Format ändern bietet JPEG, PNG und WebP an. Die Auswahl gilt für diesen Drop. Bei PDFs und gemischten Stapeln erscheint das Kompressionsziel.
- **Modi.** Ausgewogen ist die Voreinstellung. Smart priorisiert Bildtreue und optimiert PNGs verlustfrei. Maximal senkt die Qualität stärker. Eigene öffnet die Regler für JPEG, WebP, PNG und PDF.
- **Unter 1 MB bringen.** Das Ziel liegt bei höchstens 999.999 Byte. Nach jedem Durchlauf prüft ClippO die tatsächliche Größe. Bei Bedarf reduziert es Qualität und Bildabmessungen. Ein verfehltes Ziel wird im Ergebnis angezeigt.
- **Speichern.** Eine neue Kopie landet neben dem Original. Für eingefügte Bilder ist der Downloadordner voreingestellt. Ein konfigurierter Auto-Speicherort speichert jede Ausgabe sofort. Bereits vorhandene Dateinamen erhalten einen Zähler. Ein Klick auf „Gespeichert in …“ öffnet den Zielordner im Explorer und markiert die gespeicherte Kopie.
- **Ersetzen.** Die Aktion steht bei einer kleineren Ausgabe im selben Format zur Verfügung. Automatisches Ersetzen lässt sich ausdrücklich in den Einstellungen aktivieren.
- Unter Omarchy öffnet ein Klick auf das ClippO-Leisten-Icon das native Panel. Dateien kannst du direkt auf das Icon oder in das geöffnete Panel ziehen. Das Zahnrad öffnet Panel-Einstellungen für das 1-MB-Ziel, Metadaten und den Speicherort; sie bleiben in der Shell erhalten. Das Ergebnis landet kollisionssicher als `<name>-clippo.<ext>` neben der Quelle. Auf anderen Desktops kannst du unter **Einstellungen → Darstellung → Tropfen am oberen Bildschirmrand** den bisherigen oberen Tropfen aktivieren.
- **Schnellaktionen.** Eine Datei für Explorer oder E-Mail kopieren, ein Bild als Pixel in die Zwischenablage legen oder den Anhang im Explorer auswählen. Klassisches Outlook öffnet einen Entwurf mit Anhang. Beim neuen Outlook und bei Teams öffnet ClippO die Anwendung und legt die Datei zum Einfügen mit Strg + V bereit. Der Versand erfolgt in der Zielanwendung.
- **Screenshot-Erkennung.** Änderungen an der Windows-Zwischenablage und neue Bilder im registrierten Screenshot-Ordner lösen einen kurzen Hinweis aus. Das Widget erscheint ohne Fokuswechsel. Automatische Komprimierung für kopierte Bilder ist separat aktivierbar.
- **Darstellung.** Das normale Widget misst 420 × 260 logische Pixel. Einstellungen erhalten vorübergehend mehr Platz. Position, Sprache, Theme, Kompaktmodus und Animationseinstellungen bleiben beim Neustart erhalten.

## Architektur

```text
Explorer / Dateiauswahl ──┐
Windows-Zwischenablage ──┼── React-Widget ── Tauri IPC ── Rust-Worker
Screenshot-Ordner ───────┘         ▲                        │
                                  │                   private Arbeitskopie
                                  │                        │
                                  │                 Bild- oder PDF-Encoder
                                  │                        │
                                  └──── Ergebnis-ID, Größen und Fortschritt
                                                 │
                                      Kopieren / Speichern / Ersetzen
```

| Bereich | Implementierung | Verantwortung |
| --- | --- | --- |
| Oberfläche | `src/App.tsx`, `src/components` | React und TypeScript, Tailwind, angepasste shadcn/ui-Bedienelemente mit Radix Switch, Motion, deutsche und englische Texte. Die Eingaben werden in einer Warteschlange verarbeitet. |
| Desktop-Lebenszyklus | `src-tauri/src/main.rs`, `desktop.rs` | Eine Tauri-Instanz, Acrylic-Fenster, nativer Tray, globales Tastenkürzel, Position innerhalb des Monitor-Arbeitsbereichs. |
| Erkennung | `desktop.rs` | Windows-Sequenznummer der Zwischenablage, Erkennung verfügbarer Bildformate und `notify` für den über die Known-Folder-API ermittelten Screenshot-Ordner. Datei-Ereignisse werden bis zu einer stabilen Größe zusammengefasst. |
| Dateiaufträge | `jobs.rs` | Arbeitskopie, Encoder auf einem Hintergrundthread, tatsächliche Bytezahlen, Ergebnis-IDs, Speicherziele und Windows-Aktionen. Der Renderer erhält für native Dateien keine vollständigen Bilddaten. |
| Bilder | `engine.rs` | image-rs für Dekodierung und Ausrichtung, mozjpeg, libwebp und oxipng. Das mitgelieferte ImageMagick übernimmt Palettenoptimierung, Metadatenverarbeitung, HEIC-Eingabe und AVIF. |
| PDFs | `engine.rs` | lopdf verarbeitet die Dokumentstruktur. qpdf entfernt ungenutzte Ressourcen, optimiert Streams und linearisiert die Ausgabe. Ein lokal vorhandenes Ghostscript übernimmt bei einfachen PDFs zusätzlich DPI-basiertes Downsampling und Font-Subsetting. |
| Lokale Browservorschau | `src/lib/browser.ts` | Canvas, UPNG, HEIC-Dekodierung und pdf-lib. Speichern erfolgt als Download. Windows-Tray, systemweite Erkennung, Originalersetzung und die vollständige PDF-Pipeline gehören zur Desktop-App. |

### Daten und Dateiintegrität

Eine Größenangabe vor dem Drop ist eine formatunabhängige Näherung anhand des Modus. Bei einem Größenlimit begrenzt das Ziel die Vorschau. Das Ergebnis verwendet die gemessene Ausgabedatei.

Bei gleichem Format behält ClippO die kleinere der beiden Versionen. Das kann bei bereits optimierten Dateien 0 % Ersparnis ergeben. Eine ausdrücklich gewählte Formatkonvertierung kann eine größere Datei erzeugen. HEIC-Eingaben werden standardmäßig als WebP ausgegeben. Das Original bleibt bei einer Formatänderung erhalten.

Vor dem Ersetzen vergleicht ClippO den aktuellen Dateiinhalt mit dem eingelesenen Stand. Während des Vergleichs und Austauschs hält es einen Handle, der parallele Schreibzugriffe ausschließt. Es schreibt die Ausgabe zuerst auf demselben Laufwerk und ruft anschließend `ReplaceFileW` auf. Bei einem von Windows gemeldeten unvollständigen Austausch bleibt eine vorhandene Wiederherstellungskopie am angezeigten Pfad erhalten.

Die Oberfläche speichert ihre Einstellungen im lokalen WebView2-Profil. Fertige Dateien liegen im App-Cache, damit Explorer, Outlook und Teams sie nach dem Kopieren noch lesen können. Für eingefügte Bilder wird auch der Eingabestand aufbewahrt, damit das Größenlimit nachträglich auf denselben Screenshot angewendet werden kann. Beim Start werden Ergebnisordner entfernt, die älter als 24 Stunden sind. Die eigentlichen Komprimierungsaufrufe benötigen keine Netzwerkverbindung.

### Automatische Updates unter Windows

ClippO prüft bei jedem App-Start im Hintergrund, ob auf GitHub eine neuere stabile Version vorliegt. Das Widget bleibt auch offline bedienbar. Über **Update laden** lädt ClippO den Installer und prüft seine Signatur. **Installieren** startet die Aktualisierung, sobald die laufende Komprimierung beendet ist. Mit **Später** schließt du den Hinweis für diese Sitzung. In der portablen Ausgabe führt **Zur ZIP** zur Release-Seite. Entpacke die neue ZIP nach dem Beenden der App. Wer eine Version vor 0.4.0 verwendet, installiert das erste Update einmal manuell. [Windows-Releases](https://github.com/Hubertoink/ClippO/releases/latest)

Für ein signiertes x64-Release braucht der Build den privaten Schlüssel zum öffentlichen Updater-Schlüssel in `src-tauri/tauri.conf.json`. Sichere den privaten Schlüssel getrennt vom Repository. Er wird auch für spätere Releases benötigt.

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Join-Path $env:LOCALAPPDATA 'ClippO/signing/updater.key'
.\scripts\desktop.ps1 build
.\scripts\package-release.ps1 -Signed
```

Lade den Installer, seine `.sig`-Datei, die portable ZIP und `latest.json` aus `release/` gemeinsam in einen GitHub-Release-Entwurf hoch. Veröffentliche ihn unter dem Tag `v<Version>` als neuestes stabiles Release. Der manuelle GitHub-Actions-Build erzeugt dieselben Updater-Dateien, wenn der Signierschlüssel über das Actions-Secret `TAURI_SIGNING_PRIVATE_KEY` bereitsteht. [Tauri-Updater](https://v2.tauri.app/plugin/updater/)

### Grenzen

- Das Eingabelimit für Dateien beträgt 256 MiB. Der lokale Bilddecoder prüft zusätzlich die Bildabmessungen. Sehr große Bilder, komplexe PDFs und aggressive Zielgrößen können mehrere Durchläufe erfordern.
- Bei Formularen, Annotationen und bestimmten erweiterten PDF-Strukturen nutzt ClippO den strukturerhaltenden Pfad. Das Subsetting eingebetteter Fonts benötigt Ghostscript. Signierte oder gesperrte PDFs melden einen Fehler.
- Im Browser hängt die Clipboard-Freigabe vom Browser ab. Ctrl + V funktioniert auch über das normale Einfügeereignis. Bestehende Bildmetadaten können in der Windows-App erhalten werden.
- Zum globalen Anzeigen und Minimieren verwendet diese Version Strg + Umschalt + O. Bei einem Tastenkonflikt zeigt sie einen Hinweis auf das Tray-Symbol. MP4, GIF und MOV sind Erweiterungspunkte.

### Prüfung unter Windows

Im lokalen Release-Test verarbeitete die native App PNG, JPG/JPEG, WebP, AVIF, HEIC und PDF. Ein 1.920 × 1.080 Pixel großer Test-Screenshot schrumpfte von 64.930 auf 6.203 Byte. In zwei Messungen bei geöffnetem Widget vergingen vom ausgelösten Drop-Ereignis bis zur ersten DOM-Aktualisierung 4,6 und 6,2 ms. Das Ergebnis erschien nach 283 beziehungsweise 188 ms.

Ein PDF mit Bild und durchsuchbarem Text schrumpfte von 494.111 auf 63.417 Byte. Die Ausgabe ließ sich mit PDFium rendern, der Text blieb extrahierbar und qpdf bestätigte die Linearisierung. Weitere Prüfungen umfassten die Screenshot-Ordnererkennung ohne Fokuswechsel, das globale Tastenkürzel, die automatische Zwischenablageverarbeitung, das Datei-Kopierformat von Windows, wiederholtes Speichern und die Ablehnung einer zwischenzeitlich geänderten Originaldatei.

Das getestete CMYK-Poster enthält CMYK-JPEGs mit separaten Transparenzmasken und einem Druckprofil. Ab Version 0.2.0 komprimiert der strukturerhaltende PDF-Pfad auch diese JPEGs. Bei Qualitätsstufe 72 sank die Größe von 12.157.162 auf 6.595.994 Byte. Der Vergleich prüfte gerenderte Seiten, extrahierten Text und das eingebettete Farbprofil.

### Technische Referenzen

Das Widget verwendet die Fenster-Effekte, Tray-APIs und Shortcut-Verwaltung von Tauri. [Tauri-Fenster](https://v2.tauri.app/reference/javascript/api/namespacewindow/) · [Globale Tastenkürzel](https://v2.tauri.app/plugin/global-shortcut/) · [Tray](https://v2.tauri.app/reference/javascript/api/namespacetray/)

Die PDF-Ausgabe verwendet qpdfs Stream-Optimierung und Linearisierung. [qpdf-CLI](https://qpdf.readthedocs.io/en/stable/cli.html)

Der Windows-Dateiaustausch nutzt die Semantik von ReplaceFileW für Berechtigungen, Datenströme und Wiederherstellungskopien. [Microsoft-Dokumentation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)

<details>
<summary>Bildquellen und Generierungsangaben</summary>

Der Desktop-Hintergrund `public/alpine-dusk.webp` und das Ablage-Icon `public/drop-tray.webp` wurden mit dem integrierten ImageGen-Werkzeug erstellt und lokal als WebP eingebunden.

**Hintergrund**

> Use case: photorealistic-natural. Create a premium widescreen desktop wallpaper, 16:9 landscape. Expansive alpine mountain ranges at dusk, hazy layers of slate blue, lavender and deep midnight indigo. A sculptural jagged ridge slopes up along the left edge, and dark fir tree silhouettes anchor the lower right. Far mountain peaks etched gently by pale apricot light on the horizon, a vast soft lavender cloudless sky above. Center calm for a floating dark app widget. Quiet photographic film grain, elegant, matte, atmospheric, real alpine photography. A restrained harmonious dusk palette, no lettering, logos, borders or interface.

**Ablage-Icon**

> A single beautifully crafted small 3D icon for a premium file compression utility. Isolated on fully transparent background. A thick glossy translucent periwinkle-purple inbox tray, open top, rounded squared silhouette with distinctive lowered U notch on its front lip. Two miniature physical rounded files float just above it: one light lavender image tile with a simple indigo mountain-and-sun relief on its face gently tilted counterclockwise, behind it one pearl lilac document sheet tilted clockwise with three embossed lines and a folded corner. Materials like frosted glass and matte enamel. Front three-quarter view from just above, gentle perspective, clear front-facing icon silhouette, compact arrangement. Soft studio light from upper left, crisp rim glints, deep violet shadows, premium macOS app-icon rendering, friendly, softly rounded, sculptural. Entire object centered and fully visible, no text, no letters, no UI, no typography, no stand, no pedestal, no square app tile behind it. Composition wide as it is tall. Generous clear margin.

</details>


## Lizenz

Der ClippO-Quellcode steht unter der [MIT-Lizenz](LICENSE). Für die eingebundenen Bibliotheken, Schriftarten und Kompressionsprogramme gelten ihre jeweiligen Lizenzen.
