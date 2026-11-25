// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! En Linux: una ventana de X11 dentro de la del otro programa. Ese programa marca el paso llamando a `latido`.
use crate::lienzo::Lienzo;
use beethoven_daw_demo::instancia::Instancia;
use eframe::egui::{Modifiers, PointerButton, pos2};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle};
use std::ffi::c_void;
use std::num::NonZeroU32;
use std::ptr::NonNull;
use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::*;
use x11rb::xcb_ffi::XCBConnection;

pub struct Ventana {
    // La superficie se suelta antes que su ventana y su conexión.
    l: Lienzo,
    c: XCBConnection,
    pantalla: usize,
    w: Window,
}

fn modos(s: KeyButMask) -> Modifiers {
    let hay = |m: KeyButMask| u16::from(s) & u16::from(m) != 0;
    let ctrl = hay(KeyButMask::CONTROL);
    Modifiers { alt: hay(KeyButMask::MOD1), ctrl, shift: hay(KeyButMask::SHIFT), mac_cmd: false, command: ctrl }
}

impl Ventana {
    pub fn nueva(n: &'static Instancia) -> Option<Ventana> {
        let (c, pantalla) = XCBConnection::connect(None).ok()?;
        Some(Ventana { l: Lienzo::nuevo(n, 1.0), c, pantalla, w: 0 })
    }

    /// En píxeles, como los pide X11.
    pub fn tam(&self) -> (u32, u32) {
        self.l.pixeles()
    }

    pub fn al_cuadro(&mut self, f: Box<dyn FnMut()>) {
        self.l.al_cuadro(f);
    }

    pub fn pegar(&mut self, padre: *mut c_void) -> bool {
        let (w, h) = self.tam();
        let hacer = || -> Option<Window> {
            let id = self.c.generate_id().ok()?;
            let ev = EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION | EventMask::LEAVE_WINDOW | EventMask::EXPOSURE;
            let a = CreateWindowAux::new().event_mask(ev).background_pixel(0);
            self.c
                .create_window(
                    COPY_DEPTH_FROM_PARENT,
                    id,
                    padre as usize as Window,
                    0,
                    0,
                    w as u16,
                    h as u16,
                    0,
                    WindowClass::INPUT_OUTPUT,
                    x11rb::COPY_FROM_PARENT,
                    &a,
                )
                .ok()?
                .check()
                .ok()?;
            self.c.map_window(id).ok()?;
            self.c.flush().ok()?;
            Some(id)
        };
        match hacer() {
            Some(id) => {
                self.w = id;
                true
            }
            None => false,
        }
    }

    /// Atiende lo que pasó y dibuja. Lo llama el reloj del otro programa, en su hilo de ventanas.
    pub fn latido(&mut self) {
        while let Ok(Some(e)) = self.c.poll_for_event() {
            match e {
                Event::ButtonPress(e) => self.boton(&e, true),
                Event::ButtonRelease(e) => self.boton(&e, false),
                Event::MotionNotify(e) => {
                    self.l.modos(modos(e.state));
                    self.l.raton(pos2(e.event_x as f32, e.event_y as f32));
                }
                Event::LeaveNotify(_) => self.l.se_fue(),
                _ => {}
            }
        }
        let Some(w) = NonZeroU32::new(self.w) else { return };
        let Some(c) = NonNull::new(self.c.get_raw_xcb_connection()) else { return };
        let d = XcbDisplayHandle::new(Some(c), self.pantalla as i32);
        unsafe { self.l.cuadro(RawDisplayHandle::Xcb(d), RawWindowHandle::Xcb(XcbWindowHandle::new(w))) }
    }

    fn boton(&mut self, e: &ButtonPressEvent, si: bool) {
        self.l.modos(modos(e.state));
        let o = pos2(e.event_x as f32, e.event_y as f32);
        match e.detail {
            1 => self.l.boton(o, PointerButton::Primary, si),
            3 => self.l.boton(o, PointerButton::Secondary, si),
            // La rueda llega como botones.
            4 if si => self.l.rueda(0.0, 50.0),
            5 if si => self.l.rueda(0.0, -50.0),
            6 if si => self.l.rueda(50.0, 0.0),
            7 if si => self.l.rueda(-50.0, 0.0),
            _ => {}
        }
    }
}

impl Drop for Ventana {
    fn drop(&mut self) {
        self.l.soltar();
        if self.w != 0 {
            let _ = self.c.destroy_window(self.w);
            let _ = self.c.flush();
        }
    }
}
