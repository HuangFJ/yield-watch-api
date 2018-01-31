/*
Author: Philip Woolford <woolford.philip@gmail.com>
Repository: https://github.com/pantsman0/rust-hmac-sha1
*/
extern crate sha1;

// define hash constants
pub const SHA1_DIGEST_BYTES: usize = 20;
const SHA1_KEY_BYTES: usize = 64;

pub fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; SHA1_DIGEST_BYTES] {
    // set constants for HMAC
    let inner_pad_byte: u8 = 0x36;
    let outer_pad_byte: u8 = 0x5c;
    let key_pad_byte:   u8 = 0x00;

    // instantiate internal structures
    let mut sha1_ctx = sha1::Sha1::new();
    #[allow(unused_mut)]
    let mut auth_key: &mut [u8; SHA1_KEY_BYTES] = &mut [key_pad_byte; SHA1_KEY_BYTES];

    // if the key is longer than the hasher's block length, it should be truncated using the hasher
    if { key.len() > SHA1_KEY_BYTES } {
        // derive new authentication from provided key
        sha1_ctx.update(key);

        // assign derived authentication key
        auth_key[..SHA1_DIGEST_BYTES].copy_from_slice(&(sha1_ctx.digest().bytes()));

        // reset hash for reuse
        sha1_ctx.reset();
    } else {
        auth_key[..key.len()].copy_from_slice(key);
    }

    // generate padding arrays
    let mut inner_padding: [u8; SHA1_KEY_BYTES] = [inner_pad_byte; SHA1_KEY_BYTES];
    let mut outer_padding: [u8; SHA1_KEY_BYTES] = [outer_pad_byte; SHA1_KEY_BYTES];

    for offset in 0..auth_key.len() {
        inner_padding[offset] ^= auth_key[offset];
        outer_padding[offset] ^= auth_key[offset];
    }

    // perform inner hash
    sha1_ctx.update(&inner_padding);
    sha1_ctx.update(message);
    let inner_hash = sha1_ctx.digest().bytes();
    sha1_ctx.reset();

    // perform outer hash
    sha1_ctx.update(&outer_padding);
    sha1_ctx.update(&inner_hash);
    sha1_ctx.digest().bytes()
}