use mini_redis::{Connection, Frame, Result};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
pub async fn main() -> Result<()> {
    // リスナーをアドレスにバインドする。
    let listener = TcpListener::bind("localhost:6379").await.unwrap();

    loop {
        // タプルの2つ目のアイテムは、新しいコネクションのIPアドレスとポート番号を含んでいる。
        let (socket, _) = listener.accept().await.unwrap();
        // このコードのサーバーは、エラーで応答することは置いておいて、少し問題がある。
        // このサーバーは、入ってきたリクエストを1度に1つずつ処理する。
        // 接続が受け付けられたとき、レスポンスが完全にソケットに書き出されるまで、サーバーは
        // 内部の受付ループの内部に留まる。
        // ```
        // 並行(`Concurrency`)と並列(`Parallelism`)は同じではない。
        // 2つのタスクを交互に実行する場合、同時に両方のタスクを実行できるが、並列ではない。
        // 並列として実行したいのであれば、2人の人間が必要で、それぞれのタスクにそれぞれの人間に
        // 処理を任せるしかない。
        // Tokioを使用する利点の1つは、非同期コードを記述することによって、通常のスレッドを使用して
        // 多くのタスクを並列に処理するのではなく、並行に処理することができることである。
        // 実際に、Tokioを使用すれば、単一スレッドであっても、多くのタスクを並行して実行する
        // ことができる。
        // ```
        // 並行: CPUコアのような処理ユニットが、単独で複数のタスクを交互に実行すること。
        // 並列: 複数の処理ユニットが、各処理ユニットに与えられた各タスクを同時に実行すること。
        process(socket).await;
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
