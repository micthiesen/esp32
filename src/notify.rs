use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Runner, Stack, StackResources};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use embassy_time::Duration;
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice};
use heapless::String;
use reqwless::client::HttpClient;
use reqwless::request::{Method, RequestBuilder};
use static_cell::StaticCell;

use crate::config::{
    BatteryResult, SlotType, NOTIFY_URL, PUSHOVER_TOKEN, PUSHOVER_USER, WIFI_PASSWORD, WIFI_SSID,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Notification {
    pub slot: u8,
    pub slot_type: SlotType,
    pub capacity_mah: f32,
    pub result: BatteryResult,
    pub duration_s: u32,
}

pub type NotifyChannel = embassy_sync::channel::Channel<CriticalSectionRawMutex, Notification, 8>;

// ---------------------------------------------------------------------------
// WiFi initialization
// ---------------------------------------------------------------------------

pub async fn init_wifi(
    spawner: &embassy_executor::Spawner,
    wifi: esp_hal::peripherals::WIFI<'static>,
) -> Stack<'static> {
    // Initialize the radio subsystem (static lifetime needed for spawned tasks)
    static CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let controller = CONTROLLER.init(esp_radio::init().unwrap());

    // Create WiFi controller + interfaces
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(controller, wifi, esp_radio::wifi::Config::default()).unwrap();

    // Configure station mode with SSID/password
    let client_config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(WIFI_SSID.try_into().unwrap())
            .with_password(WIFI_PASSWORD.try_into().unwrap()),
    );
    wifi_controller.set_config(&client_config).unwrap();

    // Start WiFi and connect
    wifi_controller.start_async().await.unwrap();
    log::info!("[wifi] started");
    wifi_controller.connect_async().await.unwrap();
    log::info!("[wifi] connected to {}", WIFI_SSID);

    // Create embassy-net stack with DHCP
    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let seed = esp_hal::rng::Rng::new().random() as u64;

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let resources = RESOURCES.init(StackResources::new());

    static RUNNER: StaticCell<Runner<'static, WifiDevice<'static>>> = StaticCell::new();

    let (stack, runner) = embassy_net::new(interfaces.sta, net_config, resources, seed);
    let runner = RUNNER.init(runner);

    // Spawn background tasks
    spawner.spawn(net_task(runner)).unwrap();
    spawner.spawn(connection_task(wifi_controller)).unwrap();

    // Wait for DHCP
    log::info!("[wifi] waiting for IP...");
    stack.wait_config_up().await;
    log::info!("[wifi] got IP");

    stack
}

#[embassy_executor::task]
async fn net_task(runner: &'static mut Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    loop {
        if !controller.is_connected().unwrap_or(false) {
            log::info!("[wifi] reconnecting...");
            let _ = controller.connect_async().await;
        }
        embassy_time::Timer::after(Duration::from_secs(5)).await;
    }
}

// ---------------------------------------------------------------------------
// Notification task
// ---------------------------------------------------------------------------

