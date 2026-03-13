use serde::Deserialize;
use tauri::webview::{PageLoadEvent, WebviewBuilder};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl,
};

const PREVIEW_LABEL: &str = "preview-embedded";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PreviewPageLoadEvent {
    pub event: String,
    pub url: String,
}

fn with_preview_webview<F>(app: &AppHandle, f: F) -> Result<(), String>
where
    F: FnOnce(tauri::webview::Webview) -> Result<(), String>,
{
    let window = app
        .get_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    let webview = window
        .webviews()
        .into_iter()
        .find(|view| view.label() == PREVIEW_LABEL)
        .ok_or_else(|| "Preview webview not initialized".to_string())?;

    f(webview)
}

fn to_physical(bounds: &PreviewBounds) -> (PhysicalPosition<i32>, PhysicalSize<u32>) {
    let scale = if bounds.scale_factor > 0.0 {
        bounds.scale_factor
    } else {
        1.0
    };

    let x = (bounds.x * scale).round() as i32;
    let y = (bounds.y * scale).round() as i32;
    let width = (bounds.width * scale).round().max(1.0) as u32;
    let height = (bounds.height * scale).round().max(1.0) as u32;

    (PhysicalPosition::new(x, y), PhysicalSize::new(width, height))
}

#[tauri::command]
pub async fn create_preview_webview(
    app: AppHandle,
    url: String,
    bounds: PreviewBounds,
) -> Result<(), String> {
    log::info!("[preview] create_preview_webview called with url: {}, bounds: {:?}", url, bounds);

    let window = app
        .get_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    if window
        .webviews()
        .iter()
        .any(|view| view.label() == PREVIEW_LABEL)
    {
        log::info!("[preview] Webview already exists, updating bounds");
        return set_preview_bounds(app, bounds).await;
    }

    let parsed_url: url::Url = url
        .parse()
        .map_err(|e: url::ParseError| {
            log::error!("[preview] URL parse error: {}", e);
            e.to_string()
        })?;

    log::info!("[preview] Creating new webview for: {}", parsed_url);

    let app_handle = app.clone();
    let builder = WebviewBuilder::new(PREVIEW_LABEL, WebviewUrl::External(parsed_url))
        .auto_resize()
        .devtools(true)
        .on_page_load(move |_webview, payload| {
            let event = match payload.event() {
                PageLoadEvent::Started => "started",
                PageLoadEvent::Finished => "finished",
            };
            log::info!("[preview] Page load event: {} - {}", event, payload.url());
            let _ = app_handle.emit(
                "preview-page-load",
                PreviewPageLoadEvent {
                    event: event.to_string(),
                    url: payload.url().to_string(),
                },
            );
        });

    let (position, size) = to_physical(&bounds);
    log::info!("[preview] Position: {:?}, Size: {:?}", position, size);

    window
        .add_child(builder, position, size)
        .map_err(|e: tauri::Error| {
            log::error!("[preview] add_child error: {}", e);
            e.to_string()
        })?;

    log::info!("[preview] Webview created successfully");

    Ok(())
}

#[tauri::command]
pub async fn set_preview_bounds(app: AppHandle, bounds: PreviewBounds) -> Result<(), String> {
    with_preview_webview(&app, |webview| {
        if bounds.width <= 1.0 || bounds.height <= 1.0 {
            webview.hide().map_err(|e: tauri::Error| e.to_string())?;
            return Ok(());
        }

        let (position, size) = to_physical(&bounds);

        webview
            .set_position(position)
            .map_err(|e: tauri::Error| e.to_string())?;
        webview
            .set_size(size)
            .map_err(|e: tauri::Error| e.to_string())?;
        webview.show().map_err(|e: tauri::Error| e.to_string())?;
        Ok(())
    })
}

#[tauri::command]
pub async fn navigate_preview_webview(app: AppHandle, url: String) -> Result<(), String> {
    log::info!("[preview] navigate_preview_webview called with url: {}", url);

    let parsed_url: url::Url = url
        .parse()
        .map_err(|e: url::ParseError| {
            log::error!("[preview] URL parse error: {}", e);
            e.to_string()
        })?;

    let result = with_preview_webview(&app, |webview| {
        log::info!("[preview] Navigating webview to: {}", parsed_url);
        webview.navigate(parsed_url).map_err(|e| {
            log::error!("[preview] Navigate error: {}", e);
            e.to_string()
        })
    });

    if let Err(ref e) = result {
        log::error!("[preview] with_preview_webview error: {}", e);
    }

    result
}

#[tauri::command]
pub async fn reload_preview_webview(app: AppHandle) -> Result<(), String> {
    with_preview_webview(&app, |webview| webview.reload().map_err(|e| e.to_string()))
}

#[tauri::command]
pub async fn open_preview_devtools(app: AppHandle) -> Result<(), String> {
    with_preview_webview(&app, |webview| {
        webview.open_devtools();
        Ok(())
    })
}

#[tauri::command]
pub async fn close_preview_devtools(app: AppHandle) -> Result<(), String> {
    with_preview_webview(&app, |webview| {
        webview.close_devtools();
        Ok(())
    })
}

#[tauri::command]
pub async fn close_preview_webview(app: AppHandle) -> Result<(), String> {
    with_preview_webview(&app, |webview| webview.close().map_err(|e| e.to_string()))
}
