package main

import (
    "context"
    "fmt"
    "log"
    "time"
    
    lobview "github.com/VladKochetov007/lob_view"
)

func main() {
    // Инициализация клиента
    client := lobview.NewBinanceClient()
    defer client.Close()

    // Подписка на обновления
    symbol := "BTCUSDT"
    ch, err := client.Subscribe(symbol)
    if err != nil {
        log.Fatalf("Subscribe error: %v", err)
    }

    // Горутина для чтения обновлений
    ctx, cancel := context.WithCancel(context.Background())
    defer cancel()
    
    go func() {
        for {
            select {
            case <-ctx.Done():
                return
            case event, ok := <-ch:
                if !ok {
                    log.Println("Channel closed")
                    return
                }
                
                if event.Error != nil {
                    log.Printf("Error event: %v", event.Error)
                    continue
                }
                
                printOrderBook(event.OrderBook)
            }
        }
    }()

    // Ожидание и отписка
    time.Sleep(5 * time.Minute)
    client.Unsubscribe(symbol, ch)
    log.Println("Unsubscribed successfully")
}

func printOrderBook(ob lobview.OrderBook) {
    fmt.Printf("\n=== OrderBook %s [%v] ===\n", ob.Symbol, ob.Timestamp.Format(time.RFC3339))
    
    fmt.Println("Bids:")
    for i, bid := range ob.Bids {
        if i >= 5 { break }
        fmt.Printf("  %.2f @ %.4f\n", bid.Price, bid.Quantity)
    }
    
    fmt.Println("\nAsks:")
    for i, ask := range ob.Asks {
        if i >= 5 { break }
        fmt.Printf("  %.2f @ %.4f\n", ask.Price, ask.Quantity)
    }
    
    fmt.Println("========================")
} 