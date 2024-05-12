fn main() {
    let buf = b"012345\r\n";
    let pos = buf.windows(2).position(|pair| pair == [b'\r', b'\n']);
    assert_eq!(pos, Some(6));

    let buf = b"012345\r";
    let pos = buf.windows(2).position(|pair| pair == [b'\r', b'\n']);
    assert_eq!(pos, None);
}
