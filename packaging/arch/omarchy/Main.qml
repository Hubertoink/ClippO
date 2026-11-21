import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Ui as Ui
import qs.Commons
import qs.Commons as Commons

Ui.Panel {
    id: root
    moduleName: "hubertoink.clippo"
    ipcTarget: "hubertoink.clippo"
    manageIpc: false
    implicitWidth: barButton.implicitWidth
    implicitHeight: barButton.implicitHeight

    Commons.Translations { id: strings; directory: Qt.resolvedUrl("i18n") }

    readonly property color ink: Commons.Color.popups.text
    readonly property color muted: Commons.Color.muted
    property alias mode: persisted.mode
    property alias outputFormat: persisted.outputFormat
    property alias underMb: persisted.underMb
    property alias preserveMetadata: persisted.preserveMetadata
    property alias saveFolder: persisted.saveFolder
    property bool settingsOpen: false
    property string message: strings.t("panel.ready")
    property string lastPath: ""
    property string lastName: ""
    property int originalSize: 0
    property int resultSize: 0
    property bool failed: false
    property bool busy: compression.running
    readonly property int savings: originalSize > resultSize
        ? Math.round((1 - resultSize / originalSize) * 100) : 0

    PersistentProperties {
        id: persisted
        reloadableId: "hubertoink-clippo"
        property string mode: "Balanced"
        property string outputFormat: "Original"
        property bool underMb: false
        property bool preserveMetadata: false
        property string saveFolder: ""
    }

    function filePath(url) {
        let value = url.toString()
        return decodeURIComponent(value.replace(/^file:\/\//, ""))
    }

    function startFiles(urls) {
        if (busy || !urls || urls.length === 0) return
        let paths = []
        for (let url of urls) paths.push(url.toString ? filePath(url) : String(url))
        failed = false
        message = strings.t("panel.working")
        lastPath = ""
        let command = ["/usr/bin/clippo", "compress", "--mode", mode,
            "--output", outputFormat]
        if (underMb) command.push("--under-mb")
        if (preserveMetadata) command.push("--preserve")
        if (saveFolder) command.push("--folder", saveFolder)
        compression.command = command.concat(paths)
        compression.running = true
    }

    function parse(line) {
        let event
        try { event = JSON.parse(line) } catch (error) { return }
        if (event.kind === "result") {
            lastPath = String(event.path || "")
            lastName = String(event.name || "")
            originalSize = Number(event.original || 0)
            resultSize = Number(event.size || 0)
            message = ""
            failed = false
        } else if (event.kind === "error") {
            failed = true
            message = String(event.message || strings.t("panel.error"))
        }
    }

    function pretty(bytes) {
        if (!bytes) return "0 B"
        let units = ["B", "KB", "MB", "GB"]
        let unit = Math.min(3, Math.floor(Math.log(bytes) / Math.log(1000)))
        return (bytes / Math.pow(1000, unit)).toFixed(unit > 1 ? 1 : 0) + " " + units[unit]
    }

    Process {
        id: compression
        stdout: SplitParser { onRead: line => root.parse(line) }
        stderr: SplitParser { onRead: line => root.parse(line) }
        onExited: function(code, status) {
            if (code !== 0 && !root.failed) {
                root.failed = true
                root.message = strings.t("panel.error")
            }
        }
    }

    FileDialog {
        id: files
        fileMode: FileDialog.OpenFiles
        nameFilters: ["Images, PDF and WAV (*.png *.jpg *.jpeg *.webp *.avif *.heic *.pdf *.wav)"]
        onAccepted: root.startFiles(selectedFiles)
    }

    FolderDialog {
        id: folderDialog
        onAccepted: root.saveFolder = root.filePath(selectedFolder)
    }

    IpcHandler {
        target: "hubertoink.clippo"
        function open(): void { root.open() }
        function close(): void { root.close() }
        function toggle(): void { root.toggle() }
        function settings(): void { root.settingsOpen = true; root.open() }
    }

    Ui.BarIconButton {
        id: barButton
        anchors.fill: parent
        bar: root.bar
        text: "󰆏"
        dimmed: root.busy
        tooltipText: strings.t("panel.title")
        onPressed: root.toggle()
    }

    DropArea {
        anchors.fill: parent
        keys: ["text/uri-list"]
        onEntered: event => {
            event.accepted = event.hasUrls
            if (event.hasUrls) root.open()
        }
        onDropped: event => {
            root.startFiles(event.urls)
            event.accepted = true
        }
    }

    Ui.KeyboardPanel {
        anchorItem: barButton
        owner: root
        bar: root.bar
        open: root.opened
        focusTarget: keyArea
        contentWidth: fittedContentWidth(Style.space(420))
        contentHeight: cappedContentHeight(Style.space(460))

        FocusScope {
            id: keyArea
            anchors.fill: parent
            focus: true
            Keys.onPressed: function(event) {
                if (event.key === Qt.Key_Escape) {
                    if (root.settingsOpen) root.settingsOpen = false
                    else root.close()
                    event.accepted = true
                } else if ((event.modifiers & Qt.ControlModifier) && event.key === Qt.Key_O) {
                    files.open()
                    event.accepted = true
                }
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: Style.space(12)

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Style.space(10)
                    Label {
                        text: "󰆏"
                        font.pixelSize: Style.font.display
                        color: root.ink
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Style.space(1)
                        Label { text: strings.t("panel.title"); font.pixelSize: Style.font.title; font.bold: true }
                        Label {
                            text: strings.t("panel.subtitle")
                            color: root.muted
                            font.pixelSize: Style.font.caption
                        }
                    }
                    Ui.PanelActionButton {
                        iconText: "󰒓"
                        tooltipText: strings.t("settings.title")
                        foreground: root.ink
                        onClicked: root.settingsOpen = !root.settingsOpen
                    }
                }

                Ui.PanelSeparator { Layout.fillWidth: true; foreground: root.ink }

                Label {
                    visible: !root.settingsOpen
                    Layout.fillWidth: true
                    text: strings.t("panel.mode")
                    color: root.muted
                    font.pixelSize: Style.font.caption
                    font.bold: true
                }
                RowLayout {
                    visible: !root.settingsOpen
                    Layout.fillWidth: true
                    spacing: Style.space(6)
                    Repeater {
                        model: ["Smart", "Balanced", "Maximum"]
                        delegate: Ui.Button {
                            required property string modelData
                            Layout.fillWidth: true
                            text: modelData
                            selected: root.mode === modelData
                            bordered: true
                            focusable: true
                            foreground: root.ink
                            fontFamily: Style.font.family
                            fontSize: Style.font.bodySmall
                            enabled: !root.busy
                            onClicked: root.mode = modelData
                        }
                    }
                }

                Label {
                    visible: !root.settingsOpen
                    Layout.fillWidth: true
                    text: strings.t("panel.format")
                    color: root.muted
                    font.pixelSize: Style.font.caption
                    font.bold: true
                }
                RowLayout {
                    visible: !root.settingsOpen
                    Layout.fillWidth: true
                    spacing: Style.space(6)
                    Repeater {
                        model: ["Original", "WebP", "JPEG", "PNG"]
                        delegate: Ui.Button {
                            required property string modelData
                            Layout.fillWidth: true
                            text: modelData
                            selected: root.outputFormat === modelData
                            bordered: true
                            focusable: true
                            foreground: root.ink
                            fontFamily: Style.font.family
                            fontSize: Style.font.bodySmall
                            enabled: !root.busy
                            onClicked: root.outputFormat = modelData
                        }
                    }
                }

                Ui.CursorSurface {
                    visible: !root.settingsOpen
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    bordered: true
                    foreground: root.ink
                    current: dropArea.containsDrag

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Style.space(20)
                        spacing: Style.space(9)
                        visible: !root.lastPath && !root.busy

                        Item { Layout.fillHeight: true }
                        Label {
                            visible: !root.settingsOpen
                            Layout.fillWidth: true
                            text: "󰈔"
                            horizontalAlignment: Text.AlignHCenter
                            font.pixelSize: Style.space(34)
                            color: root.muted
                        }
                        Label {
                            Layout.fillWidth: true
                            text: strings.t("panel.drop")
                            horizontalAlignment: Text.AlignHCenter
                            font.pixelSize: Style.font.heading
                            font.bold: true
                        }
                        Label {
                            Layout.fillWidth: true
                            text: strings.t("panel.hint")
                            horizontalAlignment: Text.AlignHCenter
                            color: root.muted
                            font.pixelSize: Style.font.caption
                        }
                        Ui.Button {
                            Layout.alignment: Qt.AlignHCenter
                            text: strings.t("panel.choose")
                            bordered: true
                            focusable: true
                            foreground: root.ink
                            onClicked: files.open()
                        }
                        Item { Layout.fillHeight: true }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Style.space(20)
                        spacing: Style.space(9)
                        visible: root.busy
                        Item { Layout.fillHeight: true }
                        Label {
                            Layout.fillWidth: true
                            text: "󰑐"
                            horizontalAlignment: Text.AlignHCenter
                            color: root.ink
                            font.pixelSize: Style.space(34)
                        }
                        Label {
                            Layout.fillWidth: true
                            text: strings.t("panel.working")
                            horizontalAlignment: Text.AlignHCenter
                            font.pixelSize: Style.font.heading
                        }
                        Item { Layout.fillHeight: true }
                    }

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Style.space(18)
                        spacing: Style.space(8)
                        visible: !!root.lastPath
                        Item { Layout.fillHeight: true }
                        Label {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: root.lastName
                            elide: Text.ElideMiddle
                            font.bold: true
                        }
                        Label {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            text: root.savings + "% " + strings.t("panel.done")
                            font.pixelSize: Style.font.heading
                            color: Commons.Color.accent
                        }
                        Label {
                            Layout.fillWidth: true
                            horizontalAlignment: Text.AlignHCenter
                            color: root.muted
                            text: root.pretty(root.originalSize) + " → " + root.pretty(root.resultSize)
                        }
                        RowLayout {
                            Layout.alignment: Qt.AlignHCenter
                            Ui.Button {
                                text: strings.t("panel.open")
                                bordered: true
                                focusable: true
                                foreground: root.ink
                                onClicked: {
                                    let slash = root.lastPath.lastIndexOf("/")
                                    Quickshell.execDetached(["xdg-open", root.lastPath.substring(0, slash)])
                                }
                            }
                            Ui.Button {
                                text: "+"
                                bordered: true
                                focusable: true
                                foreground: root.ink
                                onClicked: {
                                    root.lastPath = ""
                                    files.open()
                                }
                            }
                        }
                        Item { Layout.fillHeight: true }
                    }

                    DropArea {
                        id: dropArea
                        anchors.fill: parent
                        keys: ["text/uri-list"]
                        onEntered: event => event.accepted = event.hasUrls
                        onDropped: event => {
                            root.startFiles(event.urls)
                            event.accepted = true
                        }
                    }
                }

                Label {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Style.space(30)
                    text: root.message || (root.lastPath ? root.lastPath : strings.t("panel.ready"))
                    color: root.failed ? Commons.Color.urgent : root.muted
                    font.pixelSize: Style.font.caption
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideMiddle
                }

                ColumnLayout {
                    visible: root.settingsOpen
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: Style.space(10)

                    Label {
                        Layout.fillWidth: true
                        text: strings.t("settings.title")
                        font.pixelSize: Style.font.heading
                        font.bold: true
                    }

                    Ui.Toggle {
                        Layout.fillWidth: true
                        label: strings.t("settings.underMb")
                        description: strings.t("settings.underMbHint")
                        checked: root.underMb
                        foreground: root.ink
                        onClicked: root.underMb = !root.underMb
                    }

                    Ui.Toggle {
                        Layout.fillWidth: true
                        label: strings.t("settings.metadata")
                        description: strings.t("settings.metadataHint")
                        checked: root.preserveMetadata
                        foreground: root.ink
                        onClicked: root.preserveMetadata = !root.preserveMetadata
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: Style.space(5)
                        Label {
                            Layout.fillWidth: true
                            text: strings.t("settings.destination")
                            color: root.muted
                            font.pixelSize: Style.font.caption
                            font.bold: true
                        }
                        RowLayout {
                            Layout.fillWidth: true
                            Label {
                                Layout.fillWidth: true
                                text: root.saveFolder || strings.t("settings.besideSource")
                                elide: Text.ElideMiddle
                                color: root.muted
                            }
                            Ui.Button {
                                text: strings.t("settings.choose")
                                bordered: true
                                focusable: true
                                foreground: root.ink
                                onClicked: folderDialog.open()
                            }
                            Ui.PanelActionButton {
                                visible: !!root.saveFolder
                                iconText: "󰆴"
                                tooltipText: strings.t("settings.clear")
                                foreground: root.ink
                                onClicked: root.saveFolder = ""
                            }
                        }
                    }

                    Item { Layout.fillHeight: true }
                    Label {
                        Layout.fillWidth: true
                        text: strings.t("settings.note")
                        color: root.muted
                        font.pixelSize: Style.font.caption
                        wrapMode: Text.WordWrap
                    }
                }
            }
        }
    }

    component Label: Text {
        textFormat: Text.PlainText
        color: root.ink
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
    }
}
