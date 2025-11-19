use anyhow::Result;
use clap::{Parser, Subcommand};
use instrument_to_midi::{config::Config, midi, processor::StreamProcessor};
use log::info;

#[derive(Parser)]
#[command(name = "instrument_to_midi")]
#[command(author = "Ian Lintner")]
#[command(version = "0.1.0")]
#[command(about = "Real-time guitar to MIDI conversion", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start real-time audio to MIDI conversion
    Stream {
        /// MIDI output port name (omit to create virtual port)
        #[arg(short, long)]
        port: Option<String>,

        /// Audio buffer size
        #[arg(short, long, default_value = "2048")]
        buffer_size: usize,

        /// MIDI velocity (0-127)
        #[arg(short, long, default_value = "80")]
        velocity: u8,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,

        /// Configuration file path
        #[arg(short, long)]
        config: Option<String>,

        /// Enable MIDI recording to file
        #[arg(short, long)]
        record: bool,

        /// Output file path for MIDI recording (defaults to recording_<timestamp>.mid)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// List available MIDI output ports
    ListPorts,

    /// Generate default configuration file
    GenerateConfig {
        /// Output file path
        #[arg(default_value = "config.json")]
        output: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stream {
            port,
            buffer_size,
            velocity,
            verbose,
            config: config_file,
            record,
            output,
        } => {
            // Initialize logger
            if verbose {
                env_logger::Builder::from_default_env()
                    .filter_level(log::LevelFilter::Debug)
                    .init();
            } else {
                env_logger::Builder::from_default_env()
                    .filter_level(log::LevelFilter::Info)
                    .init();
            }

            // Load or create config
            let mut config = if let Some(path) = config_file {
                Config::from_file(&path)?
            } else {
                Config::default()
            };

            // Override with CLI arguments
            config.midi_port = port;
            config.buffer_size = buffer_size;
            config.velocity = velocity;
            config.verbose = verbose;
            config.record_enabled = record;
            config.record_output = output;

            info!("Starting instrument to MIDI converter...");
            info!("Buffer size: {}", config.buffer_size);
            info!("Velocity: {}", config.velocity);
            if config.record_enabled {
                info!("Recording enabled");
                if let Some(ref path) = config.record_output {
                    info!("Recording output: {}", path);
                }
            }

            // Create and start processor
            let mut processor = StreamProcessor::new(config)?;
            processor.start()?;

            Ok(())
        }

        Commands::ListPorts => {
            println!("Available MIDI output ports:");
            let ports = midi::list_midi_ports()?;
            if ports.is_empty() {
                println!("  (no ports found)");
            } else {
                for (i, port) in ports.iter().enumerate() {
                    println!("  {}: {}", i + 1, port);
                }
            }
            Ok(())
        }

        Commands::GenerateConfig { output } => {
            let config = Config::default();
            config.to_file(&output)?;
            println!("Configuration file generated: {}", output);
            Ok(())
        }
    }
}
