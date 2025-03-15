// Package exchanges provides common functionality for working with exchanges
package exchanges

import (
	"fmt"
	"strings"

	"github.com/VladKochetov007/lob_view/pkg/orderbook"
)

const (
	// Number of price levels to display
	DefaultDepth = 10
	
	// Terminal formatting
	Reset  = "\033[0m"
	Red    = "\033[31m"
	Green  = "\033[32m"
	Yellow = "\033[33m"
	Blue   = "\033[34m"
	Bold   = "\033[1m"
)

// DisplayOrderBook formats and displays the order book in a terminal
func DisplayOrderBook(ob orderbook.OrderBook, depth int) {
	if depth <= 0 {
		depth = DefaultDepth
	}
	
	// Более строгая проверка на качество данных
	if len(ob.Bids) == 0 || len(ob.Asks) == 0 {
		return // Пропускаем, если нет данных
	}
	
	// Пропускаем, если нет валидной цены в первом элементе
	if ob.Bids[0].Price <= 0 || ob.Asks[0].Price <= 0 {
		return
	}
	
	// Get the top levels
	levels := ob.GetTopLevels(depth)
	
	// Пропускаем, если нет достаточного количества уровней
	if len(levels) < 3 { // Требуем как минимум 3 уровня
		return
	}
	
	// Calculate maximum width for better formatting
	maxBidPrice := 0
	maxBidQty := 0
	maxAskPrice := 0
	maxAskQty := 0
	
	for _, level := range levels {
		bidPriceLen := len(fmt.Sprintf("%.8f", level.Bid.Price))
		bidQtyLen := len(fmt.Sprintf("%.8f", level.Bid.Quantity))
		askPriceLen := len(fmt.Sprintf("%.8f", level.Ask.Price))
		askQtyLen := len(fmt.Sprintf("%.8f", level.Ask.Quantity))
		
		if bidPriceLen > maxBidPrice {
			maxBidPrice = bidPriceLen
		}
		if bidQtyLen > maxBidQty {
			maxBidQty = bidQtyLen
		}
		if askPriceLen > maxAskPrice {
			maxAskPrice = askPriceLen
		}
		if askQtyLen > maxAskQty {
			maxAskQty = askQtyLen
		}
	}
	
	// Print header
	fmt.Printf("\n%s%s Order Book - %s %s\n", Bold, Blue, ob.Symbol, Reset)
	fmt.Printf("%sLast update: %s%s\n\n", Yellow, ob.LastUpdate.Format("2006-01-02T15:04:05.000Z07:00"), Reset)
	
	// Print header row
	bidPriceHeader := "Bid Price"
	bidQtyHeader := "Bid Qty"
	askPriceHeader := "Ask Price"
	askQtyHeader := "Ask Qty"
	
	fmt.Printf("%-*s | %-*s | %-*s | %-*s\n", 
		maxBidPrice, bidPriceHeader, 
		maxBidQty, bidQtyHeader, 
		maxAskPrice, askPriceHeader, 
		maxAskQty, askQtyHeader)
	
	// Print separator
	separator := strings.Repeat("-", maxBidPrice+maxBidQty+maxAskPrice+maxAskQty+9)
	fmt.Println(separator)
	
	// Print each level
	for _, level := range levels {
		fmt.Printf("%s%-*.*f%s | %-*.*f | %s%-*.*f%s | %-*.*f\n",
			Green, maxBidPrice, 8, level.Bid.Price, Reset,
			maxBidQty, 8, level.Bid.Quantity,
			Red, maxAskPrice, 8, level.Ask.Price, Reset,
			maxAskQty, 8, level.Ask.Quantity)
	}
	
	fmt.Println("")
}

// DisplayOrderBookContinuously continuously displays the order book updates
func DisplayOrderBookContinuously(updates <-chan orderbook.OrderBook, depth int) {
	// Устанавливаем минимальный размер ордербука для отображения
	const minLevels = 5 // Должно быть как минимум 5 уровней для отображения
	
	for ob := range updates {
		// Строгая проверка на полные данные
		if len(ob.Bids) < minLevels || len(ob.Asks) < minLevels {
			continue // Пропускаем неполные данные
		}
		
		// Очищаем терминал перед каждым выводом
		fmt.Print("\033[H\033[2J")
		
		// Выводим только один полный ордербук
		DisplayOrderBook(ob, depth)
	}
} 