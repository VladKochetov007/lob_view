// Package binance provides implementation of OrderBookSource for Binance exchange
package binance

import (
	"encoding/json"
	"fmt"
	"github.com/VladKochetov007/lob_view/pkg/orderbook"
	"net/url"
	"strconv"
	"strings"
	"time"

	"github.com/gorilla/websocket"
)

const (
	binanceWsURL = "wss://stream.binance.com:9443/ws"
)

// BinanceOrderBookProvider implements the OrderBookSource interface for Binance
type BinanceOrderBookProvider struct {
	symbol     string
	conn       *websocket.Conn
	done       chan struct{}
	orderbooks chan orderbook.OrderBook
}

// BinanceOrderBookResponse represents the response from Binance WebSocket API
type BinanceOrderBookResponse struct {
	LastUpdateID int64       `json:"lastUpdateId"`
	Bids         [][2]string `json:"bids"`
	Asks         [][2]string `json:"asks"`
	Symbol       string      `json:"s"`
	EventTime    int64       `json:"E"`
}

// NewBinanceOrderBookProvider creates a new Binance order book provider
func NewBinanceOrderBookProvider(symbol string) *BinanceOrderBookProvider {
	return &BinanceOrderBookProvider{
		symbol:     strings.ToLower(symbol),
		done:       make(chan struct{}),
		orderbooks: make(chan orderbook.OrderBook, 100),
	}
}

// GetSymbol returns the trading pair symbol
func (p *BinanceOrderBookProvider) GetSymbol() string {
	return p.symbol
}

// Connect establishes a connection to Binance WebSocket API
func (p *BinanceOrderBookProvider) Connect() error {
	// Format symbol by removing slash if present
	formattedSymbol := strings.ReplaceAll(p.symbol, "/", "")
	
	// Create WebSocket connection
	u := url.URL{Scheme: "wss", Host: "stream.binance.com:9443", Path: "/ws/" + strings.ToLower(formattedSymbol) + "@depth20@100ms"}
	
	var err error
	p.conn, _, err = websocket.DefaultDialer.Dial(u.String(), nil)
	if err != nil {
		return fmt.Errorf("error connecting to Binance WebSocket: %w", err)
	}
	
	return nil
}

// Disconnect closes the WebSocket connection
func (p *BinanceOrderBookProvider) Disconnect() error {
	close(p.done)
	if p.conn != nil {
		return p.conn.Close()
	}
	return nil
}

// SubscribeOrderBook subscribes to order book updates
func (p *BinanceOrderBookProvider) SubscribeOrderBook() (<-chan orderbook.OrderBook, error) {
	go p.listenForUpdates()
	return p.orderbooks, nil
}

// listenForUpdates listens for WebSocket messages and processes them
func (p *BinanceOrderBookProvider) listenForUpdates() {
	defer close(p.orderbooks)
	
	for {
		select {
		case <-p.done:
			return
		default:
			_, message, err := p.conn.ReadMessage()
			if err != nil {
				fmt.Printf("Error reading message: %v\n", err)
				return
			}
			
			var response BinanceOrderBookResponse
			if err := json.Unmarshal(message, &response); err != nil {
				fmt.Printf("Error unmarshalling message: %v\n", err)
				continue
			}
			
			p.processOrderBookUpdate(response)
		}
	}
}

// processOrderBookUpdate processes an order book update from Binance
func (p *BinanceOrderBookProvider) processOrderBookUpdate(response BinanceOrderBookResponse) {
	// Create a new order book
	ob := orderbook.OrderBook{
		Symbol:     p.symbol,
		LastUpdate: time.Unix(0, response.EventTime*int64(time.Millisecond)),
		Bids:       make([]orderbook.PriceLevel, 0, len(response.Bids)),
		Asks:       make([]orderbook.PriceLevel, 0, len(response.Asks)),
	}
	
	// Process bids
	for _, bid := range response.Bids {
		price, err := strconv.ParseFloat(bid[0], 64)
		if err != nil {
			continue
		}
		
		quantity, err := strconv.ParseFloat(bid[1], 64)
		if err != nil {
			continue
		}
		
		ob.Bids = append(ob.Bids, orderbook.PriceLevel{
			Price:    price,
			Quantity: quantity,
		})
	}
	
	// Process asks
	for _, ask := range response.Asks {
		price, err := strconv.ParseFloat(ask[0], 64)
		if err != nil {
			continue
		}
		
		quantity, err := strconv.ParseFloat(ask[1], 64)
		if err != nil {
			continue
		}
		
		ob.Asks = append(ob.Asks, orderbook.PriceLevel{
			Price:    price,
			Quantity: quantity,
		})
	}
	
	// Send the order book to the channel
	p.orderbooks <- ob
} 