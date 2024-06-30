pub mod pd {
    tonic::include_proto!("chat");
}

use pd::ChatMessage;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::{Stream, StreamExt};
use tonic::transport::Server;
use tonic::{Request, Response, Status, Streaming};

#[derive(Debug)]
pub struct ChatServer {
    clients: Arc<Mutex<Vec<(i32, mpsc::Sender<Result<ChatMessage, Status>>)>>>,
    client_counter: Arc<Mutex<i32>>,
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<ChatMessage, Status>> + Send>>;
type ChatResult<T> = Result<Response<T>, Status>;

#[tonic::async_trait]
impl pd::chat_service_server::ChatService for ChatServer {
    type ChatMessageStreamingStream = ResponseStream;
    async fn chat_message_streaming(
        &self,
        request: Request<Streaming<ChatMessage>>,
    ) -> ChatResult<Self::ChatMessageStreamingStream> {
        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);

        let current_counter: i32;
        {
            let mut counter_guard = self.client_counter.lock().await;
            current_counter = *counter_guard;

            let mut clients = self.clients.lock().await;
            clients.push((current_counter, tx.clone()));
            *counter_guard += 1;
        }

        let clients_clone = self.clients.clone();

        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(item) => {
                        println!(
                            "id:{}|{:?}|>{:?}",
                            current_counter, item.from_name, item.message
                        );
                        let message = ChatMessage {
                            message: item.message,
                            from_name: item.from_name,
                            id: current_counter,
                        };

                        let clients_clone = clients_clone.lock().await;
                        for client in clients_clone.iter() {
                            client.1.send(Ok(message.clone())).await.unwrap();
                        }
                    }
                    Err(status) => {
                        println!("Error: {}", status);
                        break;
                    }
                }
            }

            {
                let mut clients_clone = clients_clone.lock().await;
                clients_clone.retain(|x| x.0 != current_counter);
                println!(
                    "id:{}さんが退出しました。チャットルーム人数:{}",
                    current_counter,
                    clients_clone.len()
                );
            }
        });

        let out = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(out) as Self::ChatMessageStreamingStream
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_counter = Arc::new(Mutex::new(0));
    let server = ChatServer {
        clients: Arc::new(Mutex::new(Vec::new())),
        client_counter: client_counter,
    };

    println!("Server Started..!");
    Server::builder()
        .add_service(pd::chat_service_server::ChatServiceServer::new(server))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}
