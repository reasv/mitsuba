use base64::decode;
use base32::{Alphabet, encode};
use unicode_truncate::UnicodeTruncateStr;

fn bad_hash(s: String) -> i64 {
    let mut msg = 0i64;
    let j = s.len();
    for i in 0..j {
        msg = ((msg << 5) - msg) + (s.bytes().nth(i).unwrap() as i64);
    }
    msg
}
pub fn string_to_idcolor(s: String) -> String {
    let hash = bad_hash(s);
    let r = (hash >> 24) & 0xFF;
    let g = (hash >> 16) & 0xFF;
    let b = (hash >> 8) & 0xFF;
    let c = ((r as f64 * 0.299) + (g as f64 * 0.587) + (b as f64* 0.114)) > 125f64;
    let text = match c {
        true => "black",
        false => "white"
    };
    format!("background-color: rgb({}, {}, {}); color: {};",r,g,b,text)
}
pub fn shorten_string(maxlen: usize, s: String) -> String {
    if s.len() > maxlen {
        let (ss, w) = s.unicode_truncate(maxlen);
        ss.to_string() + &"(...)".to_string()
    } else {
        s
    }
}
pub fn get_board_page_api_url(board: &String) -> String {
    format!("https://a.4cdn.org/{}/threads.json", board)
}
pub fn get_thread_api_url(board: &String, tid: &String) -> String {
    format!("https://a.4cdn.org/{}/thread/{}.json", board, tid)
}
pub fn base64_to_32(b64: String) -> anyhow::Result<String> {
    let binary = decode(b64)?;
    let s = encode(Alphabet::RFC4648{padding: false}, binary.as_slice());
    Ok(s)
}