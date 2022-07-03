use mini_redis::{client, Result};

#[tokio::main]
pub async fn main() -> Result<()> {
    // mini-redisアドレスへのコネクションを開く
    let mut client = client::connect("localhost:6379").await?;

    // "hello"というキーに"world"という値をセット
    client.set("hello", "world".into()).await?;

    // キー"hello"の値を取得
    let result = client.get("hello").await?;

    println!("サーバから値を取得しました; result={:?}", result);

    Ok(())
}
