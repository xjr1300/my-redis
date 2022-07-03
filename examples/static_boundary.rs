//! Tokioランタイムでタスクを生成したとき、そのタスクの型のライフタイムは
//! `'static`でなくてはならない。
//! これは、生成されたタスクが、外部のタスクに所有されるデータへの参照を
//! 含めてはならないことを意味する。
//!
//! よくある概念の認識間違いに、「*'static*が常に`永遠に生きる`を意味する」があり、
//! それは当てはまらない。
//! 値が*`static*であるとはメモリリークを持つことを意味しない。
//! 詳細は[Common Rust Lifetime Misconceptions](https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md#2-if-t-static-then-t-must-be-valid-for-the-entire-program)で読むことができる。

//! 以下のコードをはコンパイルできない。
//! これが発生した理由は、デフォルトで変数はasyncブロックにムーブされないためである。
//! `v`ベクトルは`main`関数によって所有されたままである。
//! `println!`行は`v`を借用する。
//! Rustコンパイラはこれを説明して、訂正を提案している!
//! 行7(ここでは行26)を`task::spawn(async move {`に変更することは、コンパイラに`v`を生成された
//! タスクにムーブすることを命令する。
//! 現在、タスクはタスクの全てのデータを所有するので、'staticになる。

use tokio::task;

#[tokio::main]
async fn main() {
    let v = vec![1, 2, 3];

    // task::spawn(async {
    //     println!("vecはここ: {:?}", v);
    // });
    let _ = task::spawn(async move {
        println!("vecはここ: {:?}", v);
    })
    .await;
}

/*
error[E0373]: async block may outlive the current function, but it borrows `v`, which is owned by the current function
  --> examples/static_bound.rs:19:23
   |
19 |       task::spawn(async {
   |  _______________________^
20 | |         println!("vecはここ: {:?}", v);
   | |                                     - `v` is borrowed here
21 | |     });
   | |_____^ may outlive borrowed value `v`
   |
   = note: async blocks are not executed immediately and must either take a reference or ownership of outside variables they use
help: to force the async block to take ownership of `v` (and any other referenced variables), use the `move` keyword
   |
19 |     task::spawn(async move {
   |                       ++++

For more information about this error, try `rustc --explain E0373`.
error: could not compile `my-redis` due to previous error
*/
