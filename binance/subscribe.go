package orderbook

import (
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"sync"
	"time"

	"github.com/gorilla/websocket"
)

type OrderBookLevel struct {
	Price    float64
	Quantity float64
}

type OrderBook struct {
	Symbol       string
	LastUpdateID int64
	Bids         []OrderBookLevel
	Asks         []OrderBookLevel
	Timestamp    time.Time
}

type OrderBookEvent struct {
	OrderBook OrderBook
	Symbol    string
	Error     error
}

type BinanceClient struct {
	wsURL       string
	restURL     string
	subscribers map[string][]chan OrderBookEvent
	mu          sync.RWMutex
	conn        *websocket.Conn
	httpClient  *HTTPClient
	ctx         context.Context
	cancel      context.CancelFunc
}

func NewBinanceClient() *BinanceClient {
	ctx, cancel := context.WithCancel(context.Background())
	return &BinanceClient{
		wsURL:       "wss://stream.binance.com:9443/ws",
		restURL:     "https://api.binance.com/api/v3/depth",
		subscribers: make(map[string][]chan OrderBookEvent),
		httpClient:  NewHTTPClient(),
		ctx:         ctx,
		cancel:      cancel,
	}
}

func (c *BinanceClient) Subscribe(symbol string) (<-chan OrderBookEvent, error) {
	// TODO: Implement
}

func (c *BinanceClient) Unsubscribe(symbol string, ch <-chan OrderBookEvent) {
	// TODO: Implement
}
