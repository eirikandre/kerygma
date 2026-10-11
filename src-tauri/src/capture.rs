//! PDF av et utsnitt av vinduet, til eksport av presentasjonen. Bruker `createPDF` i
//! WKWebView, så teksten blir tekst (ikke et bilde) med de samme skriftene som på skjermen.

use std::sync::mpsc;
use std::time::Duration;

/// Et rektangel i CSS-piksler i nettsiden.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// PDF (én side) av `rect` i nettsiden i `webview`.
#[cfg(target_os = "macos")]
pub fn pdf(webview: &tauri::Webview, rect: Rect) -> Result<Vec<u8>, String> {
    use std::sync::Mutex;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_foundation::{NSData, NSError};
    use objc2_web_kit::{WKPDFConfiguration, WKWebView};

    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();
    webview
        .with_webview(move |platform| {
            // `with_webview` kjører på hovedtråden, der WebKit må brukes.
            let Some(main_thread) = MainThreadMarker::new() else {
                let _ = tx.send(Err("PDF må lages på hovedtråden.".to_string()));
                return;
            };
            // SAFETY: `inner()` er WKWebView-en til vinduet på macOS, og den lever mens closuren kjører.
            let webview: &WKWebView = unsafe { &*platform.inner().cast::<WKWebView>() };
            // SAFETY: et nytt, tomt oppsett, laget på hovedtråden.
            let configuration = unsafe { WKPDFConfiguration::new(main_thread) };
            let tx = Mutex::new(Some(tx));
            let done = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
                // SAFETY: WebKit gir enten data eller en feil, og begge lever mens blokken kjører.
                let result = unsafe {
                    match (data.as_ref(), error.as_ref()) {
                        (Some(data), _) => Ok(data.to_vec()),
                        (None, Some(error)) => Err(format!("Kunne ikke lage PDF: {}", error.localizedDescription())),
                        (None, None) => Err("Kunne ikke lage PDF.".to_string()),
                    }
                };
                if let Some(tx) = tx.lock().unwrap().take() {
                    let _ = tx.send(result);
                }
            });
            // SAFETY: rektangelet er i sidekoordinater, og blokken holdes i live av WebKit til den er kalt.
            unsafe {
                configuration.setRect(CGRect::new(CGPoint::new(rect.x, rect.y), CGSize::new(rect.width, rect.height)));
                webview.createPDFWithConfiguration_completionHandler(Some(&configuration), &done);
            }
        })
        .map_err(|e| format!("Kunne ikke lage PDF: {e}"))?;
    rx.recv_timeout(Duration::from_secs(30)).map_err(|_| "Det tok for lang tid å lage PDF-en.".to_string())?
}

#[cfg(not(target_os = "macos"))]
pub fn pdf(_webview: &tauri::Webview, _rect: Rect) -> Result<Vec<u8>, String> {
    Err("Eksport til PDF virker foreløpig bare på macOS.".to_string())
}
