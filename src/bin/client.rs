use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use http::Uri;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_websockets::{ClientBuilder, Message};

#[tokio::main]
async fn main() -> Result<(), tokio_websockets::Error> {
    let (mut ws_stream, _) =
        ClientBuilder::from_uri(Uri::from_static("ws://127.0.0.1:2000"))
            .connect()
            .await?;

    let stdin = tokio::io::stdin();
    let mut stdin = BufReader::new(stdin).lines();

    // Split the WebSocket stream to be able to send and receive at the same time
    let (mut sender, mut receiver) = ws_stream.split();

    // Spawn a task to read from stdin and send messages
    let stdin_to_ws = tokio::spawn(async move {
        println!("Type a message and press Enter to send:");
        
        while let Ok(Some(line)) = stdin.next_line().await {
            if line.is_empty() {
                continue;
            }
            
            if let Err(e) = sender.send(Message::text(line)).await {
                eprintln!("Error sending message: {}", e);
                break;
            }
        }
        
        Ok::<_, tokio_websockets::Error>(())
    });

    // Spawn a task to receive messages and print them
    let ws_to_stdout = tokio::spawn(async move {
        while let Some(message) = receiver.next().await {
            match message {
                Ok(msg) => {
                    if msg.is_text() {
                        println!("Received: {}", msg.as_text().unwrap_or(""));
                    } else if msg.is_close() {
                        println!("Server closed connection");
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                    break;
                }
            }
        }
        
        Ok::<_, tokio_websockets::Error>(())
    });

    // Wait for either task to complete
    tokio::select! {
        _ = stdin_to_ws => println!("Stdin task completed"),
        _ = ws_to_stdout => println!("WebSocket receive task completed"),
    }

    Ok(())
}