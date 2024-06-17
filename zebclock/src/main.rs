mod zchronod;
mod node_factory;
mod storage;
mod vlc;
mod handler;
mod api;

use std::path::PathBuf;
use db_sql::pg::pg_client::setup_db;
use tools::tokio_zchronod;
use node_api::config;
use structopt::StructOpt;
use node_api::error::ZchronodConfigError;
use tracing::*;
use node_api::config::ZchronodConfig;
use tracing_subscriber::FmtSubscriber;
use crate::zchronod::ZchronodArc;
use crate::zchronod::Zchronod;

#[derive(StructOpt)]
struct ZchronodCli {
    #[structopt(short = "c", long = "config", parse(from_os_str), help = "Yaml file only")]
    config_path: Option<std::path::PathBuf>,

    #[structopt(long = "init_pg", help = "Init & refresh pg, caution: new db & new table")]
    init_pg: Option<String>,
}

fn main() {
    tokio_zchronod::block_forever_on(async_main());
}

async fn async_main() {
    let fmt_subscriber = FmtSubscriber::new();
    tracing::subscriber::set_global_default(fmt_subscriber)
        .expect("set default tracing subscriber fail");

    info!("start zchronod server");
    let mut help_info = true;
    let args = ZchronodCli::from_args();

    // init pg db
    if let Some(pg_conn_str) = args.init_pg {
        help_info = false;
        info!("PostgreSQL connection addr: {}", pg_conn_str);
        // Use the PostgreSQL connection string here for initialization
        if !init_db(pg_conn_str).await {
            return;
        }
    }

    // setup node
    if let Some(config_path) = args.config_path {
        help_info = false;
        let zchronod_config = construct_node_config(config_path.clone());

        //todo metrics init

        let _zchronod = build_zchronod(zchronod_config.clone()).await;
    }

    if help_info {
        info!("\nPlease exec: zchronod -h for help info.\n")
    }
}

async fn init_db(postgres_conn_str: String) -> bool {
    return if let Ok(url) = url::Url::parse(&postgres_conn_str) {
        let db_name = url.path().trim_start_matches('/');
        let base_url = url.as_str().trim_end_matches(db_name);
        let is_db_name_empty = db_name == "";
        info!("Base URL: {}", base_url);
        info!("Database Name: {}", db_name);
        if is_db_name_empty {
            error!("Database name is empty, exiting");
            return false;
        }

        match setup_db(base_url, db_name).await {
            Err(err) => {
                error!("{}", err);
                false
            }
            Ok(conn) => {
                info!("Setup database success");
                let _ = conn.close().await;
                true
            }
        }
    } else {
        error!("Invalid PostgreSQL connection string");
        false
    };
}

async fn build_zchronod(config: ZchronodConfig) -> ZchronodArc {
    Zchronod::zchronod_factory().set_config(config).initialize_node().await.map_err(|e| {
        panic!("Failed to build Zchronod due to error [{:?}]", e);
    }).unwrap()
}

fn construct_node_config(config_path: PathBuf) -> config::ZchronodConfig {
    match config::ZchronodConfig::load_config(config_path) {
        Err(ZchronodConfigError::ConfigMissing(_)) => {
            error!("config path can't found.");
            std::process::exit(ERROR_CODE);
        }
        Err(ZchronodConfigError::SerializationError(_)) => {
            error!("config file can't be serialize, bad yaml format");
            std::process::exit(ERROR_CODE);
        }
        Err(ZchronodConfigError::IllegalNodeId) => {
            error!("nodeid illegal, must be hex format, and 64 bits");
            std::process::exit(ERROR_CODE);
        }
        result => {
            result.expect("failed to load zhronod config")
        }
    }
}

/// start Zchronod node error code for loading config
pub const ERROR_CODE: i32 = 42;