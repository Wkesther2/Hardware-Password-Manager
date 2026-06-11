use core::sync::atomic::{AtomicBool, Ordering};
use embassy_usb::class::hid::{HidBootProtocol, HidReaderWriter, HidSubclass, HidWriter, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Handler, UsbDevice};
use esp_hal::otg_fs::{
    self,
    asynch::{Config, Driver},
};
use esp_hal::peripherals::{GPIO19, GPIO20, USB0};
use log::{error, info, warn};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

pub type Usb = UsbDevice<'static, Driver<'static>>;
pub type Writer = HidWriter<'static, Driver<'static>, 8>;

// Custom Error Type
#[derive(Debug, Clone, Copy)]
pub enum UsbError {
    EndpointError(EndpointError),
    InvalidSymbol(&'static str),
}

// USB Keyboard Struct
pub struct UsbKeyboard {
    pub usb: Usb,
    pub writer: Writer,
}

// USB initialization
pub async fn init_usb(
    usb0: USB0<'static>,
    dp: GPIO20<'static>,
    dm: GPIO19<'static>,
) -> UsbKeyboard {
    // esp-hal USB Config & Driver
    let mut config = Config::default();
    config.vbus_detection = false;

    static EP_OUT_BUFFER: StaticCell<[u8; 256]> = StaticCell::new();
    let ep_out_buffer = EP_OUT_BUFFER.init([0; 256]);

    let usb_otg = otg_fs::Usb::new(usb0, dp, dm);
    let driver = Driver::new(usb_otg, ep_out_buffer, config);

    // embassy-usb Config
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Medic&co.");
    config.product = Some("Hardware-Password-Manager");
    config.serial_number = Some("69696969");
    config.max_power = 100;
    config.max_packet_size_0 = 64;
    config.composite_with_iads = false;
    config.device_class = 0;
    config.device_sub_class = 0;
    config.device_protocol = 0;

    // embassy-usb DeviceBuilder
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);

    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);

    static MSOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    let msos_descriptor = MSOS_DESCRIPTOR.init([0; 256]);

    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    let control_buf = CONTROL_BUF.init([0; 64]);

    static DEVICE_HANDLER: StaticCell<MyDeviceHandler> = StaticCell::new();
    let device_handler = DEVICE_HANDLER.init(MyDeviceHandler::new());

    static STATE: StaticCell<State> = StaticCell::new();
    let state = STATE.init(State::new());

    let mut builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        msos_descriptor,
        control_buf,
    );

    builder.handler(device_handler);

    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 10,
        max_packet_size: 8,
        hid_subclass: HidSubclass::Boot,
        hid_boot_protocol: HidBootProtocol::Keyboard,
    };

    let (_, writer) = HidReaderWriter::<_, 1, 8>::new(&mut builder, state, config).split();

    // Build the builder.
    let usb = builder.build();

    UsbKeyboard { usb, writer }
}
struct MyDeviceHandler {
    configured: AtomicBool,
}

// Send Text via USB
pub async fn send_text_usb(writer: &mut Writer, text: &str) -> Result<(), UsbError> {
    let empty_keyboard_report = KeyboardReport::default();

    for c in text.chars() {
        let keyboard_report = KEYMAP
            .get(&c)
            .ok_or(UsbError::InvalidSymbol(stringify!(c)))?;

        writer
            .write_serialize(keyboard_report)
            .await
            .map_err(UsbError::EndpointError)?;
        writer
            .write_serialize(&empty_keyboard_report)
            .await
            .map_err(UsbError::EndpointError)?;
    }

    Ok(())
}

pub async fn enter_password(writer: &mut Writer, text: &str) -> Result<(), UsbError> {
    send_text_usb(writer, text).await?;
    send_text_usb(writer, "\n").await?;

    Ok(())
}

impl MyDeviceHandler {
    fn new() -> Self {
        MyDeviceHandler {
            configured: AtomicBool::new(false),
        }
    }
}

impl Handler for MyDeviceHandler {
    fn enabled(&mut self, enabled: bool) {
        self.configured.store(false, Ordering::Relaxed);
        if enabled {
            info!("Device enabled");
        } else {
            info!("Device disabled");
        }
    }

    fn reset(&mut self) {
        self.configured.store(false, Ordering::Relaxed);
        info!("Bus reset, the Vbus current limit is 100mA");
    }

    fn addressed(&mut self, addr: u8) {
        self.configured.store(false, Ordering::Relaxed);
        info!("USB address set to: {}", addr);
    }

    fn configured(&mut self, configured: bool) {
        self.configured.store(configured, Ordering::Relaxed);
        if configured {
            info!(
                "Device configured, it may now draw up to the configured current limit from Vbus."
            )
        } else {
            info!("Device is no longer configured, the Vbus current limit is 100mA.");
        }
    }
}

