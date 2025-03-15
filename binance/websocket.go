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

func (c *BinanceClient) readMessages() {
	// Основной цикл чтения сообщений из WebSocket
}

func (c *BinanceClient) processMessage(msg []byte) {
	// Обработка и рассылка обновлений
} 