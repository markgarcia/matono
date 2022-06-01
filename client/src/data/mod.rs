pub async fn read_bytes(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let buffer: Vec<u8>;

    #[cfg(not(target_family = "wasm"))]
    {
        buffer = tokio::fs::read(&path)
            .await
            .map_err(|_| format!("Failed to read file {}", path))?;
    }

    #[cfg(target_family = "wasm")]
    {
        use js_sys::{ArrayBuffer, Uint8Array};
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Blob, Request, Response};

        let fetch_future = JsFuture::from({
            let request = Request::new_with_str(&path)
                .map_err(|_| format!("Unable to create HTTP request: {}", path))?;

            web_sys::window().unwrap().fetch_with_request(&request)
        });

        let blob_future = JsFuture::from({
            let response: Response = fetch_future
                .await
                .map_err(|_| format!("HTTP fetch failed: {}", path))?
                .dyn_into()
                .unwrap();

            response
                .blob()
                .map_err(|_| format!("HTTP fetch failed to extract blob: {}", path))?
        });

        let array_future = JsFuture::from({
            let response_blob: Blob = blob_future
                .await
                .map_err(|_| format!("HTTP fetch failed to extract blob: {}", path))?
                .into();

            response_blob.array_buffer()
        });

        let array_buffer: ArrayBuffer = array_future.await.unwrap().into();
        buffer = Uint8Array::new(array_buffer.as_ref()).to_vec();
    }

    Ok(buffer)
}