#[embassy_executor::task]
pub async fn notify_task(
    stack: Stack<'static>,
    rx: Receiver<'static, CriticalSectionRawMutex, Notification, 8>,
) {
    // TCP client state for reqwless (1 concurrent connection, 4KB buffers)
    static TCP_STATE: StaticCell<TcpClientState<1, 4096, 4096>> = StaticCell::new();
    let tcp_state = TCP_STATE.init(TcpClientState::new());
    let tcp_client = TcpClient::new(stack, tcp_state);
    let dns_socket = DnsSocket::new(stack);

    loop {
        let notif = rx.receive().await;
        if let Err(e) = send_notification(&tcp_client, &dns_socket, &notif).await {
            log::warn!("[notify] failed: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// HTTPS POST via reqwless
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum NotifyError {
    Format,
    Request,
    NonOkStatus,
}

impl core::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NotifyError::Format => write!(f, "format overflow"),
            NotifyError::Request => write!(f, "HTTP request failed"),
            NotifyError::NonOkStatus => write!(f, "non-200 status"),
        }
    }
}

async fn send_notification<'a>(
    tcp_client: &TcpClient<'a, 1, 4096, 4096>,
    dns_socket: &DnsSocket<'a>,
    notif: &Notification,
) -> Result<(), NotifyError> {
    let body = format_body(notif)?;

    let mut client = HttpClient::new(tcp_client, dns_socket);

    let mut rx_buf = [0u8; 1024];
    let headers = [("Content-Type", "application/x-www-form-urlencoded")];
    let mut handle = client
        .request(Method::POST, NOTIFY_URL)
        .await
        .map_err(|_| NotifyError::Request)?;
    let body_bytes = body.as_bytes();
    let mut request_with_body = handle.headers(&headers).body(body_bytes);
    let response = request_with_body
        .send(&mut rx_buf)
        .await
        .map_err(|_| NotifyError::Request)?;

    if response.status.is_successful() {
        log::info!("[notify] sent for slot {}", notif.slot);
        Ok(())
    } else {
        log::warn!("[notify] HTTP {}", response.status.0);
        Err(NotifyError::NonOkStatus)
    }
}

// ---------------------------------------------------------------------------
// Message / body formatting
// ---------------------------------------------------------------------------

/// Build the URL-encoded POST body.
/// Max body size ~512 bytes assuming token/user keys under 50 chars each.
/// Returns NotifyError::Format if the buffer overflows.
fn format_body(notif: &Notification) -> Result<String<512>, NotifyError> {
    let message = format_message(notif)?;
    let mut body: String<512> = String::new();
    push(&mut body, "token=")?;
    push(&mut body, PUSHOVER_TOKEN)?;
    push(&mut body, "&user=")?;
    push(&mut body, PUSHOVER_USER)?;
    push(&mut body, "&title=Battery+Tester&message=")?;
    // Minimal URL encoding for ASCII-only battery result messages.
    // Only handles space and newline. Sufficient for controlled inputs.
    for &b in message.as_bytes() {
        match b {
            b' ' => push(&mut body, "+")?,
            b'\n' => push(&mut body, "%0A")?,
            _ => body.push(b as char).map_err(|_| NotifyError::Format)?,
        }
    }
    Ok(body)
}

fn format_message(notif: &Notification) -> Result<String<128>, NotifyError> {
    let mut msg: String<128> = String::new();
    push(&mut msg, "Battery #")?;
    push_u32(&mut msg, notif.slot as u32)?;
    push(
        &mut msg,
        match notif.slot_type {
            SlotType::AA => " (AA): ",
            SlotType::AAA => " (AAA): ",
        },
    )?;
    push_u32(&mut msg, notif.capacity_mah as u32)?;
    push(&mut msg, " mAh - ")?;
    push(
        &mut msg,
        match notif.result {
            BatteryResult::Good => "Good",
            BatteryResult::Weak => "Weak",
            BatteryResult::Dead => "Dead",
        },
    )?;
    push(&mut msg, "\nDischarge time: ")?;
    let hours = notif.duration_s / 3600;
    let minutes = (notif.duration_s % 3600) / 60;
    if hours > 0 {
        push_u32(&mut msg, hours)?;
        push(&mut msg, "h ")?;
    }
    push_u32(&mut msg, minutes)?;
    push(&mut msg, "m")?;
    Ok(msg)
}

fn push<const N: usize>(s: &mut String<N>, val: &str) -> Result<(), NotifyError> {
    s.push_str(val).map_err(|_| NotifyError::Format)
}

fn push_u32<const N: usize>(s: &mut String<N>, val: u32) -> Result<(), NotifyError> {
    let mut buf = [0u8; 10];
    if val == 0 {
        return push(s, "0");
    }
    let mut v = val;
    let mut pos = buf.len();
    while v > 0 {
        pos -= 1;
        buf[pos] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    push(s, core::str::from_utf8(&buf[pos..]).unwrap())
}
