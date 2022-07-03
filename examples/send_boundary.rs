//! `tokio::spawn`によって生成されたタスクは`Send`を実装する必要がある。
//! `Send`は、タスクが`.await`で一時停止されている間に、Tokioランタイムに
//! スレッド間をタスクが移動することを許可する。
//!
//! `.await`呼び出しで保持されたすべてのデータが`Send`のとき、タスクは`Send`
//! である。
//! これは少し微秒である。
//! `.await`が呼び出されたとき、タスクはスケジューラーに戻る。
//! 次にタスクが実行されると、タスクは最後に戻された地点から再開する。
//! この作業をするために、`.await`の後で使用される全ての状態は、タスクによって
//! 保存されなくてはならない。
//! この状態が`Send`である場合、例えばスレッド間でムーブされるなど、そのタスク
//! 自身はスレッド間でムーブされることができる。
//! 逆に、状態が`Send`でない場合、タスクではない。

use std::rc::Rc;
use tokio::task::yield_now;

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        // 以下のスコープは、`.await`の前に`rc`をドロップすることを強制する。
        {
            let rc = Rc::new("hello");
            println!("{}", rc);
        }

        // `rc`はもはや使用されない。
        // スケジューラーによってタスクが生成されたとき、`rc`は存続していない。
        yield_now().await;
    });
}
// async fn main() {
//     tokio::spawn(async {
//         let rc = Rc::new("hello");
//         // Arcはスレッド間でムーブできるため、`rc`の型をArcにした場合、
//         // コンパイルできる。
//         // use std::sync::Arc;
//         // let rc = Arc::new("hello");
//
//         // `rc`が`.await`の後で使用される。
//         // `.await`の後で使用されるものは、タスクの状態として存続される必要がある。
//         yield_now().await;
//
//         println!("{}", rc);
//     });
// }
/*
error: future cannot be sent between threads safely
   --> examples/send_boundary.rs:34:5
    |
34  |     tokio::spawn(async {
    |     ^^^^^^^^^^^^ future created by async block is not `Send`
    |
    = help: within `impl Future<Output = ()>`, the trait `Send` is not implemented for `Rc<&str>`
note: future is not `Send` as this value is used across an await
   --> examples/send_boundary.rs:37:20
    |
35  |         let rc = Rc::new("hello");
    |             -- has type `Rc<&str>` which is not `Send`
36  |
37  |         yield_now().await;
    |                    ^^^^^^ await occurs here, with `rc` maybe used later
...
40  |     });
    |     - `rc` is later dropped here
note: required by a bound in `tokio::spawn`
   --> /Users/xjr1300/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.19.2/src/task/spawn.rs:127:21
    |
127 |         T: Future + Send + 'static,
    |                     ^^^^ required by this bound in `tokio::spawn`
*/
