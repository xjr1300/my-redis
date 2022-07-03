async fn say_world() {
    println!("world");
}

#[tokio::main]
async fn main() {
    // `say_world()`を呼び出しても、`say_world()`の中身は実行されない。
    let op = say_world();

    // この`println!`が最初に実行される。
    println!("hello");

    // `op`nい対して`.await`を呼び出すことで、`say_world`の中身が実行される。
    op.await;

    // [出力結果]
    // hello
    // world
}
