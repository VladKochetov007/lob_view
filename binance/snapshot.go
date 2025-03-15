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
	c.mu.Lock()
	defer c.mu.Unlock()

	symbol = normalizeSymbol(symbol)
	ch := make(chan OrderBookEvent, 100)
	c.subscribers[symbol] = append(c.subscribers[symbol], ch)

	if len(c.subscribers[symbol]) == 1 {
		if err := c.initWSConnection(); err != nil {
			return nil, err
		}
		go c.handleInitialSnapshot(symbol)
	}

	return ch, nil
}

func (c *BinanceClient) Unsubscribe(symbol string, ch <-chan OrderBookEvent) {
	c.mu.Lock()
	defer c.mu.Unlock()

	symbol = normalizeSymbol(symbol)
	subs := c.subscribers[symbol]
	for i, sub := range subs {
		if sub == ch {
			close(sub)
			c.subscribers[symbol] = append(subs[:i], subs[i+1:]...)
			break
		}
	}

	if len(c.subscribers[symbol]) == 0 {
		c.removeSubscription(symbol)
	}
}

func (c *BinanceClient) initWSConnection() error {
	if c.conn != nil {
		return nil
	}

	conn, _, err := websocket.DefaultDialer.Dial(c.wsURL, nil)
	if err != nil {
		return fmt.Errorf("websocket connection error: %w", err)
	}
	c.conn = conn

	go c.readMessages()
	return nil
}

func (c *BinanceClient) handleInitialSnapshot(symbol string) {
	ob, err := c.fetchSnapshot(symbol)
	event := OrderBookEvent{
		OrderBook: ob,
		Symbol:    symbol,
		Error:     err,
	}

	c.mu.RLock()
	defer c.mu.RUnlock()

	for _, ch := range c.subscribers[symbol] {
		select {
		case ch <- event:
		default:
			slog.Warn("subscriber channel full, dropping snapshot", "symbol", symbol)
		}
	}
}

func (c *BinanceClient) fetchSnapshot(symbol string) (OrderBook, error) {
	url := fmt.Sprintf("%s?symbol=%s&limit=1000", c.restURL, symbol)
	resp, err := c.httpClient.Get(url)
	if err != nil {
		return OrderBook{}, fmt.Errorf("failed to get snapshot: %w", err)
	}
	defer resp.Body.Close()

	var snapshot struct {
		LastUpdateID int64        `json:"lastUpdateId"`
		Bids         [][2]float64 `json:"bids"`
		Asks         [][2]float64 `json:"asks"`
	}

	if err := json.NewDecoder(resp.Body).Decode(&snapshot); err != nil {
		return OrderBook{}, fmt.Errorf("failed to decode snapshot: %w", err)
	}

	return OrderBook{
		Symbol:       symbol,
		LastUpdateID: snapshot.LastUpdateID,
		Bids:         convertLevels(snapshot.Bids),
		Asks:         convertLevels(snapshot.Asks),
		Timestamp:    time.Now().UTC(),
	}, nil
}

func (c *BinanceClient) readMessages() {
	defer c.conn.Close()
	
	for {
		select {
		case <-c.ctx.Done():
			return
		default:
			_, msg, err := c.conn.ReadMessage()
			if err != nil {
				c.notifyAllSubscribers(OrderBookEvent{Error: err})
				return
			}
			c.processMessage(msg)
		}
	}
}

func (c *BinanceClient) processMessage(msg []byte) {
	var update struct {
		Symbol string     `json:"s"`
		Bids   [][]string `json:"b"`
		Asks   [][]string `json:"a"`
	}
	
	if err := json.Unmarshal(msg, &update); err != nil {
		slog.Error("failed to unmarshal update", "error", err)
		return
	}

	ob := OrderBook{
		Symbol:    update.Symbol,
		Timestamp: time.Now().UTC(),
		Bids:      parseStringLevels(update.Bids),
		Asks:      parseStringLevels(update.Asks),
	}

	c.mu.RLock()
	defer c.mu.RUnlock()

	for _, ch := range c.subscribers[normalizeSymbol(ob.Symbol)] {
		select {
		case ch <- OrderBookEvent{OrderBook: ob}:
		default:
			slog.Warn("subscriber channel full, dropping update", "symbol", ob.Symbol)
		}
	}
}

func normalizeSymbol(symbol string) string {
	return strings.ToLower(symbol)
}

func convertLevels(levels [][2]float64) []OrderBookLevel {
	res := make([]OrderBookLevel, len(levels))
	for i, l := range levels {
		res[i] = OrderBookLevel{Price: l[0], Quantity: l[1]}
	}
	return res
}

func parseStringLevels(levels [][]string) []OrderBookLevel {
	res := make([]OrderBookLevel, 0, len(levels))
	for _, l := range levels {
		if len(l) != 2 {
			continue
		}
		price, _ := strconv.ParseFloat(l[0], 64)
		quantity, _ := strconv.ParseFloat(l[1], 64)
		res = append(res, OrderBookLevel{Price: price, Quantity: quantity})
	}
	return res
} 