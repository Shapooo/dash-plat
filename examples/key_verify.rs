use dash_plat::crypto;

use std::fs::File;
use std::io::Read;

fn main() {
    let mut pub_file = File::open("./0.pub").unwrap();
    let mut sec_file = File::open("./0").unwrap();
    let mut pub_str = String::new();
    let mut sec_str = String::new();
    pub_file.read_to_string(&mut pub_str).unwrap();
    sec_file.read_to_string(&mut sec_str).unwrap();
    println!("{}\n{}", pub_str, sec_str);
    let pubkeybytes = crypto::publickey_from_base64(&pub_str).unwrap();
    let keypair = crypto::keypair_from_pem(&sec_str).unwrap();
    assert_eq!(keypair.public.to_bytes(), pubkeybytes);
}
