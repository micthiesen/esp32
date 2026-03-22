use embassy_net::{Runner, Stack, StackResources};
use embassy_time::Duration;
use esp_radio::wifi::{ClientConfig, ModeConfig, WifiController, WifiDevice};
use static_cell::StaticCell;

/// Initialize WiFi station mode and return the network stack.
///
/// Spawns background tasks for the network runner and connection management.
/// Requires `WIFI_SSID` and `WIFI_PASSWORD` env vars set at compile time.
pub async fn init(
    spawner: &embassy_executor::Spawner,
    wifi: esp_hal::peripherals::WIFI<'static>,
) -> Stack<'static> {
    let ssid: &str = env!("WIFI_SSID");
    let password: &str = env!("WIFI_PASSWORD");

    // Initialize the radio subsystem (static lifetime needed for spawned tasks)
    static CONTROLLER: StaticCell<esp_radio::Controller<'static>> = StaticCell::new();
    let controller = CONTROLLER.init(esp_radio::init().unwrap());

    // Create WiFi controller + interfaces
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(controller, wifi, esp_radio::wifi::Config::default()).unwrap();

    // Configure station mode
    let client_config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(ssid.into())
            .with_password(password.into()),
    );
    wifi_controller.set_config(&client_config).unwrap();

    // Start WiFi and connect
    wifi_controller.start_async().await.unwrap();
    log::info!("[wifi] started");
    wifi_controller.connect_async().await.unwrap();
    log::info!("[wifi] connected to {}", ssid);

    // Create embassy-net stack with DHCP
    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let seed = esp_hal::rng::Rng::new().random() as u64;

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let resources = RESOURCES.init(StackResources::new());

    static RUNNER: StaticCell<Runner<'static, WifiDevice<'static>>> = StaticCell::new();

    let (stack, runner) = embassy_net::new(interfaces.sta, net_config, resources, seed);
    let runner = RUNNER.init(runner);

    spawner.spawn(net_task(runner)).unwrap();
    spawner.spawn(connection_task(wifi_controller)).unwrap();

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
