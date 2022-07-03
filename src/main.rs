use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use mini_redis::{Connection, Frame, Result};
use tokio::net::{TcpListener, TcpStream};

type Db = Arc<Mutex<HashMap<String, Bytes>>>;

#[tokio::main]
pub async fn main() -> Result<()> {
    // リスナーをアドレスにバインドする。
    let listener = TcpListener::bind("localhost:6379").await.unwrap();

    println!("リスニングしています...");

    let db = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        // ハッシュマップへのハンドラをクローン
        let db = db.clone();

        println!("受信しました。");
        tokio::spawn(async move {
            process(socket, db).await;
        });
    }
}

/// 入ってくるコマンドを処理するために`process`関数を実装する。
/// 値を蓄積するために`HashMap`を使用する。
/// `SET`コマンドは`HashMap`の中に値を挿入して、`GET`はそれらを読み出す。
/// 加えて、接続につき1つのコマンドより多く受け付けるためにループを使用する。
async fn process(socket: TcpStream, db: Db) {
    use mini_redis::Command::{self, Get, Set};

    // `mini-redis`が提供する`Connection`はソケットから来るフレームを解析処理する。
    let mut connection = Connection::new(socket);

    // 接続から来るコマンドを受け取るために`read_frame`を使用する。
    while let Some(frame) = connection.read_frame().await.unwrap() {
        let response = match Command::from_frame(frame).unwrap() {
            Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
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
