use crate::api::{BinanceClient, OrderBook, OrderBookEvent, OrderBookUpdateType};
use std::error::Error;
use std::time::Duration;
use tokio::select;
use tokio::time::Instant;
use tracing::{info, warn};

/// Example of using the order book stream
pub async fn run_orderbook_stream_example() -> Result<(), Box<dyn Error>> {
    info!("Starting OrderBook Stream Example");

    // Create a Binance client
    let client = BinanceClient::new().await?;

    // Trading pair to be monitored
    let symbol = "btcusdt";

    // Subscribe to updates
    client.subscribe(symbol).await?;
    info!("Subscribed to {} orderbook updates", symbol);
    
    // Now get the stream of updates
    let mut orderbook_rx = client.orderbook_stream(symbol)?;
    info!("Obtained orderbook stream for {}", symbol);
    
    // Track update statistics
    let mut total_updates = 0;
    let mut snapshot_updates = 0;
    let mut delta_updates = 0;
    let mut last_bid_price = 0.0;
    let mut last_ask_price = 0.0;
    let mut max_bid_delta = 0.0;
    let mut max_ask_delta = 0.0;
    let mut updates_per_second = 0;
    let mut last_second = Instant::now();
    
    info!("Listening for orderbook updates. Press Ctrl+C to exit.");
    
    // Timer for statistics and reset
    let mut stats_timer = tokio::time::interval(Duration::from_secs(1));
    
    loop {
        select! {
            // Receive an order book update
            Ok(event) = orderbook_rx.recv() => {
                info!("Received orderbook event: {:?}", event.event_type);
                process_orderbook_update(
                    &event, 
                    &mut total_updates,
                    &mut snapshot_updates,
                    &mut delta_updates,
                    &mut last_bid_price,
                    &mut last_ask_price,
                    &mut max_bid_delta,
                    &mut max_ask_delta,
                    &mut updates_per_second
                );
            }
            
            // Every second, print statistics
            _ = stats_timer.tick() => {
                let elapsed = last_second.elapsed().as_secs_f64();
                info!("=== Orderbook Update Statistics ===");
                info!("Symbol: {}", symbol);
                info!("Total updates: {}", total_updates);
                info!("Snapshots: {}", snapshot_updates);
                info!("Deltas: {}", delta_updates);
                info!("Updates per second: {:.2}", updates_per_second as f64 / elapsed);
                if max_bid_delta > 0.0 {
                    info!("Max bid price delta: {:.8}", max_bid_delta);
                }
                if max_ask_delta > 0.0 {
                    info!("Max ask price delta: {:.8}", max_ask_delta);
                }
                info!("Current best bid: {:.2}", last_bid_price);
                info!("Current best ask: {:.2}", last_ask_price);
                if last_bid_price > 0.0 && last_ask_price > 0.0 {
                    let spread = last_ask_price - last_bid_price;
                    let spread_pct = (spread / last_bid_price) * 100.0;
                    info!("Current spread: {:.2} ({:.6}%)", spread, spread_pct);
                }
                info!("===================================");
                
                // Reset counters for the next interval
                updates_per_second = 0;
                max_bid_delta = 0.0;
                max_ask_delta = 0.0;
                last_second = Instant::now();
            }
            
            // Handle Ctrl+C signal
            _ = tokio::signal::ctrl_c() => {
                info!("Received shutdown signal. Exiting...");
                break;
            }
        }
    }

    info!("OrderBook Stream Example finished");
    Ok(())
}

