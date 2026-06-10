use embedded_storage::ReadStorage;
use embedded_storage::nor_flash::NorFlash;
use esp_hal::peripherals::FLASH;
use esp_storage::FlashStorage;
use littlefs2::{
    consts::{U16, U4096},
    driver::Storage,
    io::{Error, Result},
};

// ESP32-S3 NOR-Flash specific configurations
// A standard sector is exactly 4096 Bytes
const BLOCK_SIZE: usize = 4096;

// 32 Blocks = 128 KB total storage space for passwords
const BLOCK_COUNT: usize = 32;

// The starting address of the 'storage' partition
// Make sure this matches your partitions.csv!
const BASE_ADDRESS: u32 = 0x120000;

/// Hardware wrapper for the ESP32 internal flash
pub struct HardwareFlash {
    flash: FlashStorage<'static>,
}

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
    type CACHE_SIZE = U4096;
    type LOOKAHEAD_SIZE = U16;

    const READ_SIZE: usize = 1;
    const WRITE_SIZE: usize = 1;
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

        let addr = BASE_ADDRESS + off as u32;
        self.flash.erase(addr, len as u32).map_err(|_| Error::IO)?;

        Ok(len)
    }
}
