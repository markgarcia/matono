use matono_client::actual_main;

#[cfg(not(target_family = "wasm"))]
#[cfg_attr(not(target_family = "wasm"), tokio::main)]
pub async fn main() {
    actual_main().await
}

#[cfg(target_family = "wasm")]
pub fn main() {}
