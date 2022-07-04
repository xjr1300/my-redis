## 状態の共有

これまで、動作するキーバリューサーバーができた。しかしながら、重要な欠陥がある。
それは、状態が接続間で共有されていないことである。それをこの記事で（ここで）修正する。

### 戦略
Tokioにおいて状態を共有するいくつかの様々な方法がある。

1. Mutexで共有された状態をガードする。
2. 状態を管理するためにタスクを生成して、メッセージを送信することで状態を操作する。

一般的に、単純なデータであれば最初の手段を使用する必要があり、I/Oプリミティブのような非同期処理を必要
とする場合は2番目の手段を使用する。この章において、共有された状態は`HashMap`であり、操作は`insert`や
`get`である。これらの操作はどちらも非同期ではないため、`Mutex`を使用する。

最後の手段は次の章でカバーする。

### `bytes`依存関係の追加

`Vec<u8>`を使用する代わりに、`Mini-Redis`クレートは`bytes`クレートの`Bytes`を賜与する。
`Bytes`の最終目的は、ネットワークプログラミングで使用する、強固なバイト配列構造を提供することである。
`Vec<u8>`に追加される最大の機能は、浅いクローンである。
言い換えれば、`Bytes`インスタンスに対して`clone()`を呼び出すことは、基礎データをコピーしない。
代わりに、`Bytes`インスタンスは任意の基礎データへの参照カウンターのハンドルである。
大まかに言えば、`Bytes`型はArc<Vec<u8>>`だが、いくつかの追加の能力がある。

`bytes`に依存するために、以下を`Cargo.toml`の`[dependencies]`セクションに追加する。

```toml
bytes = "1"
```
### `HashMap`の初期化

`HashMap`は多くのタスクやおそらく多くのスレッド間で共有される。
これを支援するために、`HashMap`を`Arc<Mutex<_>>`内にラップする。

まず、利便性のために、`use`文の後に以下の型エイリアスを追加する。

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use byes::Bytes;

type Db = Arc<Mutex<HashMap<String, Bytes>>>;
```

その次に、`HashMap`を初期化して`process`関数に`Arc`ハンドルを渡すために、main
関数を更新する。`Arc`を使用することは、おそらく多くのスレッドで実行されている複数のタスクから
並行的に`HashMap`を参照することが可能になる。Tokioにおいて、任意の共有状態へのアクセスを提供
する値を参照するために**ハンドル**(ここでは、`Arc<Mutex<_>>`)という用語が使用されている。

#### `std:;sync::Mutex`の使用

ここで、`tokio::sync::Mutex`ではなく`std::sync::Mutex`が`HashMap`をガードするために使用
されている。非同期のコードの中で、無条件に`tokio::sync::Mutex`を使用することはよくある誤りである。
非同期ミューテックスは、`.await`呼び出し間でロックされるミューテックスである。

ロックを獲得するために待っているとき、同期ミューテックスはカレントスレッドをブロックする。
これにより、他のタスクの処理がブロックされる。しかしながら、非同期ミューテックスは内部で同期ミューテックスを
使用するため、`tokio::sync::Mutex`に切り替えることを、通常役に立たない。

