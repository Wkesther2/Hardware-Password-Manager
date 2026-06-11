use crate::aes::{decrypt_password, encrypt_password};
use aes_gcm::aead::heapless::Vec;
use embedded_storage::ReadStorage;
use embedded_storage::nor_flash::NorFlash;
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;
use getrandom::register_custom_getrandom;
use littlefs2::{
    consts::{U16, U32, U64, U512},
    driver::Storage,
    fs::{File, Filesystem},
    io::{Error, Result},
    path::Path,
};
use log::{error, info, warn};

const BLOCK_SIZE: usize = 4096;

const BLOCK_COUNT: usize = 64;

const BASE_ADDRESS: u32 = 0x3C0000;

/// Hardware wrapper for the ESP32 internal flash
pub struct HardwareFlash {
    flash: FlashStorage<'static>,
}

register_custom_getrandom!(custom_getrandom);

impl HardwareFlash {
    pub fn new(flash: FLASH<'static>) -> Self {
        Self {
            flash: FlashStorage::new(flash),
        }
    }
}

/// Implement the littlefs2 Storage Trait for our raw hardware flash
impl Storage for HardwareFlash {
    // We use typenum constants provided by littlefs2 for the buffer sizes
    type CACHE_SIZE = U512;
    type LOOKAHEAD_SIZE = U32;

    const READ_SIZE: usize = 32;
    const WRITE_SIZE: usize = 32;
    const BLOCK_SIZE: usize = BLOCK_SIZE;
    const BLOCK_COUNT: usize = BLOCK_COUNT;

    fn read(&mut self, off: usize, buf: &mut [u8]) -> Result<usize> {
        // Prevent reading out of bounds
        if off + buf.len() > BLOCK_SIZE * BLOCK_COUNT {
            return Err(Error::IO);
        }

        let addr = BASE_ADDRESS + off as u32;
        self.flash.read(addr, buf).map_err(|_| Error::IO)?;

        Ok(buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> Result<usize> {
        // Prevent writing out of bounds
        if off + data.len() > BLOCK_SIZE * BLOCK_COUNT {
            return Err(Error::IO);
        }

        let addr = BASE_ADDRESS + off as u32;
        embedded_storage::nor_flash::NorFlash::write(&mut self.flash, addr, data)
            .map_err(|_| Error::IO)?;

        Ok(data.len())
    }

    fn erase(&mut self, off: usize, len: usize) -> Result<usize> {
        // Prevent erasing out of bounds
        if off + len > BLOCK_SIZE * BLOCK_COUNT {
            return Err(Error::IO);
        }

        let from = BASE_ADDRESS + off as u32;
        let to = from + len as u32; // <-- HIER: Startadresse + Länge = Endadresse!

        self.flash.erase(from, to).map_err(|_| Error::IO)?;

        Ok(len)
    }
}

pub fn save_password_to_flash(
    fs: &Filesystem<HardwareFlash>,
    service_name: &str,
    password: &str,
) -> Result<(), &'static str> {
    let encrypted_packet = encrypt_password(password).map_err(|_| "Error encrypting!")?;

    let mut path_bytes: heapless::Vec<u8, 64> = heapless::Vec::new();
    path_bytes
        .extend_from_slice(service_name.as_bytes())
        .unwrap();
    path_bytes.extend_from_slice(b".bin\0").unwrap();

    let path = Path::from_bytes_with_nul(&path_bytes).map_err(|_| "Invalid Path!")?;
    let mut alloc = littlefs2::fs::FileAllocation::new();

    // HIER ÄNDERN: Wir fangen den echten LittleFS-Fehler ab und loggen ihn!
    let file = unsafe { File::create(fs, &mut alloc, path) }.map_err(|e| {
        error!("LittleFS File::create fehlgeschlagen mit Fehler: {:?}", e);
        "Error creating File!"
    })?;

    file.write(encrypted_packet.as_slice())
        .map_err(|_| "Error writing!")?;

    file.sync().map_err(|_| "Error synchronizing File!")?;

    Ok(())
}

pub fn load_password_from_flash<'a>(
    fs: &Filesystem<HardwareFlash>,
    service_name: &str,
    buffer: &'a mut Vec<u8, 128>,
) -> Result<&'a [u8], &'static str> {
    // 1. Create Path to File
    let mut path_str: heapless::String<64> = heapless::String::new();
    path_str.push_str(service_name).unwrap();
    path_str.push_str(".bin\0").unwrap();

    let path = Path::from_str_with_nul(path_str.as_str()).map_err(|_| "Invalid File Path!")?;

    let mut alloc = littlefs2::fs::FileAllocation::new();

    // 2. Open File in Read Only Mode
    let file = unsafe { File::open(fs, &mut alloc, path) }.map_err(|_| "Error opening File!")?;

    // 3. Prepare Buffer by resetting it's Content and setting the Capacity tempoarily to Max
    buffer.clear();
    unsafe {
        buffer.set_len(buffer.capacity());
    }

    // 4. Read Content of the File to Buffer
    let bytes_read = file
        .read(&mut buffer[..])
        .map_err(|_| "Error reading from Flash!")?;

    // Set Capacity of Buffer to the required Length
    unsafe {
        buffer.set_len(bytes_read);
    }

    // Saftey-Check: Length of Read File Content has to be greater than 28 Bytes
    if buffer.len() < 28 {
        return Err("Error invalid File!");
    }

    // 5. Decrypt Password
    let plaintext_slice = decrypt_password(buffer).map_err(|_| "Error decrypting Password!")?;

    Ok(plaintext_slice)
}

fn custom_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for byte in buf.iter_mut() {
        // Direktes Auslesen des ESP32-S3 Hardware-TRNG Registers
        let random_word = unsafe { *(0x60035078 as *const u32) };
        *byte = (random_word & 0xFF) as u8;
    }
    Ok(())
}
