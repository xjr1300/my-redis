use bytes::Bytes;

#[allow(dead_code)]
#[derive(Debug)]
enum Command {
    Get { key: String },
    Set { key: String, value: Bytes },
}
