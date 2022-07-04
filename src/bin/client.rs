use bytes::Bytes;
use mini_redis::client;
use tokio::sync::{mpsc, oneshot};

/// 複数の異なるコマンドが1つのチャネルで多重化される。
#[derive(Debug)]
enum Command {
    Get {
        key: String,
        resp: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        val: Bytes,
        resp: Responder<()>,
    },
}

/// 要求者によって提供され、コマンドの応答を要求者に返送するために使用される。
type Responder<T> = oneshot::Sender<mini_redis::Result<T>>;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    // `送信者`ハンドルはタスク内にムーブされる。2つのタスクがあるため、2つ目の`送信者`が必要になる。
    let tx2 = tx.clone();

    let manager = tokio::spawn(async move {
        // mini-redisアドレスへの接続をオープンする。
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        // コマンドがチャネルに送信されるのを待ち、コマンドが送信されたことを検知したら、コマンドを処理する。
        while let Some(cmd) = rx.recv().await {
            match cmd {
                // mini-redisからキーに紐づけられた値を取得する。
                Command::Get { key, resp } => {
                    let res = client.get(&key).await;
                    // エラーを無視する。
                    let _ = resp.send(res);
                }
                // mini-redisにキーと値を設定する。
                Command::Set { key, val, resp } => {
                    let res = client.set(&key, val).await;
                    // エラーを無視する。
                    let _ = resp.send(res);
                }
            }
        }
    });

    // 2つのタスクを生成して、1つは値を設定して、その他は設定されたキーをユーイングする。
    let t1 = tokio::spawn(async move {
        // oneshotチャネルを作成して、応答をこのチャネルで受け取れるようにする。
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "foo".to_string(),
            resp: resp_tx, // 応答を受信するチャネルを設定する。
        };

        // `mpsc`チャネルにGETリクエストを送信する。
        if tx.send(cmd).await.is_err() {
            eprintln!("connection task shutdown");
            return;
        }

        // oneshotチャネルに届く応答を待つ。
        let res = resp_rx.await;
        println!("GOT (Get) = {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "foo".to_string(),
            val: "bar".into(),
            resp: resp_tx,
        };

        // SETリクエストを送信する。
        if tx2.send(cmd).await.is_err() {
            eprintln!("connection task shutdown");
            return;
        }

        // 応答を待つ。
        let res = resp_rx.await;
        println!("GOT (Set) = {:?}", res);
    });

    // プログラムが終了する前にコマンドが完全に完了することを保証するため、ジョインハンドルを`.await`する。
    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
