use aes_gcm::{
    Aes256Gcm, Nonce, Tag,
    aead::{AeadCore, AeadInPlace, KeyInit, OsRng, heapless::Vec},
};

const TEMPORARY_TEST_KEY: [u8; 32] = [100; 32]; // = dddddddddddddddddddddddddddddddd

pub fn encrypt_password(password: &str) -> Result<Vec<u8, 128>, aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(&TEMPORARY_TEST_KEY).map_err(|_| aes_gcm::Error)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let mut packet: Vec<u8, 128> = Vec::new();

    packet.extend_from_slice(&nonce).unwrap();

    packet.extend_from_slice(password.as_bytes()).unwrap();

    let tag = cipher.encrypt_in_place_detached(&nonce, b"", &mut packet[12..])?;

    packet.extend_from_slice(&tag).unwrap();

    Ok(packet)
}

pub fn decrypt_password(packet: &mut Vec<u8, 128>) -> Result<&[u8], aes_gcm::Error> {
    let cipher = Aes256Gcm::new_from_slice(&TEMPORARY_TEST_KEY).map_err(|_| aes_gcm::Error)?;

    // 1. Saftey-Check: Packet has to be 28 Bytes or longer
    if packet.len() < 28 {
        return Err(aes_gcm::Error);
    }

    // 2. Extract Nonce from the first 12 Bytes
    let mut nonce_array = [0u8; 12];
    nonce_array.copy_from_slice(&packet[..12]);
    let nonce = Nonce::from_slice(&nonce_array);

    // 3. Extract Krypto Tag from the last 16 Bytes
    let tag_start = packet.len() - 16;
    let mut tag_array = [0u8; 16];
    tag_array.copy_from_slice(&packet[tag_start..]);
    let tag = Tag::from_slice(&tag_array);

    // 4. Create Slice of the actual encrypted Password
    let ciphertext_part: &mut [u8] = &mut packet[12..tag_start];

    // 5. Decrypt Password
    cipher.decrypt_in_place_detached(nonce, b"", ciphertext_part, tag)?;

    // 6. Return decrypted Password
    Ok(ciphertext_part)
}
