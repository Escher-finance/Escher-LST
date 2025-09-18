use std::{
    env::args,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use hex::FromHex;
use rand::{Rng, rngs::ThreadRng};
use rayon::prelude::*;
use tiny_keccak::Hasher;

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut keccak = tiny_keccak::Keccak::v256();
    keccak.update(data);
    let mut output = [0u8; 32];
    keccak.finalize(&mut output);
    output
}

fn create2_address(deployer: &[u8; 20], salt: &[u8; 32], init_code_hash: &[u8; 32]) -> [u8; 20] {
    let mut buf = Vec::with_capacity(1 + 20 + 32 + 32);
    buf.push(0xff);
    buf.extend_from_slice(deployer);
    buf.extend_from_slice(salt);
    buf.extend_from_slice(init_code_hash);

    let hash = keccak256(&buf);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);
    address
}

fn inc(x: &mut [u8; 32]) {
    for i in (0..x.len()).rev() {
        if x[i] == 255 {
            x[i] = 0;
        } else {
            x[i] += 1;
            break;
        }
    }
}

fn random_salt(rng: &mut ThreadRng) -> [u8; 32] {
    let mut s = [0; 32];
    rng.fill(&mut s);
    s
}

fn nibbles_match(address: &[u8], hex_prefix: &str) -> bool {
    for (i, c) in hex_prefix.chars().enumerate() {
        let nibble = match c {
            '0'..='9' => c as u8 - b'0',
            'a'..='f' => c as u8 - b'a' + 10,
            _ => return false,
        };
        let addr_nibble = if i % 2 == 0 {
            address[i / 2] >> 4
        } else {
            address[i / 2] & 0x0f
        };
        if nibble != addr_nibble {
            return false;
        }
    }
    true
}

fn find_matching_salt(target: &str, deployer: &[u8; 20], init_code_hash: &[u8; 32]) -> [u8; 32] {
    let found = Arc::new(AtomicBool::new(false));

    let n = num_cpus::get();
    (0..n)
        .into_par_iter()
        .map_init(
            || {
                let mut rng = rand::rng();
                random_salt(&mut rng)
            },
            |salt, _| {
                while !found.load(Ordering::Relaxed) {
                    let addr = create2_address(deployer, &salt, init_code_hash);
                    if nibbles_match(&addr, target) {
                        println!("MATCH > {}", hex::encode(addr));
                        found.store(true, Ordering::Relaxed);
                        return Some(*salt);
                    }
                    inc(salt);
                }
                None
            },
        )
        .find_any(|res| res.is_some())
        .flatten()
        .unwrap()
        .clone()
}

fn main() {
    let target = args().nth(1).expect("missing target arg e.g. c0ffee");

    let deployer = <[u8; 20]>::from_hex("226143977e08FEA768e5f11f37DCE22f9dF8be33").unwrap();
    let init_code_hash =
        <[u8; 32]>::from_hex("3a4f1278c55dc1fda49512cbc0b3ef978cf36dbf4017764cdb5a8d919687cd52")
            .unwrap();
    let start = Instant::now();
    let salt = find_matching_salt(&target, &deployer, &init_code_hash);

    println!("SALT 0x{}", hex::encode(salt));
    println!(
        "Took {}s",
        Instant::now().duration_since(start).as_secs_f32()
    );
}
