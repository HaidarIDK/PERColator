# 📁 Frontend Folder Dependencies

## **Direct Folder Usage**

Your frontend **ONLY uses 2 folders directly**:

### ✅ **1. Frontend Folder (Self-Contained)**
📂 **`C:\Users\7haid\OneDrive\Desktop\percolator\website\frontend\`**

**All code is here:**
- `website/frontend/src/app/` - All pages (dashboard, trade, portfolio, etc.)
- `website/frontend/src/components/` - UI components
- `website/frontend/src/lib/` - API client, utilities, configs
- `website/frontend/public/` - Static assets (images, icons)
- `website/frontend/package.json` - Dependencies
- `website/frontend/node_modules/` - NPM packages (auto-generated)

**No imports from other project folders!**

---

### ✅ **2. API Folder (Network Connection Only)**
📂 **`C:\Users\7haid\OneDrive\Desktop\percolator\website\api\`**

**How it's used:**
- ❌ **NOT imported as code** (no `import from '../api'`)
- ✅ **Connected via HTTP/WebSocket** at runtime
- **Connection:** `http://localhost:5001` (REST API)
- **Connection:** `ws://localhost:5001/ws` (WebSocket)

**The frontend makes HTTP requests to the API, but doesn't import any TypeScript/JavaScript files from it.**

---

## **What the Frontend Does NOT Use**

### ❌ **SDK Folder** (`sdk/typescript/`)
- **Status:** Referenced in documentation only
- **Not imported:** No `import from '../../../sdk/typescript'`
- **Purpose:** Separate package (could be published to NPM)

### ❌ **Programs Folder** (`programs/`)
- **Status:** Rust Solana programs
- **Not used directly:** Frontend only uses deployed program addresses
- **Accessed via:** Solana blockchain (on-chain), not local files

### ❌ **Scripts Folder** (`scripts/`)
- **Status:** Deployment/utility scripts
- **Not used by frontend:** Only used for deploying programs

### ❌ **CLI Folder** (`cli/`)
- **Status:** Command-line tools
- **Not used by frontend:** Separate application

### ❌ **Keeper Folder** (`keeper/`)
- **Status:** Backend service (Rust)
- **Not used by frontend:** Separate service

---

## **Frontend Architecture**

```
Frontend (Next.js App)
  │
  ├─ Self-Contained Code
  │   └─ frontend/src/ (all TypeScript/React code)
  │
  ├─ HTTP Connections (runtime)
  │   └─ api/ (backend server at localhost:5001)
  │
  ├─ Blockchain Connections (runtime)
  │   └─ Solana Network (via @solana/web3.js)
  │
  └─ NPM Packages (node_modules)
      └─ React, Next.js, Solana libraries, etc.
```

---

## **How Frontend Connects to API**

### **Not Direct Imports:**
```typescript
// ❌ Frontend does NOT do this:
import { something } from '../../api/src/...'
```

### **HTTP Requests Instead:**
```typescript
// ✅ Frontend does this instead:
const response = await fetch('http://localhost:5001/api/market/list')
const data = await response.json()

// Or via API client:
import { apiClient } from '@/lib/api-client'
const markets = await apiClient.getMarkets()
```

---

## **Environment Variables**

The frontend uses environment variables from `.env.local` (in `website/frontend/` folder):

```bash
NEXT_PUBLIC_API_URL=http://localhost:5001
NEXT_PUBLIC_WS_URL=ws://localhost:5001/ws
NEXT_PUBLIC_SOLANA_RPC_URL=https://api.devnet.solana.com
NEXT_PUBLIC_SLAB_PROGRAM_ID=...
NEXT_PUBLIC_ROUTER_PROGRAM_ID=...
# etc.
```

**Note:** These point to **deployed program addresses** and **API endpoints**, not local folders.

---

## **TypeScript Path Aliases**

The frontend uses path aliases (configured in `tsconfig.json`):

```json
{
  "paths": {
    "@/*": ["./src/*"]
  }
}
```

**This means:**
- `@/lib/api-client` → `website/frontend/src/lib/api-client`
- `@/components/ui/...` → `website/frontend/src/components/ui/...`
- **All paths resolve within the `website/frontend/` folder only**

---

## **Summary**

| Folder | Used? | How? |
|--------|-------|------|
| **`website/frontend/`** | ✅ **YES** | All source code lives here |
| **`website/api/`** | ✅ **YES** | HTTP/WebSocket connections (runtime) |
| **`sdk/typescript/`** | ❌ **NO** | Not imported |
| **`programs/`** | ❌ **NO** | Rust code, frontend only uses deployed addresses |
| **`scripts/`** | ❌ **NO** | Deployment scripts only |
| **`cli/`** | ❌ **NO** | Separate CLI app |
| **`keeper/`** | ❌ **NO** | Separate Rust service |

---

## **Answer to Your Question**

> "Does the frontend only use `api` and `frontend`?"

**Yes! Exactly:**

1. ✅ **`website/frontend/`** - Contains all frontend code
2. ✅ **`website/api/`** - Backend server (connected via HTTP/WebSocket, not imported)

**No other folders are used directly by the frontend code.**

The frontend is **completely self-contained** in the `website/frontend/` folder, and only connects to the API server at runtime via network requests.

