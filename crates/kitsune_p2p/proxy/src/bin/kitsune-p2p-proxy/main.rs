use futures::stream::StreamExt;
use ghost_actor::dependencies::tracing;
use kitsune_p2p_proxy::*;
use kitsune_p2p_transport_quic::*;
use kitsune_p2p_types::{
    dependencies::{ghost_actor, serde_json},
    transport::*,
};
use structopt::StructOpt;

mod opt;
use opt::*;

#[tokio::main]
async fn main() {
    let _ = ghost_actor::dependencies::tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    );

    if let Err(e) = inner().await {
        eprintln!("{:?}", e);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TlsFileCert {
    #[serde(with = "serde_bytes")]
    pub cert: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub priv_key: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub digest: Vec<u8>,
}

impl From<TlsConfig> for TlsFileCert {
    fn from(f: TlsConfig) -> Self {
        Self {
            cert: f.cert.to_vec(),
            priv_key: f.cert_priv_key.to_vec(),
            digest: f.cert_digest.to_vec(),
        }
    }
}

impl From<TlsFileCert> for TlsConfig {
    fn from(f: TlsFileCert) -> Self {
        Self {
            cert: f.cert.into(),
            cert_priv_key: f.priv_key.into(),
            cert_digest: f.digest.into(),
        }
    }
}

async fn inner() -> TransportResult<()> {
    let opt = Opt::from_args();

    if let Some(gen_cert) = &opt.danger_gen_unenc_cert {
        let tls = TlsConfig::new_ephemeral().await?;
        let gen_cert2 = gen_cert.clone();
        tokio::task::spawn_blocking(move || {
            let tls = TlsFileCert::from(tls);
            let mut out = Vec::new();
            kitsune_p2p_types::codec::rmp_encode(&mut out, &tls).map_err(TransportError::other)?;
            std::fs::write(gen_cert2, &out).map_err(TransportError::other)?;
            TransportResult::Ok(())
        })
        .await
        .map_err(TransportError::other)??;
        println!("Generated {:?}.", gen_cert);
        return Ok(());
    }

    let tls_conf = if let Some(use_cert) = &opt.danger_use_unenc_cert {
        let use_cert = use_cert.clone();
        tokio::task::spawn_blocking(move || {
            let tls = std::fs::read(use_cert).map_err(TransportError::other)?;
            let tls: TlsFileCert =
                kitsune_p2p_types::codec::rmp_decode(&mut std::io::Cursor::new(&tls))
                    .map_err(TransportError::other)?;
            TransportResult::Ok(TlsConfig::from(tls))
        })
        .await
        .map_err(TransportError::other)??
    } else {
        TlsConfig::new_ephemeral().await?
    };

    let (listener, events) = spawn_transport_listener_quic(opt.into()).await?;

    let proxy_config = ProxyConfig::local_proxy_server(tls_conf, AcceptProxyCallback::accept_all());

    let (listener, mut events) =
        spawn_kitsune_proxy_listener(proxy_config, listener, events).await?;

    println!("{}", listener.bound_url().await?);

    tokio::task::spawn(async move {
        while let Some(evt) = events.next().await {
            match evt {
                TransportEvent::IncomingChannel(url, mut write, _read) => {
                    tracing::debug!(
                        "{} is trying to talk directly to us - dump proxy state",
                        url
                    );
                    match listener.debug().await {
                        Ok(dump) => {
                            let dump = serde_json::to_string_pretty(&dump).unwrap();
                            let _ = write.write_and_close(dump.into_bytes()).await;
                        }
                        Err(e) => {
                            let _ = write.write_and_close(format!("{:?}", e).into_bytes()).await;
                        }
                    }
                }
            }
        }
    });

    // wait for ctrl-c
    futures::future::pending().await
}
