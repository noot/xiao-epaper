#![no_std]
#![no_main]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Runner, StackResources};
use embassy_time::{Duration, Timer};
use embedded_io_async::Read as _;
use esp_alloc as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::rng::Rng;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController, sta::StationConfig};
use reqwless::client::HttpClient;
use reqwless::request::Method;
use static_cell::StaticCell;

use xiao_epaper::uc8179::{FB_SIZE, Uc8179};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("panic: {}", info);
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const SERVER_URL: &str = env!("SERVER_URL");

macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: StaticCell<$t> = StaticCell::new();
        STATIC_CELL.uninit().write($val)
    }};
}

static mut FRAMEBUFFER: [u8; FB_SIZE] = [0u8; FB_SIZE];

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
    esp_alloc::heap_allocator!(size: 36 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

    // set up display
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0),
    )
    .expect("static spi config is valid")
    .with_sck(peripherals.GPIO8)
    .with_mosi(peripherals.GPIO10);

    let cs = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());
    let busy = Input::new(
        peripherals.GPIO4,
        InputConfig::default().with_pull(Pull::None),
    );
    let delay = Delay::new();

    let fb_ptr: *mut [u8; FB_SIZE] = &raw mut FRAMEBUFFER;
    let fb: &'static mut [u8; FB_SIZE] = unsafe { &mut *fb_ptr };

    let mut display = Uc8179::new(spi, cs, dc, rst, busy, delay, fb);

    println!("xiao-epaper: starting init");
    match display.init() {
        Ok(()) => println!("xiao-epaper: init ok"),
        Err(e) => println!("xiao-epaper: init failed: {:?}", e),
    }

    // set up wifi
    println!("wifi: connecting to {}", SSID);
    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into()),
    );

    let wifi_interface = Interface::station();
    let controller = WifiController::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("wifi controller config is valid");

    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let rng = Rng::new();
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner.spawn(connection(controller).expect("connection task has a free slot"));
    spawner.spawn(net_task(runner).expect("net task has a free slot"));

    println!("wifi: waiting for dhcp...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        println!("wifi: got ip {}", config.address);
    }

    let tcp_client = TcpClient::new(
        stack,
        mk_static!(
            TcpClientState<1, 1500, 1500>,
            TcpClientState::<1, 1500, 1500>::new()
        ),
    );
    let dns_client = DnsSocket::new(stack);

    let mut current_hash: u64 = 0;

    loop {
        println!("fetch: requesting framebuffer from {}", SERVER_URL);

        let mut client = HttpClient::new(&tcp_client, &dns_client);
        let mut rx_buf = [0u8; 4096];

        let fb = display.framebuffer_mut();

        match fetch_framebuffer(&mut client, &mut rx_buf, fb).await {
            Ok(()) => {
                let new_hash = fnv1a(fb);
                if new_hash != current_hash {
                    println!("fetch: image changed, refreshing display");
                    match display.flush() {
                        Ok(()) => {
                            current_hash = new_hash;
                            println!("fetch: display updated");
                        }
                        Err(e) => println!("fetch: flush failed: {:?}", e),
                    }
                } else {
                    println!("fetch: image unchanged, skipping refresh");
                }
            }
            Err(e) => println!("fetch: failed: {}", e),
        }

        Timer::after(Duration::from_secs(60)).await;
    }
}

async fn fetch_framebuffer<'a>(
    client: &mut HttpClient<'a, TcpClient<'a, 1, 1500, 1500>, DnsSocket<'a>>,
    rx_buf: &mut [u8],
    fb: &mut [u8; FB_SIZE],
) -> Result<(), &'static str> {
    let mut builder = client
        .request(Method::GET, SERVER_URL)
        .await
        .map_err(|_| "failed to create request")?;

    let response = builder
        .send(rx_buf)
        .await
        .map_err(|_| "failed to send request")?;

    let status = response.status.0;
    if status != 200 {
        println!("fetch: server returned {}", status);
        return Err("non-200 response");
    }

    let body = response.body();
    let mut reader = body.reader();
    let mut offset = 0;

    while offset < FB_SIZE {
        let n = reader
            .read(&mut fb[offset..])
            .await
            .map_err(|_| "read error")?;

        if n == 0 {
            break;
        }
        offset += n;
    }

    println!("fetch: received {} bytes", offset);
    if offset != FB_SIZE {
        println!("fetch: warning: expected {} bytes, got {}", FB_SIZE, offset);
    }

    Ok(())
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        println!("wifi: connecting...");
        match controller.connect_async().await {
            Ok(info) => {
                println!("wifi: connected {:?}", info);
                let info = controller.wait_for_disconnect_async().await.ok();
                println!("wifi: disconnected {:?}", info);
            }
            Err(e) => {
                println!("wifi: connect failed: {:?}", e);
            }
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface>) {
    runner.run().await
}
