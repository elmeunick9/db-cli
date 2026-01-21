use tracing_subscriber::{fmt, prelude::*, filter::LevelFilter, EnvFilter, Registry};
use std::io;
use crate::config::Config;

pub fn init_logging(config: &Config) {
    let stdout_layer = fmt::layer()
        .with_writer(io::stdout)
        .with_ansi(true)          
        .with_level(false)        
        .with_target(false)       
        .with_thread_ids(false)   
        .with_thread_names(false) 
        .without_time()           
        .with_filter(LevelFilter::DEBUG);

    let sql_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::TRACE.into())
        .parse_lossy(if config.log_sql { "" } else { "sql=off" });

    let subscriber = Registry::default()
        .with(sql_filter)
        .with(stdout_layer);

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");
}