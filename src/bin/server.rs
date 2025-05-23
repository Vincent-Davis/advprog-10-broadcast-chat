use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, Mutex},
};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

/* ========  data-structure  ======== */

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct ChatMessage {
    #[serde(rename = "messageType")]
    message_type: MsgType,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(rename = "dataArray")]
    #[serde(skip_serializing_if = "Option::is_none")]
    data_array: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
enum MsgType {
    Users,
    Register,
    Message,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UserMessage {
    from: String,
    message: String,
    time: u64,
}

struct User {
    _addr: SocketAddr,
    nickname: String,
}

type Users = Arc<Mutex<HashMap<SocketAddr, User>>>;
type Broadcaster = broadcast::Sender<Message>;

/* ========  helpers  ======== */

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

async fn push_users(users: &HashMap<SocketAddr, User>, tx: &Broadcaster) {
    let list = users.values().map(|u| u.nickname.clone()).collect();
    let msg = ChatMessage {
        message_type: MsgType::Users,
        data: None,
        data_array: Some(list),
    };
    let _ = tx.send(Message::text(serde_json::to_string(&msg).unwrap()));
}

/* ========  connection  ======== */

async fn handle_connection(
    addr: SocketAddr,
    mut ws: WebSocketStream<TcpStream>,
    users: Users,
    tx: Broadcaster,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut rx = tx.subscribe();
    let mut nickname: Option<String> = None;

    let (mut sender, mut receiver) = ws.split();

    /* --- kirim broadcast ke klien ini --- */
    let mut outbound = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });

    /* --- terima pesan dari klien --- */
    let users_clone = users.clone();
    let tx_clone = tx.clone();
    let mut inbound = tokio::spawn(async move {
        while let Some(frame) = receiver.next().await {
            let payload = match frame {
                Ok(msg) if msg.is_text() => msg.as_text().unwrap_or("").to_owned(),
                Ok(msg) if msg.is_close() => break,
                Ok(_) => continue,
                Err(e) => {
                    eprintln!("recv {addr}: {e}");
                    break;
                }
            };

            /* coba parse JSON */
            if let Ok(cm) = serde_json::from_str::<ChatMessage>(&payload) {
                match cm.message_type {
                    MsgType::Register => {
                        let nick = cm.data.unwrap_or_else(|| addr.to_string());
                        nickname = Some(nick.clone());

                        let mut lock = users_clone.lock().await;
                        lock.insert(addr, User {
                            _addr: addr,
                            nickname: nick,
                        });
                        push_users(&lock, &tx_clone).await;
                    }
                    MsgType::Message => {
                        if let Some(ref nick) = nickname {
                            let um = UserMessage {
                                from: nick.clone(),
                                message: cm.data.unwrap_or_default(),
                                time: now_ms(),
                            };
                            let env = ChatMessage {
                                message_type: MsgType::Message,
                                data: Some(serde_json::to_string(&um).unwrap()),
                                data_array: None,
                            };
                            let _ = tx_clone
                                .send(Message::text(serde_json::to_string(&env).unwrap()));
                        }
                    }
                    _ => {}
                }
            } else {
                /* fallback plaintext */
                let nick = nickname.clone().unwrap_or_else(|| addr.to_string());
                let um = UserMessage {
                    from: nick.to_owned(),        // ← &str → String
                    message: payload.to_owned(),  // ← &str → String
                    time: now_ms(),
                };
                let env = ChatMessage {
                    message_type: MsgType::Message,
                    data: Some(serde_json::to_string(&um).unwrap()),
                    data_array: None,
                };
                let _ = tx_clone.send(Message::text(serde_json::to_string(&env).unwrap()));
            }
        }
        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });

    tokio::select! {
        _ = &mut outbound => {},
        _ = &mut inbound  => {},
    }

    /* --- bersih-bersih --- */
    {
        let mut lock = users.lock().await;
        if lock.remove(&addr).is_some() {
            push_users(&lock, &tx).await;
        }
    }

    Ok(())
}

/* ========  main  ======== */

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("🟢  server listening on ws://127.0.0.1:8080");

    let (tx, _) = broadcast::channel::<Message>(128);
    let users: Users = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (sock, addr) = listener.accept().await?;
        let users = users.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let ws = match ServerBuilder::new().accept(sock).await {
                Ok(stream) => stream,          // <- hanya 1 nilai
                Err(e) => {
                    eprintln!("handshake {addr}: {e}");
                    return;
                }
            };

            if let Err(e) = handle_connection(addr, ws, users, tx).await {
                eprintln!("conn {addr}: {e}");
            }
        });
    }
}
