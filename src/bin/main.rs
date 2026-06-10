#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]

use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::{clock::CpuClock, timer::timg::TimerGroup};
use esp_rtos as _;
use hardware_password_manager::display::{DisplayPins, init_display};
use hardware_password_manager::storage::HardwareFlash;
use hardware_password_manager::usb::{Usb, Writer, enter_password, init_usb};
use littlefs2::fs::Filesystem;
use littlefs2::io::{Read, Write};
use littlefs2::path::PathBuf;
use panic_rtt_target as _;

esp_bootloader_esp_idf::esp_app_desc!();

const PASSWORD: &str = "EmbeddedRust-2004";

#[allow(clippy::large_stack_frames)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Setup Storage
    let mut flash_hardware = HardwareFlash::new(peripherals.FLASH);
    littlefs2::object_alloc!(fs_alloc, Filesystem);

    // Try to mount the Filesystem
    let filesystem = match Filesystem::mount(fs_alloc, &mut flash_hardware) {
        Ok(fs) => {
            defmt::info!("LittleFS erfolgreich gemountet!");
            fs
        }
        Err(_) => {
            defmt::warn!("Error Filesystem not found. Formating Flash...");
            Filesystem::format(&mut flash_hardware).expect("Error formating Flash!");
            Filesystem::mount(fs_alloc, &mut flash_hardware)
                .expect("CRITICAL Error mounting Filesystem after formating!")
        }
    };

    // Initialise Software Interrupt
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let software_interrupt = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, software_interrupt.software_interrupt0);

    // Initialise User Button
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
    let _display = init_display(display_pins);

    spawner.spawn(run_usb(usb_keyboard.usb).expect("Error spawning USB Task!"));
    spawner.spawn(button_task(button, usb_keyboard.writer).expect("Error spawning Button Task!"));

    loop {
        embassy_time::Timer::after_secs(1).await;
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
            && let Err(_e) = enter_password(&mut writer, PASSWORD).await
        {
            defmt::error!("Error sending Password!");
        }

        embassy_time::Timer::after_millis(50).await;
    }
}
