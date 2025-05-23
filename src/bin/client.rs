use futures_util::{SinkExt, StreamExt};
use http::Uri;                                 // ← http v0.2
use rand::Rng;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_websockets::{ClientBuilder, Message};

/* ========= protocol structs ========= */

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum MsgType {
    Users,
    Register,
    Message,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsMsg {
    #[serde(rename = "messageType")]
    message_type: MsgType,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(rename = "dataArray")]
    #[serde(skip_serializing_if = "Option::is_none")]
    data_array: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct Inner {
    from: String,
    message: String,
    time: u64,
}

/* ============== main ============== */

#[tokio::main]
async fn main() -> Result<(), tokio_websockets::Error> {
    let nick = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("cli-{}", rand::thread_rng().gen::<u16>()));

    /* ── connect ── */
    let (mut ws, _) = ClientBuilder::from_uri(Uri::from_static("ws://127.0.0.1:8080"))
        .connect()
        .await?;

    /* ── send REGISTER ── */
    ws.send(Message::text(
        serde_json::to_string(&WsMsg {
            message_type: MsgType::Register,
            data: Some(nick.clone()),
            data_array: None,
        })
        .unwrap(),
    ))
    .await?;

    let (mut tx, mut rx) = ws.split();

    /* stdin → ws */
    let stdin_task = tokio::spawn(async move {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        println!("> ketik pesan lalu Enter");
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }
            let out = WsMsg {
                message_type: MsgType::Message,
                data: Some(line),
                data_array: None,
            };
            if tx
                .send(Message::text(serde_json::to_string(&out).unwrap()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    /* ws → stdout */
    let stdout_task = tokio::spawn(async move {
        while let Some(frame) = rx.next().await {
            match frame {
                Ok(msg) if msg.is_text() => {
                    let txt = msg.as_text().unwrap_or("").to_owned();   // ← ganti .into_text()
                    if let Ok(parsed) = serde_json::from_str::<WsMsg>(&txt) {
                        match parsed.message_type {
                            MsgType::Users => {
                                let list = parsed.data_array.unwrap_or_default();
                                println!("👥  users: {}", list.join(", "));
                            }
                            MsgType::Message => {
                                let inner: Inner =
                                    serde_json::from_str(parsed.data.as_ref().unwrap()).unwrap();
                                println!("[{}] {}", inner.from, inner.message);
                            }
                            _ => {}
                        }
                    } else {
                        println!("{txt}");
                    }
                }
                Ok(msg) if msg.is_close() => break,
                Err(e) => {
                    eprintln!("error: {e}");
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::try_join!(stdin_task, stdout_task).unwrap();
    Ok(())
}
