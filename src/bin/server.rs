use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{channel, Sender};
use tokio_websockets::{Message, ServerBuilder, WebSocketStream};

async fn handle_connection(
    addr: SocketAddr,
    mut ws_stream: WebSocketStream<TcpStream>,
    bcast_tx: Sender<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Subscribe to the broadcast channel
    let mut bcast_rx = bcast_tx.subscribe();

    // Split the WebSocket stream to be able to send and receive at the same time
    let (mut sender, mut receiver) = ws_stream.split();

    // Spawn a task to receive broadcast messages and forward them to this client
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = bcast_rx.recv().await {
            // Skip messages that were sent by this client
            if sender.send(Message::text(msg)).await.is_err() {
                break;
            }
        }
        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });

    // Task for receiving messages from this client
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = receiver.next().await {
            match result {
                Ok(msg) => {
                    if msg.is_text() {
                        // Extract the message text
                        let text = msg.as_text().unwrap_or("").to_string();
                        println!("Received from {addr}: {text}");
                        
                        // Broadcast to all subscribers
                        let _ = bcast_tx.send(text);
                    } else if msg.is_close() {
                        break;
                    }
                }
                Err(e) => {
                    println!("Error receiving from {addr}: {e}");
                    break;
                }
            }
        }
        println!("Client {addr} disconnected");
        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });

    // Wait for either task to complete (e.g. if client disconnects)
    tokio::select! {
        result = &mut send_task => result??,
        result = &mut recv_task => result??,
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (bcast_tx, _) = channel(16);

    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    println!("listening on port 8000");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {addr:?}");
        let bcast_tx = bcast_tx.clone();
        tokio::spawn(async move {
            // Wrap the raw TCP stream into a websocket.
            let (_, ws_stream) = ServerBuilder::new().accept(socket).await?;

            handle_connection(addr, ws_stream, bcast_tx).await
        });
    }
}