# ESP32-S3 Bare-Metal Hardware Password Vault

A secure, autonomous hardware password manager written in pure, bare-metal Rust (`no_std`) for the Espressif ESP32-S3 microcontroller. This system leverages custom flash storage abstractions and the fault-tolerant, wear-leveled `littlefs2` filesystem to securely preserve credentials without an underlying Operating System.

## Key Features & Architecture

- **True Bare-Metal Rust (`no_std`):** Zero dependency on a standard library, maximizing execution speed, minimizing binary footprint, and eliminating typical OS-level security vulnerabilities.
- **Power-Loss Resilient Storage:** Integrated with `littlefs2` utilizing a specialized Copy-on-Write (CoW) mechanism to guarantee that the database is never corrupted, even if disconnected mid-write.
- **Embedded Wear Leveling:** Dynamically distributes erase and write cycles across 32 sequential 4KB NOR Flash sectors (128 KB total storage allocation) to protect the underlying physical silicon from premature wear.
- **Strict RAM Optimization:** Operates on static allocations without a heap allocator. Uses a 16-bit Lookahead Buffer (2 bytes RAM) for rapid block state evaluation and a 4KB Sector Buffer (matching the physical NOR flash sector size) to maintain ideal write-amplification boundaries.

---

## Hardware & Tech Stack

- **Microcontroller:** Espressif ESP32-S3 (Xtensa LX7 Dual-Core)
- **Language & Paradigm:** Rust stable toolchain with `embedded-hal` v1.0
- **Driver Infrastructure:** `esp-hal` (Hardware Abstraction Layer) & `esp-storage`
- **Filesystem Protocol:** `littlefs2` (Rust bindings for ARM's littlefs)
- **Peripheral Targets:** SPI/I2C driven miniature display, physical navigation buttons, and USB-OTG acting as a native HID Keyboard.

---

## Storage Architecture Deep-Dive

The repository includes a custom implementation of the `littlefs2::driver::Storage` trait, interfacing directly with the ESP32-S3's internal NOR flash memory via `esp-storage`.

### Physical Configurations:
- **`BLOCK_SIZE` (4096 Bytes):** The hardware boundaries dictate that bits can be written from `1` to `0` granularly, but pulling a `0` back to a `1` requires a full 4KB physical sector erasure.
- **`READ_SIZE` / `WRITE_SIZE` (1 Byte):** Pinpoint precision reading and writing to prevent unnecessary zero-padding and reduce cross-bit degradation.
- **`CACHE_SIZE` (4096 Bytes):** Exactly matches the sector block size to prevent partial-sector thrashing and costly write amplification.

---

## Project Status & Roadmap

- [x] Bare-metal project initialization via `esp-generate`
- [x] Custom USB HID Keyboard Driver for sending the password to the computer 
- [x] Custom `Storage` trait driver architecture implementation for internal NOR flash
- [x] Fail-safe `mount()` / auto-`format()` sequence loop implementation
- [ ] Cryptographic layer configuration (AES-GCM / ChaCha20-Poly1305 credential encryption)
- [ ] I2C/SPI Display UI Driver configuration & layout trees
- [ ] Password Menu System
- [ ] CLI application for saving new and changing existent passwords
---

## Building and Flashing

### Prerequisites
Ensure your Rust environment is optimized for Espressif targets. You will need the Xtensa toolchain or the standard toolchain depending on your compilation profile (this project targets standard stable rust via the `direct-boot` methodology where applicable):

```bash
cargo install espup
espup install
cargo install cargo-espflash
