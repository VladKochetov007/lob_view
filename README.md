# LOB View

A lightweight terminal application for displaying real-time order book data from cryptocurrency exchanges.

## Features

- Real-time order book data from Binance
- Colored terminal output showing top N levels of bid and ask prices
- Minimal dependencies
- Extensible design with interfaces for adding other exchanges

## Installation

Clone the repository and build the application:

```bash
git clone https://github.com/VladKochetov007/lob_view.git
cd lob_view
go mod tidy
go build -o lob_view cmd/lob_view/main.go
```

## Usage

Run the application with default settings (BTC/USDT, 10 levels):

```bash
./lob_view
```

Or specify a different trading pair and/or depth:

```bash
./lob_view -symbol ETH/USDT -depth 15
```

## Command Line Options

- `-symbol`: Trading pair symbol (default: "BTC/USDT")
- `-depth`: Number of order book levels to display (default: 10)

## License

MIT License - see the LICENSE file for details.

## Project Structure

The project follows a clean architecture approach with the following components:

- `pkg/orderbook`: Common structures and interfaces for working with order books
- `pkg/exchanges`: Shared utilities for exchanges and display functionality
- `pkg/exchanges/binance`: Binance-specific implementation of the OrderBookSource interface
- `cmd/lob_view`: Main application entry point

## Architecture

The application is designed around interfaces to allow for easy extension to support other exchanges:

```
OrderBookSource (Interface)
     |
     +--- BinanceOrderBookProvider (Implementation)
     |
     +--- [Future: Other Exchange Providers]
```

Each exchange provider implements the OrderBookSource interface, which standardizes how order book data is retrieved and processed regardless of the specific exchange API details.

## Future Enhancements

- Support for additional exchanges (e.g., Coinbase, Kraken)
- WebSocket reconnection logic
- More detailed order book statistics
- Order book visualization with charts
- WebSocket server for browser-based visualization
