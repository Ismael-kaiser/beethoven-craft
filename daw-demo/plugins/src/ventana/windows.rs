// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! En Windows: una ventana hija de la del otro programa.
use crate::lienzo::Lienzo;
use beethoven_daw_demo::instancia::Instancia;
use eframe::egui::{self, Modifiers, PointerButton, pos2};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowsDisplayHandle};
use std::ffi::c_void;
use std::num::NonZeroIsize;
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{ScreenToClient, ValidateRect};
use windows_sys::Win32::System::LibraryLoader::{
    GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleHandleExW,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const RELOJ: usize = 1;

pub struct Ventana {
    w: HWND,
    d: *mut Lienzo,
}

fn modos() -> Modifiers {
    let abajo = |k: u16| unsafe { GetKeyState(k as i32) < 0 };
    let (shift, ctrl, alt) = (abajo(VK_SHIFT), abajo(VK_CONTROL), abajo(VK_MENU));
    Modifiers { alt, ctrl, shift, mac_cmd: false, command: ctrl }
}

unsafe extern "system" fn proceso(w: HWND, m: u32, a: WPARAM, l: LPARAM) -> LRESULT {
    let r = crate::seguro(None, || atender(w, m, a, l));
    r.unwrap_or_else(|| DefWindowProcW(w, m, a, l))
}

unsafe fn atender(w: HWND, m: u32, a: WPARAM, l: LPARAM) -> Option<LRESULT> {
    let d = (GetWindowLongPtrW(w, GWLP_USERDATA) as *mut Lienzo).as_mut()?;
    let (x, y) = ((l & 0xffff) as i16 as f32, (l >> 16 & 0xffff) as i16 as f32);
    let o = pos2(x / d.escala, y / d.escala);
    let boton = |d: &mut Lienzo, b: PointerButton, si: bool| {
        d.modos(modos());
        d.boton(o, b, si);
        // Para seguir el arrastre aunque salga de la ventana.
        if si {
            SetCapture(w);
            SetFocus(w);
        } else {
            ReleaseCapture();
        }
        Some(0)
    };
    match m {
        WM_TIMER if a == RELOJ => {
            let h = Win32WindowHandle::new(NonZeroIsize::new(w as isize)?);
            d.cuadro(RawDisplayHandle::Windows(WindowsDisplayHandle::new()), RawWindowHandle::Win32(h));
            Some(0)
        }
        WM_PAINT => {
            ValidateRect(w, std::ptr::null());
            Some(0)
        }
        WM_ERASEBKGND => Some(1),
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => boton(d, PointerButton::Primary, true),
        WM_LBUTTONUP => boton(d, PointerButton::Primary, false),
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => boton(d, PointerButton::Secondary, true),
        WM_RBUTTONUP => boton(d, PointerButton::Secondary, false),
        WM_MOUSEMOVE => {
            d.modos(modos());
            d.raton(o);
            let mut t = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: w, dwHoverTime: 0 };
            TrackMouseEvent(&mut t);
            Some(0)
        }
        // La que define Windows: aquí no hace falta traer todos los controles comunes.
        0x02A3 => {
            d.se_fue();
            Some(0)
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
            // Aquí el lugar viene en la pantalla.
            let mut p = POINT { x: x as i32, y: y as i32 };
            ScreenToClient(w, &mut p);
            d.modos(modos());
            d.raton(pos2(p.x as f32 / d.escala, p.y as f32 / d.escala));
            let v = (a >> 16 & 0xffff) as i16 as f32 / 120.0 * 50.0;
            if m == WM_MOUSEWHEEL {
                d.rueda(0.0, v)
            } else {
                d.rueda(-v, 0.0)
            }
            Some(0)
        }
        WM_CHAR if d.escribiendo() => {
            if let Some(c) = char::from_u32(a as u32) {
                d.texto(&c.to_string());
            }
            Some(0)
        }
        WM_KEYDOWN if d.escribiendo() => {
            let k = match a as u16 {
                VK_BACK => egui::Key::Backspace,
                VK_RETURN => egui::Key::Enter,
                VK_ESCAPE => egui::Key::Escape,
                VK_LEFT => egui::Key::ArrowLeft,
                VK_RIGHT => egui::Key::ArrowRight,
                VK_DELETE => egui::Key::Delete,
                _ => return None,
            };
            d.modos(modos());
            d.tecla(k);
            Some(0)
        }
        // Las demás teclas son del otro programa.
        WM_KEYDOWN | WM_KEYUP | WM_CHAR | WM_SYSKEYDOWN | WM_SYSKEYUP => {
            let p = GetParent(w);
            if !p.is_null() {
                PostMessageW(p, m, a, l);
            }
            Some(0)
        }
        _ => None,
    }
}

/// Este mismo archivo, no el programa que lo abrió.
fn modulo() -> *mut c_void {
    let mut h = std::ptr::null_mut();
    unsafe {
        let f = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;
        GetModuleHandleExW(f, proceso as *const () as *const u16, &mut h);
    }
    h
}

/// La clase de la ventana. Cada copia de este archivo (VST3 y CLAP) registra la suya.
fn clase() -> &'static [u16] {
    static C: OnceLock<Vec<u16>> = OnceLock::new();
    C.get_or_init(|| {
        let n: Vec<u16> = format!("BeethovenVista{:p}\0", &C).encode_utf16().collect();
        unsafe {
            let mut k: WNDCLASSW = std::mem::zeroed();
            k.style = CS_DBLCLKS | CS_OWNDC;
            k.lpfnWndProc = Some(proceso);
            k.hInstance = modulo();
            k.hCursor = LoadCursorW(std::ptr::null_mut(), IDC_ARROW);
            k.lpszClassName = n.as_ptr();
            RegisterClassW(&k);
        }
        n
    })
}

/// Píxeles por punto.
pub fn escala() -> f32 {
    (unsafe { GetDpiForSystem() } as f32 / 96.0).clamp(1.0, 4.0)
}

impl Ventana {
    pub fn nueva(n: &'static Instancia) -> Option<Ventana> {
        let d = Box::into_raw(Box::new(Lienzo::nuevo(n, escala())));
        Some(Ventana { w: std::ptr::null_mut(), d })
    }

    /// En píxeles, como los pide Windows.
    pub fn tam(&self) -> (u32, u32) {
        unsafe { (*self.d).pixeles() }
    }

    pub fn al_cuadro(&mut self, f: Box<dyn FnMut()>) {
        unsafe { (*self.d).al_cuadro(f) }
    }

    pub fn pegar(&mut self, padre: *mut c_void) -> bool {
        let (w, h) = self.tam();
        unsafe {
            let estilo = WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_CLIPSIBLINGS;
            let nada = std::ptr::null();
            self.w = CreateWindowExW(
                0,
                clase().as_ptr(),
                nada,
                estilo,
                0,
                0,
                w as i32,
                h as i32,
                padre as HWND,
                std::ptr::null_mut(),
                modulo(),
                nada as *const c_void,
            );
            if self.w.is_null() {
                return false;
            }
            SetWindowLongPtrW(self.w, GWLP_USERDATA, self.d as isize);
            SetTimer(self.w, RELOJ, 16, None);
            true
        }
    }
}

impl Drop for Ventana {
    fn drop(&mut self) {
        unsafe {
            if !self.w.is_null() {
                KillTimer(self.w, RELOJ);
                SetWindowLongPtrW(self.w, GWLP_USERDATA, 0);
            }
            // La superficie se suelta antes que su ventana.
            drop(Box::from_raw(self.d));
            if !self.w.is_null() {
                DestroyWindow(self.w);
            }
        }
    }
}
