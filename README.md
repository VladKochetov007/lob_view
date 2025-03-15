# LOB View

A lightweight terminal application for displaying real-time order book data from cryptocurrency exchanges.

## Features

- Real-time order book data from Binance for BTC/USDT pair
- Colored terminal output showing top 10 levels of bid and ask prices
- Minimal dependencies
- Extensible design with interfaces for adding other exchanges

## Installation

Clone the repository and build the application:

```bash
git clone https://github.com/VladKochetov007/lob_view.git
cd lob_view
go mod tidy
make run
```

## Usage

Run the application to view BTC/USDT order book:

```bash
./lob_view
```

## Example Output

```
 Order Book - btc/usdt 
Last update: 2025-06-12T15:23:45.123Z+03:00

Bid Price      | Bid Qty    | Ask Price      | Ask Qty   
---------------------------------------------------------
84211.76000000 | 0.24183000 | 84211.77000000 | 7.83714000
84211.75000000 | 0.00070000 | 84211.78000000 | 0.00021000
84211.74000000 | 0.00041000 | 84211.79000000 | 0.00007000
84211.73000000 | 0.00023000 | 84211.99000000 | 0.00021000
84211.72000000 | 0.00014000 | 84212.00000000 | 0.05813000
84211.26000000 | 0.00007000 | 84212.12000000 | 0.00007000
84211.13000000 | 0.00007000 | 84212.24000000 | 0.00021000
84211.12000000 | 0.00007000 | 84212.25000000 | 0.07151000
84210.71000000 | 0.00030000 | 84212.26000000 | 0.35626000
84210.70000000 | 0.08415000 | 84212.33000000 | 0.10165000
```

## Project Structure

The project follows a clean architecture approach with the following components:

- `pkg/orderbook`: Common structures and interfaces for working with order books
- `pkg/exchanges`: Shared utilities for exchanges and display functionality
- `pkg/exchanges/binance`: Binance-specific implementation of the OrderBookSource interface
- `cmd/lob_view`: Main application entry point


## License

MIT License - see the LICENSE file for details.
