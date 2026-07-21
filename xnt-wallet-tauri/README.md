# Neptune Wallet - Tauri Desktop Application

A desktop wallet application for the Neptune blockchain, built with Tauri, Vue 3, and TypeScript.

## Features

- **Secure Wallet Management**: Create, import, and manage Neptune wallets
- **Transaction Building**: Build and broadcast transactions
- **UTXO Management**: View and manage your UTXOs
- **Native Performance**: Rust backend for optimal performance
- **Cross-Platform**: Runs on Windows, macOS, and Linux

## Prerequisites

- Node.js 18+ and npm
- Rust (latest stable version)
- System dependencies for Tauri (see [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites))

## Development

### Install Dependencies

```bash
npm install
```

### Run Development Server

```bash
npm run tauri dev
```

This will:
1. Start the Vite dev server for the frontend
2. Compile and run the Tauri backend
3. Open the application window

### Build for Production

```bash
npm run tauri build
```

This creates platform-specific installers in `src-tauri/target/release/`.

## Project Structure

```
Neptune-wallet-tauri/
├── src/                    # Vue 3 frontend
│   ├── views/             # Page components
│   ├── components/        # Reusable components
│   ├── stores/            # Pinia stores
│   ├── composables/       # Vue composables
│   ├── api/               # API client
│   └── router/            # Vue Router
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── lib.rs         # Main Tauri commands
│   │   ├── keystore.rs    # Keystore management
│   │   ├── wallet.rs      # Wallet operations
│   │   └── transaction.rs # Transaction building
│   └── Cargo.toml         # Rust dependencies
└── package.json           # Node.js dependencies
```

## Configuration

### API Endpoint

Set the `VITE_APP_API` environment variable to configure the RPC endpoint:

```bash
export VITE_APP_API=http://your-node:8080
```

Or create a `.env` file:

```
VITE_APP_API=http://your-node:8080
```

## Native Module Integration

This project uses the Neptune core library directly in Rust, providing the same functionality as the native Node.js module but integrated into the Tauri backend.

## Building from Source

1. Clone the repository
2. Install dependencies: `npm install`
3. Build: `npm run tauri build`


## Build APK app 

run script : 
```bash
scripts/build-android.sh
```


## License

Apache-2.0
