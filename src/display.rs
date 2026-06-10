use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_hal::Blocking;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig, Pull};
use esp_hal::peripherals::{GPIO7, GPIO21, GPIO35, GPIO36, GPIO37, GPIO39, GPIO40, GPIO45, SPI2};
use esp_hal::spi::master::{Config, Spi};
use esp_hal::time::Rate;
use mipidsi::Builder;
use mipidsi::interface::SpiInterface;
use mipidsi::models::ST7789;
use mipidsi::options::ColorInversion;
use static_cell::StaticCell;

type Display<'a> = mipidsi::Display<
    SpiInterface<'a, ExclusiveDevice<Spi<'a, Blocking>, Output<'a>, NoDelay>, Output<'a>>,
    ST7789,
    Output<'a>,
>;

pub struct DisplayPins<'a> {
    pub tft_i2c_pwr_pin: GPIO21<'a>,
    pub backlight_pin: GPIO45<'a>,
    pub sck_pin: GPIO36<'a>,
    pub miso_pin: GPIO37<'a>,
    pub mosi_pin: GPIO35<'a>,
    pub cs_pin: GPIO7<'a>,
    pub dc_pin: GPIO39<'a>,
    pub rst_pin: GPIO40<'a>,
    pub spi2: SPI2<'a>,
}

pub fn init_display(display_pins: DisplayPins<'static>) -> Display<'static> {
    // Initialise TFT Display
    // Turn on Display
    let _tft_i2c_power = Output::new(
        display_pins.tft_i2c_pwr_pin,
        Level::High,
        OutputConfig::default().with_pull(Pull::Up),
    );

    // Turn on Backlight
    let _backlight = Output::new(
        display_pins.backlight_pin,
        Level::High,
        OutputConfig::default().with_pull(Pull::Up),
    );

    // Initialise Control Pins
    let dc = Output::new(display_pins.dc_pin, Level::Low, OutputConfig::default());
    let mut rst = Output::new(display_pins.rst_pin, Level::Low, OutputConfig::default());
    rst.set_high();

    let spi_config = Config::default().with_frequency(Rate::from_mhz(40));
    let spi_bus = Spi::new(display_pins.spi2, spi_config)
        .expect("Error initializing SPI Bus!")
        .with_sck(display_pins.sck_pin)
        .with_miso(display_pins.miso_pin)
        .with_mosi(display_pins.mosi_pin);
    let cs_output = Output::new(display_pins.cs_pin, Level::High, OutputConfig::default());
    let spi_device =
        ExclusiveDevice::new_no_delay(spi_bus, cs_output).expect("Error creating SPI Device!");

    static BUFFER: StaticCell<[u8; 512]> = StaticCell::new();
    let buffer = BUFFER.init([0; 512]);

    // Define Display Interface
    let di = SpiInterface::new(spi_device, dc, buffer);

    // Create Delay Object
    let mut delay = Delay::new();

    // Define the Display
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .display_size(135, 240)
        .display_offset(52, 40)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .expect("Error creating Display!");

    // Reset Display to Black
    display
        .clear(Rgb565::BLACK)
        .expect("Error clearing Display!");

    display
}
