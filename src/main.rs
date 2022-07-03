use mini_redis::{Connection, Frame, Result};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
pub async fn main() -> Result<()> {
    // リスナーをアドレスにバインドする。
    let listener = TcpListener::bind("localhost:6379").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        // 入ってくるソケットそれぞれについて新しいタスクを生成する。
        // ソケットは新しいタスクにムーブされて、そこで処理される。
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}

async fn process(socket: TcpStream) {
    // `Connection`はバイトストリームの代わりに、Redisフレームを読み書きさせてくれる。
    // `Connection`型はmini-redisによって定義されている。
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.unwrap() {
        println!("獲得: {:?}", frame);

        // エラーで応答する。
        let response = Frame::Error("実装していません。".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}
