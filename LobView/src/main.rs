mod api;
mod examples;

use api::BinanceClient;
use std::env;
use std::error::Error;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set subscriber");

    // Check command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Print usage if no arguments
    if args.len() == 1 {
        println!("Usage: {} [command]", args[0]);
        println!("Commands:");
        println!("  monitor             - Run the multi-symbol orderbook monitor");
        println!("  stream              - Run the orderbook stream example");
        println!("  [symbol]            - Subscribe to a specific symbol (e.g., btcusdt)");
        return Ok(());
    }
    
    match args[1].as_str() {
        "monitor" => {
            // Run the order book monitor example
            examples::run_order_book_monitor().await?;
        },
        "stream" => {
            // Run the orderbook stream example
            examples::run_orderbook_stream_example().await?;
        },
        symbol => {
            info!("Starting Binance client");

            // Create a new Binance client
            let client = BinanceClient::new().await?;

            // Subscribe to order book updates
            client.subscribe(symbol).await?;
            info!("Subscribed to {} order book", symbol);

            // Wait for some updates
            info!("Waiting for order book updates (15 seconds)...");
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;

            // Print the current order book
            if let Some(orderbook) = client.get_orderbook(symbol) {
                info!("Order book for {}:", symbol);
                info!("Last update ID: {}", orderbook.last_update_id);
                
                info!("Top 5 bids:");
                for (i, bid) in orderbook.bids.iter().take(5).enumerate() {
                    info!("  {}. Price: {}, Quantity: {}", i + 1, bid.price, bid.quantity);
                }
                
                info!("Top 5 asks:");
                for (i, ask) in orderbook.asks.iter().take(5).enumerate() {
                    info!("  {}. Price: {}, Quantity: {}", i + 1, ask.price, ask.quantity);
                }
            } else {
                info!("No order book available for {}", symbol);
            }

            // Keep the program running to continue receiving updates
            info!("Press Ctrl+C to exit");
            tokio::signal::ctrl_c().await?;
            info!("Shutting down");
        }
    }

    Ok(())
}
