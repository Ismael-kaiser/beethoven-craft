// Beethoven DAW Demo. Copyright (C) 2026 mixter-ha99. SPDX-License-Identifier: MIT OR Apache-2.0
//! La ventana propia de cada plugin. En Mac es nativa; en los otros sistemas, por ahora, se usan los controles.
use super::Plugin;
use std::ffi::c_void;
use std::sync::Arc;

#[cfg(target_os = "macos")]
mod nativa {
    use super::*;
    use objc2::rc::Retained;
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
    use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

    pub struct Ventana {
        w: Retained<NSWindow>,
        p: Arc<Plugin>,
    }

    impl Ventana {
        pub fn abrir(p: &Arc<Plugin>) -> Option<Ventana> {
            let mtm = MainThreadMarker::new()?;
            let r = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(600.0, 400.0));
            let w = unsafe {
                let w = NSWindow::initWithContentRect_styleMask_backing_defer(
                    NSWindow::alloc(mtm),
                    r,
                    NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
                    NSBackingStoreType::Buffered,
                    false,
                );
                w.setReleasedWhenClosed(false);
                w
            };
            w.setTitle(&NSString::from_str(&p.nombre));
            let vista = w.contentView()?;
            let (x, y) = p.pegar(Retained::as_ptr(&vista) as *mut c_void, Retained::as_ptr(&w) as *mut c_void)?;
            w.setContentSize(NSSize::new(x as f64, y as f64));
            w.makeKeyAndOrderFront(None);
            Some(Ventana { w, p: p.clone() })
        }

        /// Falso cuando ya se cerró.
        pub fn sigue(&self) -> bool {
            if let Some((x, y)) = self.p.tamano_pedido() {
                self.w.setContentSize(NSSize::new(x as f64, y as f64));
            }
            self.w.isVisible()
        }

        pub fn al_frente(&self) {
            self.w.makeKeyAndOrderFront(None);
        }
    }

    impl Drop for Ventana {
        fn drop(&mut self) {
            self.p.despegar();
            self.w.close();
        }
    }

    pub fn medir(w: *mut c_void, x: u32, y: u32) {
        if let Some(w) = unsafe { (w as *const NSWindow).as_ref() } {
            w.setContentSize(NSSize::new(x as f64, y as f64));
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod nativa {
    use super::*;
    pub struct Ventana(Arc<Plugin>);
    impl Ventana {
        pub fn abrir(_: &Arc<Plugin>) -> Option<Ventana> {
            None
        }
        pub fn sigue(&self) -> bool {
            let _ = self.0.tamano_pedido();
            false
        }
        pub fn al_frente(&self) {}
    }
    pub fn medir(_: *mut c_void, _: u32, _: u32) {}
}

pub use nativa::Ventana;

pub(super) fn medir(w: *mut c_void, x: u32, y: u32) {
    nativa::medir(w, x, y);
}
