use gtk::prelude::WidgetExt;
use tauri::WebviewWindow;
use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window},
    rust_connection::RustConnection,
};

pub struct Probe {
    connection: RustConnection,
    root: Window,
    state: Atom,
    fullscreen: Atom,
}

impl Probe {
    pub fn new() -> Option<Self> {
        // A Wayland session's Xwayland root cannot describe native Wayland windows.
        if std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
            || (std::env::var_os("WAYLAND_DISPLAY").is_some()
                && std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("x11"))
        {
            return None;
        }
        let (connection, screen) = x11rb::connect(None).ok()?;
        let root = connection.setup().roots[screen].root;
        let atom = |name: &[u8]| Some(connection.intern_atom(false, name).ok()?.reply().ok()?.atom);
        let state = atom(b"_NET_WM_STATE")?;
        let fullscreen = atom(b"_NET_WM_STATE_FULLSCREEN")?;
        Some(Self {
            connection,
            root,
            state,
            fullscreen,
        })
    }

    pub fn fullscreen_on_monitor(&self, widget: &WebviewWindow) -> bool {
        let Ok(Some(monitor)) = widget.current_monitor() else {
            return false;
        };
        let origin = monitor.position();
        let size = monitor.size();
        let Ok(x) = i16::try_from(origin.x + size.width as i32 / 2) else {
            return false;
        };
        let Ok(y) = i16::try_from(origin.y + size.height as i32 / 2) else {
            return false;
        };
        self.fullscreen_at(x, y)
    }

    fn fullscreen_at(&self, x: i16, y: i16) -> bool {
        let conn = &self.connection;
        let mut parent = self.root;
        // A reparenting WM puts a frame between the X root and the browser.
        // Follow only the visible branch under the center of the dock's monitor.
        while let Some(hit) = conn
            .translate_coordinates(self.root, parent, x, y)
            .ok()
            .and_then(|r| r.reply().ok())
        {
            let child = hit.child;
            if child == 0 {
                break;
            }
            let fullscreen = conn
                .get_property(false, child, self.state, AtomEnum::ATOM, 0, 64)
                .ok()
                .and_then(|r| r.reply().ok())
                .is_some_and(|r| {
                    r.value32()
                        .is_some_and(|mut atoms| atoms.any(|a| a == self.fullscreen))
                });
            if fullscreen {
                return true;
            }
            parent = child;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::{
        protocol::xproto::{CreateWindowAux, PropMode, WindowClass},
        wrapper::ConnectionExt as _,
    };

    #[test]
    #[ignore = "requires a dedicated X11 display"]
    fn detects_fullscreen_client_inside_its_frame() {
        let probe = Probe::new().unwrap();
        let conn = &probe.connection;
        let screen = &conn.setup().roots[0];
        let frame = conn.generate_id().unwrap();
        let client = conn.generate_id().unwrap();
        for (id, parent) in [(frame, screen.root), (client, frame)] {
            conn.create_window(
                0,
                id,
                parent,
                0,
                0,
                200,
                200,
                0,
                WindowClass::INPUT_OUTPUT,
                screen.root_visual,
                &CreateWindowAux::new(),
            )
            .unwrap()
            .check()
            .unwrap();
            conn.map_window(id).unwrap().check().unwrap();
        }
        assert!(!probe.fullscreen_at(100, 100));
        conn.change_property32(
            PropMode::REPLACE,
            client,
            probe.state,
            AtomEnum::ATOM,
            &[probe.fullscreen],
        )
        .unwrap()
        .check()
        .unwrap();
        assert!(probe.fullscreen_at(100, 100));
        assert!(!probe.fullscreen_at(300, 300));
        conn.delete_property(client, probe.state)
            .unwrap()
            .check()
            .unwrap();
        assert!(!probe.fullscreen_at(100, 100));
    }
}

pub fn set_mask(window: &WebviewWindow, hidden: bool) -> bool {
    let Ok(widget) = window.gtk_window() else {
        return false;
    };
    let region = hidden.then(gtk::cairo::Region::create);
    widget.shape_combine_region(region.as_ref());
    widget.input_shape_combine_region(region.as_ref());
    true
}
