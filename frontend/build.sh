set -xeEu
ACCESS_TOKEN=your-secure-access-token-here-change-this-in-production VITE_API_ACCESS_TOKEN=your-secure-access-token-here-change-this-in-production bun run build:wasm
ACCESS_TOKEN=your-secure-access-token-here-change-this-in-production VITE_API_ACCESS_TOKEN=your-secure-access-token-here-change-this-in-production bun run build
