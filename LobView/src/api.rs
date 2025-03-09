use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info};
use url::Url;

/// Binance API Errors
#[derive(Error, Debug)]
pub enum BinanceError {
    #[error("WebSocket error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("JSON serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),
    #[error("Channel error")]
    ChannelError,
}

/// Order Book Level with price and quantity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookLevel {
    pub price: f64,
    pub quantity: f64,
}

/// Full Order Book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub symbol: String,
    pub last_update_id: u64,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}

/// Binance REST API response format for order book snapshot request
#[derive(Debug, Deserialize)]
struct BinanceOrderBookSnapshot {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    bids: Vec<[String; 2]>,
    asks: Vec<[String; 2]>,
}

/// Binance WebSocket depth update message format
#[derive(Debug, Deserialize)]
struct BinanceDepthUpdate {
    #[serde(rename = "e")]
    event_type: String,
    #[serde(rename = "E")]
    event_time: u64,
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "b")]
    bids: Vec<[String; 2]>,
    #[serde(rename = "a")]
    asks: Vec<[String; 2]>,
}

/// WebSocket subscription request
#[derive(Debug, Serialize)]
struct SubscribeRequest {
    method: String,
    params: Vec<String>,
    id: u64,
}

/// Binance API Client
pub struct BinanceClient {
    orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
    http_client: reqwest::Client,
    ws_sender: mpsc::Sender<String>,
}

