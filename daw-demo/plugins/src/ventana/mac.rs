// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! En Mac: una vista que se pega dentro de la del otro programa.
use crate::lienzo::Lienzo;
use beethoven_daw_demo::instancia::Instancia;
use eframe::egui::{self, Modifiers, PointerButton, pos2};
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{ClassType, MainThreadMarker, msg_send, sel};
use objc2_app_kit::{NSEvent, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{NSPoint, NSRect, NSRunLoop, NSRunLoopCommonModes, NSSize, NSTimer};
use raw_window_handle::{AppKitDisplayHandle, AppKitWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::ffi::{CStr, CString, c_void};
use std::ptr::NonNull;
use std::sync::OnceLock;

pub struct Ventana {
    vista: Retained<NSView>,
    reloj: Option<Retained<NSTimer>>,
    d: *mut Lienzo,
}

const DATO: &CStr = c"beethoven";

unsafe fn dentro<'a>(v: *mut AnyObject) -> Option<&'a mut Lienzo> {
    let p: *mut c_void = *(*v).class().instance_variable(DATO)?.load_ptr::<*mut c_void>(&*v);
    (p as *mut Lienzo).as_mut()
}

unsafe fn lugar(v: *mut AnyObject, e: &NSEvent) -> egui::Pos2 {
    let p: NSPoint = msg_send![v, convertPoint: e.locationInWindow(), fromView: std::ptr::null::<AnyObject>()];
    pos2(p.x as f32, p.y as f32)
}

fn modos(e: &NSEvent) -> Modifiers {
    let f = e.modifierFlags().0;
    let (shift, ctrl, alt, cmd) = (f & 1 << 17 != 0, f & 1 << 18 != 0, f & 1 << 19 != 0, f & 1 << 20 != 0);
    Modifiers { alt, ctrl, shift, mac_cmd: cmd, command: cmd }
}

unsafe fn raton(v: *mut AnyObject, e: *mut NSEvent, boton: Option<(PointerButton, bool)>) {
    crate::seguro((), || {
        let (Some(d), Some(e)) = (dentro(v), e.as_ref()) else { return };
        let o = lugar(v, e);
        let m = modos(e);
        d.modos(m);
        match boton {
            // Control y clic es el botón derecho.
            Some((PointerButton::Primary, si)) if m.ctrl => d.boton(o, PointerButton::Secondary, si),
            Some((b, si)) => d.boton(o, b, si),
            None => d.raton(o),
        }
    })
}

unsafe extern "C-unwind" fn abajo(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    raton(v, e, Some((PointerButton::Primary, true)))
}
unsafe extern "C-unwind" fn arriba(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    raton(v, e, Some((PointerButton::Primary, false)))
}
unsafe extern "C-unwind" fn d_abajo(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    raton(v, e, Some((PointerButton::Secondary, true)))
}
unsafe extern "C-unwind" fn d_arriba(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    raton(v, e, Some((PointerButton::Secondary, false)))
}
unsafe extern "C-unwind" fn mueve(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    raton(v, e, None)
}
unsafe extern "C-unwind" fn sale(v: *mut AnyObject, _: Sel, _: *mut NSEvent) {
    if let Some(d) = dentro(v) {
        d.se_fue();
    }
}
unsafe extern "C-unwind" fn rueda(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    crate::seguro((), || {
        let (Some(d), Some(e)) = (dentro(v), e.as_ref()) else { return };
        let k: f32 = if e.hasPreciseScrollingDeltas() { 1.0 } else { 10.0 };
        d.modos(modos(e));
        d.rueda(e.scrollingDeltaX() as f32 * k, e.scrollingDeltaY() as f32 * k);
    })
}
unsafe extern "C-unwind" fn tecla(v: *mut AnyObject, _: Sel, e: *mut NSEvent) {
    crate::seguro((), || {
        let (Some(d), Some(e)) = (dentro(v), e.as_ref()) else { return };
        // Solo si se está escribiendo; si no, la tecla es del otro programa.
        if !d.escribiendo() {
            let sup: *mut AnyObject = msg_send![v, nextResponder];
            if !sup.is_null() {
                let _: () = msg_send![sup, keyDown: e];
            }
            return;
        }
        let k = match e.keyCode() {
            51 => Some(egui::Key::Backspace),
            36 => Some(egui::Key::Enter),
            53 => Some(egui::Key::Escape),
            123 => Some(egui::Key::ArrowLeft),
            124 => Some(egui::Key::ArrowRight),
            _ => None,
        };
        match k {
            Some(k) => d.tecla(k),
            None => {
                if let Some(s) = e.characters() {
                    d.texto(&s.to_string());
                }
            }
        }
    })
}
unsafe extern "C-unwind" fn volteada(_: *mut AnyObject, _: Sel) -> Bool {
    Bool::YES
}
unsafe extern "C-unwind" fn acepta(_: *mut AnyObject, _: Sel) -> Bool {
    Bool::YES
}
unsafe extern "C-unwind" fn primero(_: *mut AnyObject, _: Sel, _: *mut NSEvent) -> Bool {
    Bool::YES
}
unsafe extern "C-unwind" fn cuadro(v: *mut AnyObject, _: Sel, _: *mut AnyObject) {
    crate::seguro((), || {
        if let Some(d) = dentro(v) {
            cuadro_de(d, v);
        }
    })
}

/// La clase de la vista. Cada copia de este archivo (VST3 y CLAP) registra la suya: en un mismo programa no chocan.
fn clase() -> &'static AnyClass {
    static C: OnceLock<&'static AnyClass> = OnceLock::new();
    C.get_or_init(|| {
        let n = CString::new(format!("BeethovenVista{:p}", &C)).expect("nombre de la clase");
        let mut b = ClassBuilder::new(&n, NSView::class()).expect("clase nueva");
        b.add_ivar::<*mut c_void>(DATO);
        unsafe {
            b.add_method(sel!(mouseDown:), abajo as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(mouseUp:), arriba as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(rightMouseDown:), d_abajo as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(rightMouseUp:), d_arriba as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(mouseDragged:), mueve as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(rightMouseDragged:), mueve as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(mouseMoved:), mueve as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(mouseExited:), sale as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(scrollWheel:), rueda as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(keyDown:), tecla as unsafe extern "C-unwind" fn(_, _, _));
            b.add_method(sel!(isFlipped), volteada as unsafe extern "C-unwind" fn(_, _) -> _);
            b.add_method(sel!(acceptsFirstResponder), acepta as unsafe extern "C-unwind" fn(_, _) -> _);
            b.add_method(sel!(acceptsFirstMouse:), primero as unsafe extern "C-unwind" fn(_, _, _) -> _);
            b.add_method(sel!(beethovenCuadro:), cuadro as unsafe extern "C-unwind" fn(_, _, _));
        }
        b.register()
    })
}

unsafe fn cuadro_de(d: &mut Lienzo, v: *mut AnyObject) {
    let w: *mut AnyObject = msg_send![v, window];
    let Some(p) = NonNull::new(v as *mut c_void).filter(|_| !w.is_null()) else { return };
    let e: f64 = msg_send![w, backingScaleFactor];
    d.escala = e as f32;
    d.cuadro(RawDisplayHandle::AppKit(AppKitDisplayHandle::new()), RawWindowHandle::AppKit(AppKitWindowHandle::new(p)));
}

impl Ventana {
    pub fn nueva(n: &'static Instancia) -> Option<Ventana> {
        let mtm = MainThreadMarker::new()?;
        let _ = mtm;
        let l = Lienzo::nuevo(n, 2.0);
        let tam = l.tam;
        let d = Box::into_raw(Box::new(l));
        unsafe {
            let r = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(tam.0 as f64, tam.1 as f64));
            let v: *mut AnyObject = msg_send![clase(), alloc];
            let v: *mut AnyObject = msg_send![v, initWithFrame: r];
            let vista: Retained<NSView> = Retained::from_raw(v as *mut NSView)?;
            *(*v).class().instance_variable(DATO)?.load_ptr::<*mut c_void>(&*v) = d as *mut c_void;
            vista.setWantsLayer(true);
            let area: Retained<NSTrackingArea> = {
                let a: *mut AnyObject = msg_send![NSTrackingArea::class(), alloc];
                let o = NSTrackingAreaOptions::MouseMoved
                    | NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect;
                let a: *mut AnyObject = msg_send![a, initWithRect: r, options: o, owner: v, userInfo: std::ptr::null::<AnyObject>()];
                Retained::from_raw(a as *mut NSTrackingArea)?
            };
            vista.addTrackingArea(&area);
            Some(Ventana { vista, reloj: None, d })
        }
    }

    /// En puntos, como los pide Mac.
    pub fn tam(&self) -> (u32, u32) {
        unsafe { (*self.d).tam }
    }

    pub fn al_cuadro(&mut self, f: Box<dyn FnMut()>) {
        unsafe { (*self.d).al_cuadro(f) }
    }

    /// Dentro de la vista del otro programa.
    pub fn pegar(&mut self, padre: *mut c_void) -> bool {
        unsafe {
            let Some(p) = (padre as *mut NSView).as_ref() else { return false };
            p.addSubview(&self.vista);
            let v = Retained::as_ptr(&self.vista) as *const AnyObject;
            let t = NSTimer::timerWithTimeInterval_target_selector_userInfo_repeats(1.0 / 60.0, &*v, sel!(beethovenCuadro:), None, true);
            NSRunLoop::mainRunLoop().addTimer_forMode(&t, NSRunLoopCommonModes);
            self.reloj = Some(t);
            true
        }
    }
}

impl Drop for Ventana {
    fn drop(&mut self) {
        unsafe {
            if let Some(t) = self.reloj.take() {
                t.invalidate();
            }
            self.vista.removeFromSuperview();
            let v = Retained::as_ptr(&self.vista) as *mut AnyObject;
            if let Some(i) = (*v).class().instance_variable(DATO) {
                *i.load_ptr::<*mut c_void>(&*v) = std::ptr::null_mut();
            }
            drop(Box::from_raw(self.d));
        }
    }
}
