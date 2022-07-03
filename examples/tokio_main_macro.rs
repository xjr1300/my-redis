//! ```rust
//! #[tokio::main]
//! async fn main() {
//!     println!("hello");
//! }
//! ```
fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    // `block_on`は与えられたフューチャーを実行して、フューチャーが完了するまで
    // ブロックする。そして、解決した結果を生み出す。
    // フューチャーが生成する任意のタスクやタイマーは、内部的にランタイムで実行される。
    rt.block_on(async {
        println!("hello");
    })
}