impl BinanceClient {
    /// Create a new Binance client
    pub async fn new() -> Result<Self, BinanceError> {
        let orderbooks = Arc::new(Mutex::new(HashMap::new()));
        let http_client = reqwest::Client::new();
        
        // Create a channel for symbol subscriptions
        let (ws_sender, mut ws_receiver) = mpsc::channel::<String>(100);
        
        // Start WebSocket processing in the background
        let orderbooks_clone = orderbooks.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::run_websocket(orderbooks_clone, &mut ws_receiver).await {
                error!("WebSocket error: {}", e);
            }
        });
        
        Ok(Self {
            orderbooks,
            http_client,
            ws_sender,
        })
    }
    
    /// Subscribe to order book updates for a symbol
    pub async fn subscribe(&self, symbol: &str) -> Result<(), BinanceError> {
        if symbol.is_empty() {
            return Err(BinanceError::InvalidSymbol(symbol.to_string()));
        }
        
        // First, get the order book snapshot
        let snapshot = self.fetch_orderbook_snapshot(symbol).await?;
        
        // Save the snapshot in the storage
        {
            let mut orderbooks = self.orderbooks.lock().unwrap();
            orderbooks.insert(symbol.to_string(), snapshot);
        }
        
        // Subscribe to updates via WebSocket
        self.ws_sender
            .send(symbol.to_string())
            .await
            .map_err(|_| BinanceError::ChannelError)
    }
    
    /// Get the current order book for a symbol
    pub fn get_orderbook(&self, symbol: &str) -> Option<OrderBook> {
        let orderbooks = self.orderbooks.lock().unwrap();
        orderbooks.get(symbol).cloned()
    }
    
    /// Start WebSocket connection and message processing
    async fn run_websocket(
        orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
        ws_receiver: &mut mpsc::Receiver<String>
    ) -> Result<(), BinanceError> {
        // Connect to Binance WebSocket API
        let url = Url::parse("wss://stream.binance.com:9443/ws")?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut ws_sender, mut ws_reader) = ws_stream.split();
        
        info!("Connected to Binance WebSocket");
        
        // Start a task to send subscription requests
        let mut active_subscriptions = Vec::new();
        
        // Main message processing loop
        loop {
            tokio::select! {
                // Process new subscription requests
                Some(symbol) = ws_receiver.recv() => {
                    // If the symbol is already in the subscriptions, skip
                    if active_subscriptions.contains(&symbol) {
                        continue;
                    }
                    
                    // Form a subscription request
                    let subscribe_req = SubscribeRequest {
                        method: "SUBSCRIBE".to_string(),
                        params: vec![format!("{}@depth@100ms", symbol.to_lowercase())],
                        id: 1,
                    };
                    
                    // Serialize and send the request
                    match serde_json::to_string(&subscribe_req) {
                        Ok(msg) => {
                            if let Err(e) = ws_sender.send(Message::Text(msg)).await {
                                error!("Failed to send subscription request: {}", e);
                            } else {
                                info!("Subscribed to {} order book", symbol);
                                active_subscriptions.push(symbol);
                            }
                        }
                        Err(e) => error!("Failed to serialize subscription request: {}", e),
                    }
                }
                
                // Process incoming WebSocket messages
                msg = ws_reader.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            Self::handle_ws_message(&text, &orderbooks).await;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                                error!("Failed to send pong: {}", e);
                            }
                        }
                        Some(Ok(_)) => {} // Ignore other message types
                        Some(Err(e)) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        None => {
                            info!("WebSocket connection closed");
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Handle WebSocket message
    async fn handle_ws_message(text: &str, orderbooks: &Arc<Mutex<HashMap<String, OrderBook>>>) {
        // Check if this is a depth update message
        if !text.contains("\"e\":\"depthUpdate\"") {
            return;
        }
        
        // Try to deserialize the message
        match serde_json::from_str::<BinanceDepthUpdate>(text) {
            Ok(update) => {
                // Update the order book
                let mut orderbooks_lock = orderbooks.lock().unwrap();
                
                if let Some(orderbook) = orderbooks_lock.get_mut(&update.symbol) {
                    // Check if this is a newer update
                    if update.final_update_id <= orderbook.last_update_id {
                        return;
                    }
                    
                    // Update the last update ID
                    orderbook.last_update_id = update.final_update_id;
                    
                    // Update the bids
                    Self::update_levels(&mut orderbook.bids, &update.bids, true);
                    
                    // Update the asks
                    Self::update_levels(&mut orderbook.asks, &update.asks, false);
                }
            }
            Err(e) => error!("Failed to parse depth update: {}", e),
        }
    }
    
    /// Update order book levels
    fn update_levels(levels: &mut Vec<OrderBookLevel>, updates: &[[String; 2]], is_bid: bool) {
        for update in updates {
            let price = update[0].parse::<f64>().unwrap_or(0.0);
            let quantity = update[1].parse::<f64>().unwrap_or(0.0);
            
            // If the quantity is 0, remove the level
            if quantity == 0.0 {
                levels.retain(|level| level.price != price);
                continue;
            }
            
            // Find the existing level to update
            let mut found = false;
            for level in levels.iter_mut() {
                if level.price == price {
                    level.quantity = quantity;
                    found = true;
                    break;
                }
            }
            
            // If the level is not found, add a new one
            if !found {
                levels.push(OrderBookLevel { price, quantity });
            }
        }
        
        // Sort the levels
        if is_bid {
            // Sort the bids in descending order
            levels.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        } else {
            // Sort the asks in ascending order
            levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        }
    }
    
    /// Fetch order book snapshot via REST API
    async fn fetch_orderbook_snapshot(&self, symbol: &str) -> Result<OrderBook, BinanceError> {
        let url = format!(
            "https://api.binance.com/api/v3/depth?symbol={}&limit=1000",
            symbol.to_uppercase()
        );
        
        info!("Fetching orderbook snapshot for {}", symbol);
        
        // Execute HTTP request
        let response = self.http_client
            .get(&url)
            .send()
            .await?
            .json::<BinanceOrderBookSnapshot>()
            .await?;
        
        // Convert the response to our OrderBook structure
        let mut orderbook = OrderBook {
            symbol: symbol.to_string(),
            last_update_id: response.last_update_id,
            bids: Vec::new(),
            asks: Vec::new(),
        };
        
        // Process the bids
        for bid in &response.bids {
            let price = bid[0].parse::<f64>().unwrap_or(0.0);
            let quantity = bid[1].parse::<f64>().unwrap_or(0.0);
            
            if quantity > 0.0 {
                orderbook.bids.push(OrderBookLevel { price, quantity });
            }
        }
        
        // Sort the bids in descending order
        orderbook.bids.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap());
        
        // Process the asks
        for ask in &response.asks {
            let price = ask[0].parse::<f64>().unwrap_or(0.0);
            let quantity = ask[1].parse::<f64>().unwrap_or(0.0);
            
            if quantity > 0.0 {
                orderbook.asks.push(OrderBookLevel { price, quantity });
            }
        }
        
        // Sort the asks in ascending order
        orderbook.asks.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        
        info!(
            "Fetched orderbook for {} with {} bids and {} asks",
            symbol,
            orderbook.bids.len(),
            orderbook.asks.len()
        );
        
        Ok(orderbook)
    }
}

/// Order Book Provider trait
#[async_trait]
pub trait OrderBookProvider {
    /// Subscribe to order book updates for a symbol
    async fn subscribe(&self, symbol: &str) -> Result<(), BinanceError>;
    
    /// Get the current order book for a symbol
    fn get_orderbook(&self, symbol: &str) -> Option<OrderBook>;
}

#[async_trait]
impl OrderBookProvider for BinanceClient {
    async fn subscribe(&self, symbol: &str) -> Result<(), BinanceError> {
        self.subscribe(symbol).await
    }
    
    fn get_orderbook(&self, symbol: &str) -> Option<OrderBook> {
        self.get_orderbook(symbol)
    }
}
