use log::debug;
use tonic::transport::Server;

use bazelfe_protos::*;

use crate::bep::build_events;
use crate::bep::BazelEventHandler;
use crate::bep::EventStreamListener;

use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::sync::Arc;

pub struct BazelWrapperBuilder<T> {
    pub bes_server_bind_address: Option<std::net::SocketAddr>,
    pub processors: Vec<Arc<dyn BazelEventHandler<T>>>,
}
impl<T> BazelWrapperBuilder<T>
where
    T: Send + 'static,
{
    pub async fn build(self) -> Result<super::BazelWrapper<T>, super::BazelWrapperError> {
        let mut rng = rand::thread_rng();

        crate::bazel_subprocess_wrapper::register_ctrlc_handler();
        let aes = EventStreamListener::new(self.processors);

        let default_port = {
            let rand_v: u16 = rng.gen();
            40000 + (rand_v % 3000)
        };

        let addr: std::net::SocketAddr = self
            .bes_server_bind_address
            .map(|s| s.to_owned())
            .unwrap_or_else(|| {
                format!("127.0.0.1:{}", default_port)
                    .parse()
                    .expect("can't parse BIND_ADDRESS variable")
            });

        debug!("Services listening on {}", addr);

        let (bes, sender_arc, _) =
            build_events::build_event_server::build_bazel_build_events_service();

        let bes_port: u16 = addr.port();

        let _service_fut = tokio::spawn(async move {
            Server::builder()
                .add_service(PublishBuildEventServer::new(bes))
                .serve(addr)
                .await
                .unwrap();
        });

        Ok(super::BazelWrapper::new(&sender_arc, aes, bes_port))
    }
}