/// Process order book update
fn process_orderbook_update(
    event: &OrderBookEvent,
    total_updates: &mut u64,
    snapshot_updates: &mut u64,
    delta_updates: &mut u64,
    last_bid_price: &mut f64,
    last_ask_price: &mut f64,
    max_bid_delta: &mut f64,
    max_ask_delta: &mut f64,
    updates_per_second: &mut u32
) {
    // Increment counters
    *total_updates += 1;
    *updates_per_second += 1;
    
    // Get the order book from the event
    let orderbook = &event.orderbook;
    
    // Analyze the update type
    match event.event_type {
        OrderBookUpdateType::Snapshot => {
            *snapshot_updates += 1;
            info!("Received orderbook snapshot with {} bids and {} asks",
                orderbook.bids.len(), orderbook.asks.len());
                
            // Save the current best prices
            if let Some(best_bid) = orderbook.bids.first() {
                *last_bid_price = best_bid.price;
            }
            
            if let Some(best_ask) = orderbook.asks.first() {
                *last_ask_price = best_ask.price;
            }
        },
        OrderBookUpdateType::Delta => {
            *delta_updates += 1;
            
            // Get the best prices after the update
            if let Some(best_bid) = orderbook.bids.first() {
                // Calculate the price change
                if *last_bid_price > 0.0 {
                    let delta = (best_bid.price - *last_bid_price).abs();
                    if delta > *max_bid_delta {
                        *max_bid_delta = delta;
                    }
                    
                    // If the change is significant, log it
                    if delta > 10.0 {
                        warn!("Large bid price change detected: {} -> {} (delta: {})",
                            *last_bid_price, best_bid.price, delta);
                    }
                }
                
                *last_bid_price = best_bid.price;
            }
            
            if let Some(best_ask) = orderbook.asks.first() {
                // Calculate the price change
                if *last_ask_price > 0.0 {
                    let delta = (best_ask.price - *last_ask_price).abs();
                    if delta > *max_ask_delta {
                        *max_ask_delta = delta;
                    }
                    
                    // If the change is significant, log it
                    if delta > 10.0 {
                        warn!("Large ask price change detected: {} -> {} (delta: {})",
                            *last_ask_price, best_ask.price, delta);
                    }
                }
                
                *last_ask_price = best_ask.price;
            }
            
            // Additional microstructure analysis
            analyze_microstructure(orderbook);
        }
    }
}

/// Analyze market microstructure
fn analyze_microstructure(orderbook: &OrderBook) {
    // Calculate the total volumes of bids and asks up to certain levels
    let bid_volume_level1 = calculate_volume(&orderbook.bids, 1);
    let ask_volume_level1 = calculate_volume(&orderbook.asks, 1);
    
    // Calculate the order book imbalance
    if bid_volume_level1 > 0.0 && ask_volume_level1 > 0.0 {
        let imbalance = bid_volume_level1 / (bid_volume_level1 + ask_volume_level1);
        
        // If there is a significant imbalance, log it
        if imbalance > 0.7 || imbalance < 0.3 {
            info!("Order book imbalance detected: {:.2}% bids vs {:.2}% asks",
                imbalance * 100.0, (1.0 - imbalance) * 100.0);
        }
    }
    
    // Analyze the "wall" in the order book (large volume at one level)
    detect_walls(orderbook);
}

/// Calculate volume up to a certain level
fn calculate_volume(levels: &[crate::api::OrderBookLevel], num_levels: usize) -> f64 {
    levels.iter()
        .take(num_levels)
        .map(|level| level.quantity)
        .sum()
}

/// Detect "walls" in the order book
fn detect_walls(orderbook: &OrderBook) {
    const WALL_THRESHOLD: f64 = 10.0; // Threshold for detecting a "wall" (in BTC)
    
    // Check for "walls" in bids
    for (_, bid) in orderbook.bids.iter().enumerate().take(10) {
        if bid.quantity > WALL_THRESHOLD {
            info!("Bid wall detected: {} BTC at price {}", bid.quantity, bid.price);
            break;
        }
    }
    
    // Check for "walls" in asks
    for (_, ask) in orderbook.asks.iter().enumerate().take(10) {
        if ask.quantity > WALL_THRESHOLD {
            info!("Ask wall detected: {} BTC at price {}", ask.quantity, ask.price);
            break;
        }
    }
} 