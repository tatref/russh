extern crate env_logger;
extern crate futures;
extern crate russh;
extern crate russh_keys;
extern crate tokio;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context;
use russh::*;
use russh_keys::*;

struct Client {}

impl client::Handler for Client {
    type Error = russh::Error;
    type FutureUnit = futures::future::Ready<Result<(Self, client::Session), Self::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, bool), Self::Error>>;

    fn finished_bool(self, b: bool) -> Self::FutureBool {
        futures::future::ready(Ok((self, b)))
    }
    fn finished(self, session: client::Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, session)))
    }
    fn check_server_key(self, server_public_key: &key::PublicKey) -> Self::FutureBool {
        println!("check_server_key: {:?}", server_public_key);
        self.finished_bool(true)
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let config = russh::client::Config::default();
    let config = Arc::new(config);
    let sh = Client {};

    let mut agent = russh_keys::agent::client::AgentClient::connect_env()
        .await
        .unwrap();
    let mut identities = agent.request_identities().await.unwrap();
    let mut session =
        russh::client::connect(config, SocketAddr::from_str("127.0.0.1:2200").unwrap(), sh)
            .await
            .unwrap();
    let (_, auth_res) = session
        .authenticate_future("pe", identities.pop().unwrap(), agent)
        .await;
    let auth_res = auth_res.unwrap();
    println!("=== auth: {}", auth_res);
    let mut channel = session
        .channel_open_direct_tcpip("localhost", 8000, "localhost", 3333)
        .await
        .unwrap();
    // let mut channel = session.channel_open_session().await.unwrap();
    println!("=== after open channel");
    let data = b"GET /les_affames.mkv HTTP/1.1\nUser-Agent: curl/7.68.0\nAccept: */*\nConnection: close\n\n";
    channel.data(&data[..]).await.unwrap();
    let mut f = std::fs::File::create("les_affames.mkv").unwrap();
    while let Some(msg) = channel.wait().await {
        use std::io::Write;
        match msg {
            russh::ChannelMsg::Data { ref data } => {
                f.write_all(&data).unwrap();
            }
            russh::ChannelMsg::Eof => {
                f.flush().unwrap();
                break;
            }
            _ => {}
        }
    }
    session
        .disconnect(Disconnect::ByApplication, "", "English")
        .await
        .unwrap();
    let res = session.await.context("session await");
    println!("{:#?}", res);
}
