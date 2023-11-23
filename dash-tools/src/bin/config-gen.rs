use dash_node::crypto;

use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    (0..4)
        .into_iter()
        .for_each(|i| gen_keypair_file(i.to_string()));
}

fn gen_keypair_file(mut file_name: String) {
    let keypair = crypto::generate_keypair();
    let pubkey_bytes = keypair.public.to_bytes();
    let pem = crypto::keypair_to_pem(keypair);
    let pk_b64 = crypto::publickey_to_base64(pubkey_bytes);
    let mut keypair_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&file_name)
        .unwrap();
    keypair_file.write_all(pem.as_bytes()).unwrap();
    file_name += ".pub";
    let mut pubkey_file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(file_name)
        .unwrap();
    pubkey_file.write_all(pk_b64.as_bytes()).unwrap();
}
