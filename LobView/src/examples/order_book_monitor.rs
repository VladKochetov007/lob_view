use crate::api::{BinanceClient, OrderBook};
use std::error::Error;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tracing::info;

/// A simple order book monitor that subscribes to multiple symbols
/// and tracks changes in the best bid and ask prices
pub async fn run_order_book_monitor() -> Result<(), Box<dyn Error>> {
    info!("Starting Order Book Monitor");

    // Create a new Binance client
    let client = BinanceClient::new().await?;

    // Symbols to monitor
    let symbols = vec!["btcusdt", "ethusdt", "solusdt"];

    // Subscribe to each symbol
    for symbol in &symbols {
        client.subscribe(symbol).await?;
        info!("Subscribed to {} order book", symbol);
    }

    // Wait for initial data
    info!("Waiting for initial data (3 seconds)...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Create a channel to periodically check order books
    let (tx, mut rx) = mpsc::channel(1);
    
    // Spawn a task to send a signal every second
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if tx.send(()).await.is_err() {
                break;
            }
        }
    });

    // Track previous best prices to detect changes
    let mut prev_best_bids = std::collections::HashMap::new();
    let mut prev_best_asks = std::collections::HashMap::new();

    info!("Monitoring order books - press Ctrl+C to stop");
    
    // Monitor loop
    while rx.recv().await.is_some() {
        for symbol in &symbols {
            if let Some(orderbook) = client.get_orderbook(symbol) {
                // Get the best bid and ask
                let best_bid = orderbook.bids.first().map(|bid| bid.price).unwrap_or(0.0);
                let best_ask = orderbook.asks.first().map(|ask| ask.price).unwrap_or(0.0);
                
                // Calculate spread
                let spread = best_ask - best_bid;
                let spread_percent = if best_bid > 0.0 { (spread / best_bid) * 100.0 } else { 0.0 };
                
                // Check if prices have changed
                let prev_bid = prev_best_bids.get(&symbol.to_string()).copied().unwrap_or(0.0);
                let prev_ask = prev_best_asks.get(&symbol.to_string()).copied().unwrap_or(0.0);
                
                if best_bid != prev_bid || best_ask != prev_ask {
                    info!(
                        "{}: Best Bid: {:.2}, Best Ask: {:.2}, Spread: {:.2} ({:.4}%)",
                        symbol.to_uppercase(), best_bid, best_ask, spread, spread_percent
                    );
                    
                    // Calculate market depth at different price levels
                    let bid_depth = calculate_depth(&orderbook, true, 0.1);
                    let ask_depth = calculate_depth(&orderbook, false, 0.1);
                    
                    info!(
                        "{}: Bid Depth: {:.2}, Ask Depth: {:.2}, Ratio: {:.2}",
                        symbol.to_uppercase(),
                        bid_depth,
                        ask_depth,
                        if ask_depth > 0.0 { bid_depth / ask_depth } else { 0.0 }
                    );
                    
                    // Update previous values
                    prev_best_bids.insert(symbol.to_string(), best_bid);
                    prev_best_asks.insert(symbol.to_string(), best_ask);
                }
            }
        }
    }

    info!("Order Book Monitor stopped");
    Ok(())
}

/// Calculate the cumulative depth at a certain percentage from the best price
fn calculate_depth(orderbook: &OrderBook, is_bid: bool, percent_threshold: f64) -> f64 {
    let levels = if is_bid { &orderbook.bids } else { &orderbook.asks };
    
    if levels.is_empty() {
        return 0.0;
    }
    
    let best_price = if is_bid {
        levels.first().unwrap().price
    } else {
        levels.first().unwrap().price
    };
    
    let threshold_price = if is_bid {
        best_price * (1.0 - percent_threshold)
    } else {
        best_price * (1.0 + percent_threshold)
    };
    
    let mut cumulative_quantity = 0.0;
    
    for level in levels {
        let price = level.price;
        if (is_bid && price >= threshold_price) || (!is_bid && price <= threshold_price) {
            cumulative_quantity += level.quantity;
        }
    }
    
    cumulative_quantity
} 