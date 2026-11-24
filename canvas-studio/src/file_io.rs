use std::io::Cursor;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
pub fn open_image_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Image Files", &["png", "jpg", "jpeg", "bmp", "gif"])
        .add_filter("All Files", &["*"])
        .pick_file()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_image_dialog(default_name: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_file_name(default_name)
        .add_filter("PNG Image (*.png)", &["png"])
        .add_filter("JPEG Image (*.jpg)", &["jpg", "jpeg"])
        .add_filter("BMP Image (*.bmp)", &["bmp"])
        .save_file()
}

pub fn decode_rgba_bytes(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("Failed to decode image: {e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}

pub fn encode_rgba_bytes(ext: &str, width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    let ext_lower = ext.to_ascii_lowercase();
    if ext_lower == "jpg" || ext_lower == "jpeg" {
        let mut rgb = Vec::with_capacity((width * height * 3) as usize);
        for px in rgba.chunks_exact(4) {
            let a = px[3] as f32 / 255.0;
            rgb.push((px[0] as f32 * a + 255.0 * (1.0 - a)) as u8);
            rgb.push((px[1] as f32 * a + 255.0 * (1.0 - a)) as u8);
            rgb.push((px[2] as f32 * a + 255.0 * (1.0 - a)) as u8);
        }
        image::write_buffer_with_format(
            &mut out,
            &rgb,
            width,
            height,
            image::ColorType::Rgb8,
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| format!("Failed to encode JPEG: {e}"))?;
    } else if ext_lower == "bmp" {
        image::write_buffer_with_format(
            &mut out,
            rgba,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::Bmp,
        )
        .map_err(|e| format!("Failed to encode BMP: {e}"))?;
    } else {
        image::write_buffer_with_format(
            &mut out,
            rgba,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("Failed to encode PNG: {e}"))?;
    }
    Ok(out.into_inner())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_rgba_file(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let img = image::open(path).map_err(|e| format!("Failed to open image: {e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((w, h, rgba.into_raw()))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_rgba_file(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "png".to_string());

    let bytes = encode_rgba_bytes(&ext, width, height, rgba)?;
    std::fs::write(path, bytes).map_err(|e| format!("Failed to save image: {e}"))
}

#[cfg(target_arch = "wasm32")]
pub struct PendingWebFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub as_layer: bool,
}

#[cfg(target_arch = "wasm32")]
pub fn trigger_web_open_dialog(
    queue: std::rc::Rc<std::cell::RefCell<Vec<PendingWebFile>>>,
    ctx: eframe::egui::Context,
    as_layer: bool,
) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else { return };
    let Some(document) = window.document() else { return };
    let Ok(el) = document.create_element("input") else { return };
    let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() else { return };

    input.set_type("file");
    input.set_accept("image/png,image/jpeg,image/bmp,image/gif");

    let input_clone = input.clone();
    let onchange = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
        let Some(files) = input_clone.files() else { return };
        let Some(file) = files.get(0) else { return };
        let name = file.name();
        let Ok(reader) = web_sys::FileReader::new() else { return };
        let reader_clone = reader.clone();
        let queue_clone = queue.clone();
        let ctx_clone = ctx.clone();

        let onload = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
            if let Ok(result) = reader_clone.result() {
                let array = js_sys::Uint8Array::new(&result);
                let bytes = array.to_vec();
                queue_clone.borrow_mut().push(PendingWebFile {
                    name: name.clone(),
                    bytes,
                    as_layer,
                });
                ctx_clone.request_repaint();
            }
        });
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
        let _ = reader.read_as_array_buffer(&file);
    });

    input.set_onchange(Some(onchange.as_ref().unchecked_ref()));
    onchange.forget();
    input.click();
}

#[cfg(target_arch = "wasm32")]
pub fn download_web_file(filename: &str, mime: &str, data: &[u8]) -> Result<(), String> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or("No browser window")?;
    let document = window.document().ok_or("No browser document")?;

    let uint8 = js_sys::Uint8Array::from(data);
    let array = js_sys::Array::new();
    array.push(&uint8.buffer());

    let bag = web_sys::BlobPropertyBag::new();
    bag.set_type(mime);

    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&array, &bag)
        .map_err(|e| format!("Failed to create blob: {e:?}"))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| format!("Failed to create object URL: {e:?}"))?;

    let anchor = document
        .create_element("a")
        .map_err(|e| format!("Failed to create anchor: {e:?}"))?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| "Anchor cast failed".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();

    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}
