// Package orderbook provides common structures and interfaces for working with order books
package orderbook

import (
	"time"
)

// PriceLevel represents a single level in the order book with price and quantity
type PriceLevel struct {
	Price    float64
	Quantity float64
}

// OrderBookLevel represents a single level in the order book with both bid and ask sides
type OrderBookLevel struct {
	Bid PriceLevel
	Ask PriceLevel
}

// OrderBook represents an order book with bids and asks
type OrderBook struct {
	Symbol     string
	LastUpdate time.Time
	Bids       []PriceLevel
	Asks       []PriceLevel
}

// OrderBookSource is an interface for any exchange or data source that can provide order book data
type OrderBookSource interface {
	// GetSymbol returns the trading pair symbol
	GetSymbol() string
	
	// Connect establishes a connection to the exchange
	Connect() error
	
	// Disconnect closes the connection to the exchange
	Disconnect() error
	
	// SubscribeOrderBook subscribes to order book updates for the configured symbol
	SubscribeOrderBook() (<-chan OrderBook, error)
}

// GetTopLevels returns the top N levels of the order book
func (ob *OrderBook) GetTopLevels(n int) []OrderBookLevel {
	result := make([]OrderBookLevel, 0, n)
	
	// Calculate the number of levels to include
	levels := n
	if len(ob.Bids) < levels {
		levels = len(ob.Bids)
	}
	if len(ob.Asks) < levels {
		levels = len(ob.Asks)
	}
	
	// Create combined levels
	for i := 0; i < levels; i++ {
		level := OrderBookLevel{
			Bid: ob.Bids[i],
			Ask: ob.Asks[i],
		}
		result = append(result, level)
	}
	
	return result
} 