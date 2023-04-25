use clap::Parser;
use color_eyre::Report;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Verbosity log
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

const VERBOSE_LEVEL: &[&str] = &["info", "debug", "trace"];

pub fn initialize() -> Result<Args, Report> {
    color_eyre::install()?;
    let args = Args::parse();

    let verbosity = match args.verbose {
        1..=3 => Some(VERBOSE_LEVEL[(args.verbose as usize) - 1]),
        _ => None,
    };

    // let verbosity_str = verbosity.unwrap_or("trace");
    // let env_filter = EnvFilter::from_default_env()
    //     .add_directive(tracing::level_filters::LevelFilter::INFO.into())
    //     .add_directive(format!("{}={}", env!("CARGO_PKG_NAME"), verbosity_str).parse()?)
    //     .add_directive(tracing::level_filters::LevelFilter::WARN.into());

    let env_filter = EnvFilter::from_default_env()
        .add_directive(tracing::level_filters::LevelFilter::DEBUG.into());

    // Default logging layer
    let fmt_layer = fmt::layer().with_writer(std::io::stderr);

    match verbosity {
        Some(_) => {
            // construct a layer that prints formatted traces to stderr
            let fmt_layer = fmt_layer
                .with_level(true) // include levels in formatted output
                .with_target(true) // include targets
                .with_thread_ids(true) // include the thread ID of the current thread
                .with_thread_names(true); // include the name of the current thread

            registry()
                .with(ErrorLayer::default())
                .with(fmt_layer)
                .with(env_filter)
                .init();
        }
        None => {
            // construct a layer that prints formatted traces to stderr
            let fmt_layer = fmt_layer.without_time().compact();

            registry()
                .with(ErrorLayer::default())
                .with(fmt_layer)
                .with(env_filter)
                .init();
        }
    };

    Ok(args)
}
