#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]

use aes_gcm::aead::heapless::Vec;
use esp_backtrace as _;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{clock::CpuClock, timer::timg::TimerGroup};
use esp_rtos as _;
use hardware_password_manager::display::{DisplayPins, init_display};
use hardware_password_manager::storage::{
    HardwareFlash, load_password_from_flash, save_password_to_flash,
};
use hardware_password_manager::usb::{Usb, Writer, enter_password, init_usb};
use littlefs2::fs::Filesystem;
use log::{error, info, warn};

esp_bootloader_esp_idf::esp_app_desc!();

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Initialise Software Interrupt
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    let delay = esp_hal::delay::Delay::new();
    delay.delay_millis(2000);

    // Setup Storage
    let mut flash_hardware = HardwareFlash::new(peripherals.FLASH);
    let mut fs_alloc: littlefs2::fs::Allocation<HardwareFlash> = littlefs2::fs::Allocation::new();

    // Try to mount the Filesystem
    let filesystem = match Filesystem::mount(&mut fs_alloc, &mut flash_hardware) {
        Ok(fs) => {
            info!("LittleFS erfolgreich gemountet!");
            fs
        }
        Err(_) => {
            warn!("Error Filesystem not found. Formating Flash...");
            Filesystem::format(&mut flash_hardware).expect("Error formating Flash!");
            Filesystem::mount(&mut fs_alloc, &mut flash_hardware)
                .expect("CRITICAL Error mounting Filesystem after formating!")
        }
    };

    /*// Initialise User Button
    let button_config = InputConfig::default().with_pull(Pull::Up);
    let button = Input::new(peripherals.GPIO0, button_config);

    // Initialise USB Keyboard
    let usb_keyboard = init_usb(peripherals.USB0, peripherals.GPIO20, peripherals.GPIO19).await;

    // Initialise TFT Display
    let display_pins = DisplayPins {
        tft_i2c_pwr_pin: peripherals.GPIO21,
        backlight_pin: peripherals.GPIO45,
        sck_pin: peripherals.GPIO36,
        miso_pin: peripherals.GPIO37,
        mosi_pin: peripherals.GPIO35,
        cs_pin: peripherals.GPIO7,
        dc_pin: peripherals.GPIO39,
        rst_pin: peripherals.GPIO40,
        spi2: peripherals.SPI2,
    };
    let _display = init_display(display_pins); */

    /*spawner.spawn(run_usb(usb_keyboard.usb).expect("Error spawning USB Task!"));
    spawner.spawn(button_task(button, usb_keyboard.writer).expect("Error spawning Button Task!"));*/

    // Save Example Password to Filesystem
    if let Err(e) = save_password_to_flash(&filesystem, "github", "EmbeddedRust") {
        error!("{:?}", e);
    } else {
        info!("Password saved successfully!");
    }

    // Load Example Password from Filesystem
    let mut ram_buffer: Vec<u8, 128> = Vec::new();

    match load_password_from_flash(&filesystem, "github", &mut ram_buffer) {
        Ok(password_bytes) => {
            // 1. Bytes in einen String-Slice konvertieren für die Ausgabe/Nutzung
            if let Ok(password_str) = core::str::from_utf8(password_bytes) {
                info!("Password: {:?}", password_str);

                // 2. HIER: Später der Befehl, das Passwort per USB-HID abzufeuern
                // send_as_usb_keyboard(password_str);
            }
        }
        Err(e) => {
            error!("{:?}", e);
        }
    }

    loop {
        core::future::pending::<()>().await;
    }
}

#[embassy_executor::task]
async fn run_usb(mut usb: Usb) {
    usb.run().await;
}

#[embassy_executor::task]
async fn button_task(mut button: Input<'static>, mut writer: Writer) {
    loop {
        button.wait_for_falling_edge().await;

        if button.is_low()
            && let Err(_e) = enter_password(&mut writer, "EmbeddedRust-200423").await
        {
            error!("Error sending Password!");
        }

        embassy_time::Timer::after_millis(50).await;
    }
}
