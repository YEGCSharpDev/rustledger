//! Beancount Language Server.
//!
//! Usage:
//!   rledger-lsp              # Start LSP server (stdio)
//!   rledger-lsp --version    # Print version
//!   rledger-lsp --help       # Print help

use rustledger_lsp::Server;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse simple args (no clap needed for LSP server)
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("rledger-lsp {}", rustledger_lsp::VERSION);
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Beancount Language Server");
        println!();
        println!("Usage: rledger-lsp [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -h, --help     Print help");
        println!("  -V, --version  Print version");
        println!();
        println!("The server communicates via stdio using the Language Server Protocol.");
        return ExitCode::SUCCESS;
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Run the server
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let server = Server::new();
        server.run().await;
    });

    ExitCode::SUCCESS
}
