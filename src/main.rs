//! # 状態の共有
//!
//! これまで、動作するキーバリューサーバーができた。しかしながら、重要な欠陥がある。
//! それは、状態が接続間で共有されていないことである。それをこの記事で（ここで）修正する。
//!
//! ## 戦略
//! Tokioにおいて状態を共有するいくつかの様々な方法がある。
//!
//! 1. Mutexで共有された状態をガードする。
//! 2. 状態を管理するためにタスクを生成して、メッセージを送信することで状態を操作する。
//!
//! 一般的に、単純なデータであれば最初の手段を使用する必要があり、I/Oプリミティブのような非同期処理を必要
//! とする場合は2番目の手段を使用する。この章において、共有された状態は`HashMap`であり、操作は`insert`や
//! `get`である。これらの操作はどちらも非同期ではないため、`Mutex`を使用する。
//!
//! 最後の手段は次の章でカバーする。
//!
//! ## `bytes`依存関係の追加
//!
//! `Vec<u8>`を使用する代わりに、`Mini-Redis`クレートは`bytes`クレートの`Bytes`を賜与する。
//! `Bytes`の最終目的は、ネットワークプログラミングで使用する、強固なバイト配列構造を提供することである。
//! `Vec<u8>`に追加される最大の機能は、浅いクローンである。
//! 言い換えれば、`Bytes`インスタンスに対して`clone()`を呼び出すことは、基礎データをコピーしない。
//! 代わりに、`Bytes`インスタンスは任意の基礎データへの参照カウンターのハンドルである。
//! 大まかに言えば、`Bytes`型はArc<Vec<u8>>`だが、いくつかの追加の能力がある。
//!
//! `bytes`に依存するために、以下を`Cargo.toml`の`[dependencies]`セクションに追加する。
//!
//! ```toml
//! bytes = "1"
//! ```
//! ## `HashMap`の初期化
//!
//! `HashMap`は多くのタスクやおそらく多くのスレッド間で共有される。
//! これを支援するために、`HashMap`を`Arc<Mutex<_>>`内にラップする。
//!
//! まず、利便性のために、`use`文の後に以下の型エイリアスを追加する。
//!
//! ```rust
//! use std::collections::HashMap;
//! use std::sync::{Arc, Mutex};
//!
//! use byes::Bytes;
//!
//! type Db = Arc<Mutex<HashMap<String, Bytes>>>;
//! ```
//!
//! その次に、`HashMap`を初期化して`process`関数に`Arc`ハンドルを渡すために、main
//! 関数を更新する。`Arc`を使用することは、おそらく多くのスレッドで実行されている複数のタスクから
//! 並行的に`HashMap`を参照することが可能になる。Tokioにおいて、任意の共有状態へのアクセスを提供
//! する値を参照するために**ハンドル**(ここでは、`Arc<Mutex<_>>`)という用語が使用されている。
//!
//! ### `std:;sync::Mutex`の使用
//!
//! ここで、`tokio::sync::Mutex`ではなく`std::sync::Mutex`が`HashMap`をガードするために使用
//! されている。非同期のコードの中で、無条件に`tokio::sync::Mutex`を使用することはよくある誤りである。
//! 非同期ミューテックスは、`.await`呼び出し間でロックされるミューテックスである。
//!
//! ロックを獲得するために待っているとき、同期ミューテックスはカレントスレッドをブロックする。
//! これにより、他のタスクの処理がブロックされる。しかしながら、非同期ミューテックスは内部で同期ミューテックスを
//! 使用するため、`tokio::sync::Mutex`に切り替えることを、通常役に立たない。
//!
//! 経験則として、非同期コードで同期ミューテックスを使用することは、競合が少なく`.await`呼び出し間でロックを
//! 保持しない限り、問題ない。加えて、`std::sync::Mutex`の高速な互換として
//! [parking_log::Mutex](https://docs.rs/parking_lot/0.10.2/parking_lot/type.Mutex.html)の使用を
//! 検討してもよい。
//!
//! ## タスク、スレッドそして接続
//!
//! 短いクリティカルセクションをガードするためにブロッキングミューテックスを使用することは、競合が最小限のとき、
//! 受け入れることができる戦略である。
//! ロックが競合したとき、タスクを実行しているスレッドは、ブロックしてミューテックスを待つ必要がある。
//! これは現在のタスクをブロックするだけでなく、カレントスレッドにスケジュールされたすべての他のタスクもブロックする。
//!
//! デフォルトでは、Tokioランタイムはマルチスレッドスケジューラーを使用する。
//! タスクはランタイムによって管理された数個のスレッドにスケジュールされる。
//! 実行するために多くのタスクがスケジュールされ、それらのすべてのタスクがミューテックスへのアクセスを必要とした場合、
//! 競合が発生する。一方で、[current_thread](https://docs.rs/tokio/1/tokio/runtime/index.html#current-thread-scheduler)
//! ランタイムフレーバーが使用されている場合、ミューテックスは決して競合しない。
//!
//! `current_thread`ランタイムフレーバーは軽量で、シングルスレッドランタイムである。
//! 数個のタスクのみを生成して、少数のソケットを開く場合、それは良い選択である。
//! 例えば、非同期クライアントライブラリに非同期APIへの橋渡しを提供するとき、このオプションは十分に機能する。
//!
//! 同期ミューテックスの競合が問題になる場合、最善の修正は稀にTokioのミューテックスに切り替えることである。
//! 代わりに、考えられる選択肢は以下である。
//!
//! * 状態を管理してメッセージパッシングを使用する献身的なタスクに切り替える
//! * ミューテックスを共有する
//! * ミューテックスを回避するためにコードを再構築する
//!
//! 本件の場合、それぞれの`key`は独立しているので、ミューテックスシャーディング（水平分割）が十分に機能する。
//! れをするために、1つの`Mutex<HashMap<_, _>>`インスタンスを持つことに代えて、`N`個の区別されたインスタンスを導入する。
//!
//! ```rust
//! type SharedDb = Arc<Vec<Mutex<HashMap<String, Vec<u8>>>>>;
//!
//! fn new_shared_db(num_shards: usize) -> SharedDb {
//!     let mut db = Vec::with_capacity(num_shards);
//!     for _ in 0..num_shards {
//!         db.push(Mutex::new(HashMap::new()));
//!     }
//!
//!     Arc::new(db)
//! }
//!
//! それで、任意の与えられたキーのセルを探すことは、2つのステップの処理からなる。
//! 最初は、キーがどのシャードの部分になるかを示す識別子として使用される。
//! そして、キーは`HashMap`内から検索される。
//!
//! ```rust
//! let shard = db[hash(key) % db.len()].lock().unwrap();
//! shard.insert(key, value);
//! ```
//!
//! 上記で概要を説明した単純な実装では、固定数のシャードを使用する必要があり、共有されたマップが
//! 一度作成されると、シャードの数を変更することができない。
//! [dashmap](https://docs.rs/dashmap)クレートは、より洗練された共有されたハッシュマップの
//! 実装を提供する。
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
