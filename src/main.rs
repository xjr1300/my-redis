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

/// 入ってくるコマンドを処理するために`process`関数を実装する。
/// 値を蓄積するために`HashMap`を使用する。
/// `SET`コマンドは`HashMap`の中に値を挿入して、`GET`はそれらを読み出す。
/// 加えて、接続につき1つのコマンドより多く受け付けるためにループを使用する。
async fn process(socket: TcpStream) {
    use mini_redis::Command::{self, Get, Set};
    use std::collections::HashMap;

    // データを蓄積するために`HashMap`を使用する。
    let mut db = HashMap::new();

    // `mini-redis`が提供する`Connection`はソケットから来るフレームを解析処理する。
    let mut connection = Connection::new(socket);

    // 接続から来るコマンドを受け取るために`read_frame`を使用する。
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                // `Vec<u8>`として値を蓄積する。
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    // `Frame::Bulk`はデータがBytes`型であることを想定する。
                    // この型はチュートリアルの後半で説明する。
                    // 現在のところ、`&Vec<u8>`は`into()`の使用することで`Bytes`に変換される。
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            cmd => panic!("実装されていません。{:?}", cmd),
        };

        // クライアントへの応答を記述する。
        connection.write_frame(&response).await.unwrap();
    }
}
