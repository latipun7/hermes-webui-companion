//! Bubble window utilities — positioning, visibility polling.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::debug;

/// Position the bubble window centered above the pet window.
/// If there's no room above, position it below.
pub fn reposition_bubble(pet: &tauri::WebviewWindow, bubble: &tauri::WebviewWindow) {
    let Ok(pet_pos) = pet.outer_position() else { return };
    let Ok(pet_size) = pet.outer_size() else { return };

    let bubble_w = 320i32;
    let bubble_h = 84i32;
    let mut x = pet_pos.x + (pet_size.width as i32 - bubble_w) / 2;
    let mut y = pet_pos.y.saturating_sub(bubble_h);

    if let Ok(Some(monitor)) = pet.current_monitor() {
        let m = monitor.position();
        let ms = monitor.size();
        if y < m.y {
            y = pet_pos.y + pet_size.height as i32;
        }
        let right_edge = x + bubble_w;
        if right_edge > m.x + ms.width as i32 {
            x = m.x + ms.width as i32 - bubble_w;
        }
        if x < m.x {
            x = m.x;
        }
    }

    let _ = bubble.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(x, y)));
}

/// Spawn a background thread that polls the bubbles_visible flag
/// every 250ms and shows/hides the bubbles window accordingly.
pub fn spawn_bubble_visibility_poller(
    bubbles_visible: Arc<AtomicBool>,
    bubbles: tauri::WebviewWindow,
) {
    let bv = bubbles_visible;
    let bw = bubbles;
    let mut was_visible = true;
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        let want = bv.load(Ordering::SeqCst);
        if want != was_visible {
            was_visible = want;
            if want {
                let _ = bw.show();
            } else {
                let _ = bw.hide();
            }
            debug!("[companion] bubbles window visible = {want}");
        }
    });
}
