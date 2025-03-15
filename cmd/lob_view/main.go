// Command lob_view displays the top levels of an order book in real-time
package main

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/VladKochetov007/lob_view/pkg/exchanges"
	"github.com/VladKochetov007/lob_view/pkg/exchanges/binance"
)

func main() {
	// Фиксированные параметры для простоты примера
	symbol := "BTC/USDT"
	depth := 10

	fmt.Printf("Starting LOB Viewer for %s\n", symbol)
	fmt.Printf("Displaying top %d levels\n", depth)
	
	// Create Binance order book provider
	provider := binance.NewBinanceOrderBookProvider(symbol)
	
	// Connect to the exchange
	fmt.Println("Connecting to Binance...")
	if err := provider.Connect(); err != nil {
		fmt.Printf("Error connecting to Binance: %v\n", err)
		os.Exit(1)
	}
	
	// Setup clean shutdown
	defer provider.Disconnect()
	
	// Subscribe to order book updates
	updates, err := provider.SubscribeOrderBook()
	if err != nil {
		fmt.Printf("Error subscribing to order book: %v\n", err)
		os.Exit(1)
	}
	
	// Setup signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	
	// Start displaying the order book
	fmt.Println("Waiting for order book data...")
	
	// Небольшая задержка для получения стабильных данных
	time.Sleep(2 * time.Second)
	
	// Run in a separate goroutine so we can handle signals
	go exchanges.DisplayOrderBookContinuously(updates, depth)
	
	// Wait for termination signal
	<-sigChan
	fmt.Println("\nShutting down...")
} 