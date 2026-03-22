use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Receiver;
use heapless::String;
use reqwless::client::HttpClient;
use reqwless::request::{Method, RequestBuilder};
use static_cell::StaticCell;

use crate::channel::{Notification, NotificationKind};
use crate::config::{BatteryResult, SlotType, NOTIFY_URL, PUSHOVER_TOKEN, PUSHOVER_USER};

#[embassy_executor::task]
pub async fn notify_task(
    stack: Stack<'static>,
    rx: Receiver<'static, CriticalSectionRawMutex, Notification, 8>,
) {
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
    let handle = client
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

    match &notif.kind {
        NotificationKind::Complete {
            capacity_mah,
            result,
            duration_s,
        } => {
            push_u32(&mut msg, *capacity_mah as u32)?;
            push(&mut msg, " mAh - ")?;
            push(
                &mut msg,
                match result {
                    BatteryResult::Good => "Good",
                    BatteryResult::Weak => "Weak",
                    BatteryResult::Dead => "Dead",
                },
            )?;
            push(&mut msg, "\nDischarge time: ")?;
            let hours = duration_s / 3600;
            let minutes = (duration_s % 3600) / 60;
            if hours > 0 {
                push_u32(&mut msg, hours)?;
                push(&mut msg, "h ")?;
            }
            push_u32(&mut msg, minutes)?;
            push(&mut msg, "m")?;
        }
        NotificationKind::WrongChemistry { ocv } => {
            push(&mut msg, "wrong chemistry (")?;
            push_u32(&mut msg, (*ocv * 100.0) as u32 / 100)?;
            push(&mut msg, ".")?;
            push_u32(&mut msg, ((*ocv * 100.0) as u32) % 100)?;
            push(&mut msg, "V)\nAlkaline or non-NiMH detected")?;
        }
        NotificationKind::NotCharged { ocv } => {
            push(&mut msg, "not charged (")?;
            push_u32(&mut msg, (*ocv * 100.0) as u32 / 100)?;
            push(&mut msg, ".")?;
            push_u32(&mut msg, ((*ocv * 100.0) as u32) % 100)?;
            push(&mut msg, "V)\nOCV too low to test")?;
        }
        NotificationKind::Timeout {
            capacity_mah,
            duration_s,
        } => {
            push(&mut msg, "timeout after ")?;
            let hours = duration_s / 3600;
            let minutes = (duration_s % 3600) / 60;
            if hours > 0 {
                push_u32(&mut msg, hours)?;
                push(&mut msg, "h ")?;
            }
            push_u32(&mut msg, minutes)?;
            push(&mut msg, "m\nCapacity so far: ")?;
            push_u32(&mut msg, *capacity_mah as u32)?;
            push(&mut msg, " mAh")?;
        }
        NotificationKind::AdcFault => {
            push(&mut msg, "ADC communication failure")?;
        }
    }

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