// Macros for keyboard modifiers
macro_rules! normal {
    ($code:expr) => {
        KeyboardReport {
            keycodes: [$code, 0, 0, 0, 0, 0],
            modifier: 0,
            leds: 0,
            reserved: 0,
        }
    };
}
macro_rules! shift {
    ($code:expr) => {
        KeyboardReport {
            keycodes: [$code, 0, 0, 0, 0, 0],
            modifier: 0x02,
            leds: 0,
            reserved: 0,
        }
    };
}
macro_rules! altgr {
    ($code:expr) => {
        KeyboardReport {
            keycodes: [$code, 0, 0, 0, 0, 0],
            modifier: 0x40,
            leds: 0,
            reserved: 0,
        }
    };
}

// Keyboardkeys-Map
static KEYMAP: phf::Map<char, KeyboardReport> = phf::phf_map! {
    // --- Lowercase (Z and Y swapped) ---
    'a' => normal!(0x04), 'b' => normal!(0x05), 'c' => normal!(0x06), 'd' => normal!(0x07),
    'e' => normal!(0x08), 'f' => normal!(0x09), 'g' => normal!(0x0A), 'h' => normal!(0x0B),
    'i' => normal!(0x0C), 'j' => normal!(0x0D), 'k' => normal!(0x0E), 'l' => normal!(0x0F),
    'm' => normal!(0x10), 'n' => normal!(0x11), 'o' => normal!(0x12), 'p' => normal!(0x13),
    'q' => normal!(0x14), 'r' => normal!(0x15), 's' => normal!(0x16), 't' => normal!(0x17),
    'u' => normal!(0x18), 'v' => normal!(0x19), 'w' => normal!(0x1A), 'x' => normal!(0x1B),
    'y' => normal!(0x1D), 'z' => normal!(0x1C),

    // --- Uppercase ---
    'A' => shift!(0x04), 'B' => shift!(0x05), 'C' => shift!(0x06), 'D' => shift!(0x07),
    'E' => shift!(0x08), 'F' => shift!(0x09), 'G' => shift!(0x0A), 'H' => shift!(0x0B),
    'I' => shift!(0x0C), 'J' => shift!(0x0D), 'K' => shift!(0x0E), 'L' => shift!(0x0F),
    'M' => shift!(0x10), 'N' => shift!(0x11), 'O' => shift!(0x12), 'P' => shift!(0x13),
    'Q' => shift!(0x14), 'R' => shift!(0x15), 'S' => shift!(0x16), 'T' => shift!(0x17),
    'U' => shift!(0x18), 'V' => shift!(0x19), 'W' => shift!(0x1A), 'X' => shift!(0x1B),
    'Y' => shift!(0x1D), 'Z' => shift!(0x1C),

    // --- Numbers ---
    '1' => normal!(0x1E), '2' => normal!(0x1F), '3' => normal!(0x20), '4' => normal!(0x21),
    '5' => normal!(0x22), '6' => normal!(0x23), '7' => normal!(0x24), '8' => normal!(0x25),
    '9' => normal!(0x26), '0' => normal!(0x27),

    // --- German Umlauts & special chars ---
    'ä' => normal!(0x34), 'Ä' => shift!(0x34),
    'ö' => normal!(0x33), 'Ö' => shift!(0x33),
    'ü' => normal!(0x2F), 'Ü' => shift!(0x2F),
    'ß' => normal!(0x2D),

    ' '  => normal!(0x2C),
    '\n' => normal!(0x28),
    '\t' => normal!(0x2B),

    // --- Shift numbers ---
    '!' => shift!(0x1E), '"' => shift!(0x1F), '§' => shift!(0x20), '$' => shift!(0x21),
    '%' => shift!(0x22), '&' => shift!(0x23), '/' => shift!(0x24), '(' => shift!(0x25),
    ')' => shift!(0x26), '=' => shift!(0x27), '?' => shift!(0x2D),

    // --- Formatting ---
    '-' => normal!(0x38), '_' => shift!(0x38), '.' => normal!(0x37), ',' => normal!(0x36),
    ':' => shift!(0x37), ';' => shift!(0x36), '+' => normal!(0x30), '*' => shift!(0x30),
    '#' => normal!(0x31), '\'' => shift!(0x31),

    // --- AltGr Characters ---
    '\\' => altgr!(0x2D), '@'  => altgr!(0x11), '€'  => altgr!(0x08), '~'  => altgr!(0x30),
    '{'  => altgr!(0x24), '['  => altgr!(0x25), ']'  => altgr!(0x26), '}'  => altgr!(0x27),
    '|'  => altgr!(0x64), '<'  => normal!(0x64), '>' => shift!(0x64),
};
