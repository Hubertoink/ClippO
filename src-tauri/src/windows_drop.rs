use serde::{Deserialize, Serialize};
use tauri::{Emitter, WebviewWindow};
use webview2_com::{
    take_pwstr,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Controller4, ICoreWebView2File, ICoreWebView2WebMessageReceivedEventArgs2,
    },
    WebMessageReceivedEventHandler,
};
use windows_core::{Interface, PWSTR};

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Compress,
    Resize,
    Convert,
}

#[derive(Deserialize)]
struct Message {
    clippo_drop: Action,
}

#[derive(Clone, Serialize)]
struct Drop {
    action: Action,
    paths: Vec<String>,
}

pub fn install(window: &WebviewWindow) -> tauri::Result<()> {
    let widget = window.clone();
    window.with_webview(move |view| {
        // WebView2 supplies file objects for an actual Explorer drop. Their Path
        // property keeps the original location available for Save copy and Replace.
        let result: windows_core::Result<()> = (|| unsafe {
            let controller = view.controller();
            // Wry handles OLE drops on the child windows registered at startup.
            // Let WebView2 handle drops that land on a different child surface.
            controller
                .cast::<ICoreWebView2Controller4>()?
                .SetAllowExternalDrop(true)?;
            let webview = controller.CoreWebView2()?;
            let handler = WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                let Some(args) = args else { return Ok(()) };
                let Ok(extra) = args
                    .cast::<ICoreWebView2WebMessageReceivedEventArgs2>()
                    .and_then(|a| a.AdditionalObjects())
                else {
                    return Ok(());
                };
                let mut count = 0;
                extra.Count(&mut count)?;
                if count == 0 {
                    return Ok(());
                }
                let mut json = PWSTR::null();
                args.TryGetWebMessageAsString(&mut json)?;
                let Ok(message) = serde_json::from_str::<Message>(&take_pwstr(json)) else {
                    return Ok(());
                };
                let mut source = PWSTR::null();
                args.Source(&mut source)?;
                let Ok(origin) = tauri::Url::parse(&take_pwstr(source)) else {
                    return Ok(());
                };
                let origin = origin.origin().ascii_serialization();
                if !matches!(
                    origin.as_str(),
                    "http://tauri.localhost" | "https://tauri.localhost"
                ) && !(cfg!(debug_assertions) && origin == "http://127.0.0.1:1420")
                {
                    return Ok(());
                }
                let paths: windows_core::Result<Vec<_>> = (0..count)
                    .map(|index| {
                        let file = extra.GetValueAtIndex(index)?.cast::<ICoreWebView2File>()?;
                        let mut path = PWSTR::null();
                        file.Path(&mut path)?;
                        Ok(take_pwstr(path))
                    })
                    .collect();
                match paths {
                    Ok(paths) if paths.iter().all(|p| std::path::Path::new(p).is_absolute()) => {
                        let _ = widget.emit(
                            "windows-file-drop",
                            Drop {
                                action: message.clippo_drop,
                                paths,
                            },
                        );
                    }
                    _ => {
                        let _ = widget.emit("windows-file-drop-error", ());
                    }
                }
                Ok(())
            }));
            let mut token = 0;
            webview.add_WebMessageReceived(&handler, &mut token)
        })();
        if let Err(error) = result {
            eprintln!("Could not register the Explorer drop handler: {error}");
        }
    })
}
