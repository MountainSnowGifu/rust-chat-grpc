pub mod pd {
    tonic::include_proto!("chat");
}

use pd::chat_service_client::ChatServiceClient;
use pd::ChatMessage;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;

pub async fn input_name() -> String {
    println!("Type your name...!");
    let mut inp = String::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    reader
        .read_line(&mut inp)
        .await
        .expect("Failed to read line");
    inp.trim().to_string()
}

pub async fn input() -> String {
    let mut inp = String::new();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    reader
        .read_line(&mut inp)
        .await
        .expect("Failed to read line");
    inp.trim().to_string()
}

async fn chat(client: &mut ChatServiceClient<Channel>, name: String) {
    let (tx, rx) = mpsc::channel(128);

    let in_stream = ReceiverStream::new(rx);

    tokio::spawn(async move {
        loop {
            let user_msg = input().await;
            if user_msg.eq_ignore_ascii_case("exit") {
                break;
            } else {
                let msg = ChatMessage {
                    message: user_msg,
                    from_name: name.clone(),
                    id: 0,
                };

                if tx.send(msg).await.is_err() {
                    break;
                }
            }
        }
    });

    let response = client
        .chat_message_streaming(Request::new(in_stream))
        .await
        .unwrap();

    let mut resp_stream = response.into_inner();

    while let Some(receive) = resp_stream.next().await {
        let item = receive.unwrap();
        println!("id:{}|{:?}|>{:?}", item.id, item.from_name, item.message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = input_name().await;

    let mut client = ChatServiceClient::connect("http://[::1]:50051")
        .await
        .unwrap();

    println!("Client Started..!");
    chat(&mut client, name).await;
    Ok(())
}
