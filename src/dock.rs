//! The dock menu (macOS), which snooze.rs and policy.rs add their submenus to.
//!
//! Tauri has no dock menu, so this one is built with muda, which Tauri's menus
//! are built on (so its items' events reach on_menu_event all the same), and
//! handed to AppKit by the app delegate's applicationDockMenu:, which tao's
//! delegate doesn't have, so it gets one added.

use std::cell::RefCell;

use muda::ContextMenu;
use objc2::runtime::{AnyObject, Imp, Sel};
use objc2::{class, msg_send, sel};

thread_local! {
    // muda's menus live on the main thread.
    static MENU: RefCell<Option<muda::Menu>> = const { RefCell::new(None) };
}

/// Appends an item to the dock menu, adding the menu to the dock first if
/// need be. Call on the main thread.
pub fn append(item: &dyn muda::IsMenuItem) -> tauri::Result<()> {
    if MENU.with_borrow(Option::is_none) {
        MENU.with_borrow_mut(|menu| *menu = Some(muda::Menu::new()));
        install();
    }
    MENU.with_borrow(|menu| menu.as_ref().unwrap().append(item))?;
    Ok(())
}

fn install() {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        let Some(delegate) = delegate.as_ref() else { return };
        let imp: Imp = std::mem::transmute(
            dock_menu as extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
        );
        let class = delegate.class() as *const _ as *mut _;
        if !objc2::ffi::class_addMethod(class, sel!(applicationDockMenu:), imp, c"@@:@".as_ptr()).as_bool() {
            eprintln!("dock menu unavailable: the app delegate already has one");
        }
        // NSApplication may look over its delegate's methods only when it's set.
        let _: () = msg_send![app, setDelegate: delegate];
    }
}

extern "C-unwind" fn dock_menu(_: &AnyObject, _: Sel, _: *mut AnyObject) -> *mut AnyObject {
    MENU.with_borrow(|menu| menu.as_ref().map_or(std::ptr::null_mut(), |menu| menu.ns_menu().cast()))
}