経験則として、非同期コードで同期ミューテックスを使用することは、競合が少なく`.await`呼び出し間でロックを
保持しない限り、問題ない。加えて、`std::sync::Mutex`の高速な互換として
[parking_log::Mutex](https://docs.rs/parking_lot/0.10.2/parking_lot/type.Mutex.html)の使用を
検討してもよい。

### タスク、スレッドそして接続

短いクリティカルセクションをガードするためにブロッキングミューテックスを使用することは、競合が最小限のとき、
受け入れることができる戦略である。
ロックが競合したとき、タスクを実行しているスレッドは、ブロックしてミューテックスを待つ必要がある。
これは現在のタスクをブロックするだけでなく、カレントスレッドにスケジュールされたすべての他のタスクもブロックする。

デフォルトでは、Tokioランタイムはマルチスレッドスケジューラーを使用する。
タスクはランタイムによって管理された数個のスレッドにスケジュールされる。
実行するために多くのタスクがスケジュールされ、それらのすべてのタスクがミューテックスへのアクセスを必要とした場合、
競合が発生する。一方で、[current_thread](https://docs.rs/tokio/1/tokio/runtime/index.html#current-thread-scheduler)
ランタイムフレーバーが使用されている場合、ミューテックスは決して競合しない。

`current_thread`ランタイムフレーバーは軽量で、シングルスレッドランタイムである。
数個のタスクのみを生成して、少数のソケットを開く場合、それは良い選択である。
例えば、非同期クライアントライブラリに非同期APIへの橋渡しを提供するとき、このオプションは十分に機能する。

同期ミューテックスの競合が問題になる場合、最善の修正は稀にTokioのミューテックスに切り替えることである。
代わりに、考えられる選択肢は以下である。

* 状態を管理してメッセージパッシングを使用する献身的なタスクに切り替える
* ミューテックスを共有する
* ミューテックスを回避するためにコードを再構築する

本件の場合、それぞれの`key`は独立しているので、ミューテックスシャーディング（水平分割）が十分に機能する。
れをするために、1つの`Mutex<HashMap<_, _>>`インスタンスを持つことに代えて、`N`個の区別されたインスタンスを導入する。

```rust
type SharedDb = Arc<Vec<Mutex<HashMap<String, Vec<u8>>>>>;

fn new_shared_db(num_shards: usize) -> SharedDb {
    let mut db = Vec::with_capacity(num_shards);
    for _ in 0..num_shards {
        db.push(Mutex::new(HashMap::new()));
    }

    Arc::new(db)
}

それで、任意の与えられたキーのセルを探すことは、2つのステップの処理からなる。
最初は、キーがどのシャードの部分になるかを示す識別子として使用される。
そして、キーは`HashMap`内から検索される。

```rust
let shard = db[hash(key) % db.len()].lock().unwrap();
shard.insert(key, value);
```

上記で概要を説明した単純な実装では、固定数のシャードを使用する必要があり、共有されたマップが
一度作成されると、シャードの数を変更することができない。
[dashmap](https://docs.rs/dashmap)クレートは、より洗練された共有されたハッシュマップの
実装を提供する。

### `.await`間の`MutexGuard`の保持

このように見えるコードを記述するかもしれない。

```rust
use std::sync::{Mutex, MUtexGuard};

async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
    *lock += 1;

    do_something_async().await;
}   // ここでロックはスコープ外になる。

この関数を呼び出す何かを生成することを試みた場合、以下のエラーメッセージに遭遇するだろう。

error: future cannot be sent between threads safely
   --> src/lib.rs:13:5
   |
13  |     tokio::spawn(async move {
   |     ^^^^^^^^^^^^ future created by async block is not `Send`
   |
  ::: /playground/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-0.2.21/src/task/spawn.rs:127:21
   |
127 |         T: Future + Send + 'static,
   |                     ---- required by this bound in `tokio::task::spawn::spawn`
   |
   = help: within `impl std::future::Future`, the trait `std::marker::Send` is not implemented for `std::sync::MutexGuard<'_, i32>`
note: future is not `Send` as this value is used across an await
  --> src/lib.rs:7:5
   |
4   |     let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
   |         -------- has type `std::sync::MutexGuard<'_, i32>` which is not `Send`
...
7   |     do_something_async().await;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^ await occurs here, with `mut lock` maybe used later
8   | }
   | - `mut lock` is later dropped here

これは、`std::sync::MutexGuard`型が`Send`出ないことが理由で発生する。
これは、他のスレッドにミューテックスを送信できないことを意味しており、そのエラーはTokioランタイムが
すべての`.await`でスレッド間にタスクをムーブできることが理由で発生する。
これを避けるために、むテックスロックが`.await`の前に破壊されるようなコードに再構成する必要がある。

```rust
// これは動作する!
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    {
        let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
        *lock += 1;
    }   // ここでロックがスコープ外になる。

    do_something_async().await;
}

以下は動作しないことに注意しなさい。

```rust
use std::sync::{Mutex, MutexGuard};

// これも失敗する。
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    let mut lock: MutexGuard<i32> = mutex.lock().unwrap();
    *lock += 1;
    drop(lock);

    do_something_async().await;
}

これは、現在コンパイラがスコープの情報のみに基づいてフューチャーが`Send`であるかどうかを計算する
ことが理由である。うまくいけば、将来、コンパイラは明示的なドロップを支援するために更新されるが、
現在では、明示的にスコープを使用する必要がある。

ここで議論したエラーは、[Send bound section from the spawning](https://tokio.rs/tokio/tutorial/spawning#send-bound)
の章でも議論されている。

`Send`であることを要求しない方法でタスクを生成することによって、この問題を回避することを試みるべきではない。
なぜなら、タスクがロックを抱えている間、Tokioは`.await`でそのタスクを中断すると、任意の他のタスクは同じスレッ
ドで実行されるようにスケジュールされているかもしれず、そしてこのその他のタスクはそのミューテックスのロックを試み
るかもしれず、ミューテックスをロックするために待っているタスクは、ミューテックスを保持しているタスクがミューテックス
を解放することが妨げられるためデッドロックとなる。

そのエラーを修正するための手段を以下で議論する。

#### `.await`間でロックを保持しないようにコードを再構成する

上記のスニペットで1つの例を見たが、これをするより強固な方法がいくつかある。
例えば、構造体の中にミューテックスをラップして、その構造体内の非同期でないメソッドでのみミューテックスのロックを獲得する。

```rust
use std::sync::Mutex;

struct CanIncrement {
    mutex: Mutex<i32>,
}
impl CanIncrement {
    // この関数は非同期でないとマークされている。
    fn increment(&self) {
        let mut lock = self.mutex.lock().unwrap();
        *lock += 1;
    }
}

async fn increment_and_do_stuff(can_incr: &CanIncrement) {
    can_incr.increment();
    do_something_async().await();
}

このパターンは、`Send`エラーに陥らないことを保証する。なぜならミューテックスガードが非同期関数内の
どこにも現れないからである。

#### 状態を管理するタスクを生成して、それで操作するためにメッセージパッシングを使用する

これは、この章の最初で言及した2番目の手法で、それは共有するリソースがI/Oリソースのときによく使用される。
詳細は次の章で説明する。

#### Tokioの非同期ミューテックスを使用する

Tokioによって提供される`tokio::sync::Mutex`型もまた使用される。
Tokioミューテックスの主要な機能は、問題なしに`.await`間で保持することができることである。
つまり、非同期ミューテックスは普通のミューテックスよりも高価であり、典型的に2つの他の手段のうち1つを
使用する方が良い。

```rust
use tokio::sync::Mutex;     // Tokioのミューテックスを使用していることに注意

// これはコンパイルできる!
// (しかし、この場合、コードを再構成する方が良いだろう)
async fn increment_and_do_stuff(mutex: &Mutex<i32>) {
    let mut lock = mutex.lock().await;
    *lock += 1;

    do_something_async().await;
}   // ここでロックはスコープ外になる。

## チャネル

現在、Tokioの並行性について少し学んだので、これをクライアント側に適用する。
以前に記述したサーバーのコードを明示的なバイナリファイルに入れる。

```shell
mkdir src/bin
mv src/main.rs src/bin/server.rs
```

そして、クライアントコードを含める予定の新しいバイナリファイルを作成する。

```shell
touch src/bin/client.rs
```

このファイル内に、この章のコードを記述する。
それを実行する必要があるとき、最初にサーバーを分離したターミナルウィンドウで起動する必要がある。

```shell
cargo run --bin server
```

そして、クライアントを、他のターミナルで実行する。

```shell
cargo run --bin client
```

コードを記述する。

言ってみれば、2つの並行したRedisコマンドを実行する必要がある。
1つのコマンドにつき1つのタスクを生成できる。
そして、2つのコマンドは並行で発生するだろう。

最初に、以下のようなものを試す。

```rust
use mini_redis::client;

#[tokio::main]
async fn main() {
    // サーバーとの接続を確立する。
    let mut client = client::connect("127.0.0.1:6379").await.unwrap();

    // 2つのタスクを生成して、1つはキーを取得して、その他はキーを設定する。
    let t1 = tokio::spawn(async {
        let res = client.get("hello").await;
    });

    let t2 = tokio::spawn(async {
        client.set("foo", "bar".into()).await;
    })

    t1.await.unwrap();
    t2.await.unwrap();
}
```

両方のタスクはなんとかして`client`にアクセスする必要があるためコンパイルできない。
`Client`は`Copy`を実装していないため、この共有を容易にするためのコードがないとコンパイルできない。
加えて、`Client::set`は`&mut self`を受け取り、それはそれを呼ぶために排他的なアクセスを要求する
ことを意味する。
タスクごとに接続を開くことができるが、それは理想的ではない。
ロックを保持して`.await`を呼び出す必要があるため、`std::sync::Mutex`を使用することはできない。
`tokio::sync::Mutex`を使用できるが、それは1つのリクエストしか許可しない。
クライアントが[パイプライン](https://redis.io/topics/pipelining)を実装している場合、
非同期ミューテックスは、十分に活用されない結果になる。

### メッセージパッシング

その答えはメッセージパッシングを使用することである。
そのパターンは`client`のリソースを管理する献身的なタスクの生成を巻き込む。
リクエストを発行したい任意のガス区は`client`タスクにメッセージを送信する。
`client`タスクは送信者に代わってリクエストを発行して、応答が送信者に返送される。

この戦略を使用することで、1つの接続が確立される。
`client`を管理するタスクは、`get`と`set`を呼び出すために排他的なアクセスを取得できる。
加えて、チャネルはバッファとして機能する。
`client`タスクが忙しい間も、操作が`client`タスクに送信されるだろう。
一旦、`client`タスクが新しいリクエストを処理できるようになれば、それはチャネルから次のリクエスト
を引き出す。
これは良いスループットをもたらし、接続プーリングをサポートするように拡張できる。

### Tokioのチャネル基本要素

Tokioは[いくつかのチャネル](https://docs.rs/tokio/1/tokio/sync/index.html)を提供しており、
それぞれ別の目的を提供している。

* [mpsc](https://docs.rs/tokio/1/tokio/sync/mpsc/index.html): 複数の生産者、単独の消費者チャネル。多くの値が送信される。
* [oneshot](https://docs.rs/tokio/1/tokio/sync/oneshot/index.html): 単独の生産者、単独の消費者チャネル。1つの値が送信される。
* [broadcast](https://docs.rs/tokio/1/tokio/sync/broadcast/index.html): 複数の生産者、複数の消費者。多くの値が送信される。各受信者はすべての値を見る。
* [watch](https://docs.rs/tokio/1/tokio/sync/watch/index.html): 単独の生産者、複数の消費者。多くの値が送信されるが、履歴を保持しない。受信者は最も最新の値のみ見る。

1つの消費者が各メッセージを見る複数の生産者、複数の消費者チャネルが必要な場合、[async-channel](https://docs.rs/async-channel/)
クレートを使用できる。
[std::sync::mpsc](https://doc.rust-lang.org/stable/std/sync/mpsc/index.html)や
[crossbeam::channel](https://docs.rs/crossbeam/latest/crossbeam/channel/index.html)のような、
非同期Rustの外部で使用するためのチャネルがある。
これらのチャネルはスレッドをブロックすることによってメッセージを待ち受け、それは非同期コードを許可しない。

この節では、`mpsc`と`oneshot`を使用する。
メッセージパッシングチャネルのその他の型は、後の節で探求される。
この節の完全なコードは[ここ](https://github.com/tokio-rs/website/blob/master/tutorial-code/channels/src/main.rs)で見つかる。

### メッセージ型の定義

ほとんどの場合、メッセージパッシングを使用するとき、メッセージを受け取っているタスクは1つのコマンドより多く応答する。
本件の場合、タスクは`GET`と`SET`コマンドに応答する。
これを構成するために、最初に`Command`列挙型を定義して、それぞれのコマンドの種類にバリアントを含める。

```rust
use bytes::Bytes;

#[derive(Debug)]
enum Command {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: Bytes,
    }
}
```

### チャネルの作成

`main`関数内に、`mpsc`チャネルを構築する。

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // 最大32個までの容量を持つ新しいチャネルを作成する。
    // tx: 送信: Transmitter
    // rx: 受信: Receiver
    // 末尾の`x`は「省略」を示す文字という説と、e`X`changeの略という説がある。
    let (tx, mut rx) = mpsc::channel(32);

    // ...残りはここにくる。
}
```

`mpsc`チャネルはコマンドをRedisコネクションを管理するタスクに`send`するために使用される。
複数の生産者の能力は多くのタスクからメッセージを送信されることを許可する。
チャネルを作成すると送信者と受信者の2つの値が返却される。
その2つのハンドルは分離して使用される。
それらは異なるタスクにムーブされる。

そのチャネルは32個の容量で作成される。
メッセージがそれらが受け取るよりも早く送信されると、チャネルはそれらを蓄積する。
一旦、チャネル内に32個のメッセージが蓄積されると、`send(...).await`の呼び出しは、
受信者によってメッセージが取り除かれるまでスリープする。

複数のタスクから送信することは、`Sender`の`cloning`によってなされる。

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);
    let tx = tx.clone();

    tokio::spawn(async move {
        tx.send("sending from first handle").await;
    });

    tokio::spawn(async move {
        tx2.send("sending from second handle").await;
    });

    while let Some(message) = rx.recv().await {
        println!("GOT = {}", message);
    }
}
```

療法のメッセージは`Receiver`ハンドルに送信される。
`mpsc`チャネルの受信者はクローンすることができない。

すべての`Sender`がスコープ外になるか、その他の方法でドロップされた時、チャネルにそれ以上メッセージを
送信することができなくなる。
この点において、`Receiver`への`recv`呼び出しは`None`を返却して、これはすべての送信者がいなくなり、
チャネルが閉じられたことを意味する。

Redisコネクションを管理するタスクの場合、一旦、Redisコネクションが閉じられたら、チャネルが閉られて、
再度接続を使用できないことを知る。
