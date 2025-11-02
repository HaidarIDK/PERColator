# 🚀 How to Start Frontend & Backend Servers

## 📋 **Quick Start (Copy & Paste)**

### **Option 1: Start Both Automatically** ✅

Open **PowerShell** in the project root and run:

```powershell
# Start backend in background
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd 'C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\api'; npm run dev"

# Start frontend in background
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd 'C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\frontend'; npm run dev"
```

---

### **Option 2: Manual (2 Separate Terminals)** 🖥️

#### **Terminal 1 - Backend API:**
```powershell
cd C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\api
npm run dev
```

**Expected Output:**
```
> percolator-api@1.0.0 dev
> tsx watch src/index.ts

[API] Starting Percolator API...
[API] Server running on http://localhost:5001
[WS] WebSocket server listening on ws://localhost:5001/ws
```

#### **Terminal 2 - Frontend:**
```powershell
cd C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\frontend
npm run dev
```

**Expected Output:**
```
  ▲ Next.js 15.x.x
  - Local:        http://localhost:3000
  - Network:      http://192.168.x.x:3000

 ✓ Starting...
 ✓ Ready in 3.2s
```

---

## 🌐 **Access Your App**

Once both servers are running:

| Service | URL | Purpose |
|---------|-----|---------|
| **Frontend** | http://localhost:3000 | Main website |
| **Dashboard** | http://localhost:3000/dashboard | Trading interface |
| **Create Slab** | http://localhost:3000/create-slab | Create trading pairs |
| **Backend API** | http://localhost:5001 | Chart & orderbook data |

---

## 🛑 **How to Stop Servers**

### **If running in terminals:**
- Press **Ctrl + C** in each terminal window

### **If running as background processes:**
```powershell
# Kill all node processes (⚠️ stops ALL node apps)
Get-Process node -ErrorAction SilentlyContinue | Stop-Process -Force

# Or find and kill specific ones
Get-Process | Where-Object {$_.MainWindowTitle -like "*npm*"} | Stop-Process
```

---

## 🔄 **Restart Servers**

If you make code changes:

### **Backend (API):**
- It auto-restarts on file changes (using `tsx watch`)
- Or manually: **Ctrl + C** → `npm run dev`

### **Frontend:**
- It auto-reloads on file changes (Next.js Fast Refresh)
- Or manually: **Ctrl + C** → `npm run dev`

---

## ⚠️ **Troubleshooting**

### **Port Already in Use:**
```
Error: listen EADDRINUSE: address already in use :::3000
```

**Solution:**
```powershell
# Kill process on port 3000
netstat -ano | findstr :3000
taskkill /PID <PID_NUMBER> /F

# Kill process on port 5001
netstat -ano | findstr :5001
taskkill /PID <PID_NUMBER> /F
```

### **Module Not Found:**
```
Error: Cannot find module 'express'
```

**Solution:**
```powershell
# Reinstall dependencies
cd C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\api
npm install

cd C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\frontend
npm install
```

### **Connection Refused:**
```
Failed to fetch http://localhost:5001/api/...
```

**Solution:**
- Make sure **backend is running** (Terminal 1)
- Check backend logs for errors
- Verify `http://localhost:5001` shows API info

---

## 📊 **What Each Server Does**

### **Backend (Port 5001):**
```
┌──────────────────────────────────────┐
│  Hyperliquid API Integration         │
│    ↓                                  │
│  Fetch SOL/ETH/BTC chart data        │
│    ↓                                  │
│  WebSocket real-time updates         │
│    ↓                                  │
│  Serve to frontend via REST API      │
└──────────────────────────────────────┘
```

**Endpoints:**
- `GET /api/hyperliquid/candles` - Historical price data
- `GET /api/hyperliquid/current-price` - Latest price
- `GET /api/slab/orderbook` - Orderbook data
- `WS /ws` - Real-time updates

### **Frontend (Port 3000):**
```
┌──────────────────────────────────────┐
│  Next.js React App                   │
│    ↓                                  │
│  Dashboard Page (charts, orders)     │
│    ↓                                  │
│  Create Slab Page                    │
│    ↓                                  │
│  Solana Wallet Integration           │
└──────────────────────────────────────┘
```

**Pages:**
- `/` - Landing page
- `/dashboard` - Trading interface
- `/create-slab` - Create slabs & LP

---

## 🔧 **Development Tips**

### **View Backend Logs:**
```powershell
# Terminal 1 shows all API logs
[API] Server running...
[WS] Client connected
[API] GET /api/hyperliquid/candles?coin=SOL
```

### **View Frontend Logs:**
```powershell
# Terminal 2 shows Next.js logs
✓ Compiled successfully
⚠ Fast Refresh rebuilding
✓ Ready in 284ms
```

### **Check if Servers are Running:**
```powershell
# Check backend
curl http://localhost:5001

# Check frontend
curl http://localhost:3000
```

---

## 📁 **Project Structure**

```
percolator-v2/
├── website/
│   ├── api/                    ← Backend (Port 5001)
│   │   ├── src/
│   │   │   ├── index.ts       ← Main server
│   │   │   ├── routes/        ← API endpoints
│   │   │   └── services/      ← Business logic
│   │   └── package.json
│   │
│   └── frontend/               ← Frontend (Port 3000)
│       ├── src/
│       │   ├── app/           ← Pages
│       │   ├── components/    ← UI components
│       │   └── lib/           ← Utilities
│       └── package.json
│
└── programs/                   ← Solana programs (Rust)
```

---

## ✅ **Checklist**

Before accessing your app:

- [ ] Backend running on http://localhost:5001
- [ ] Frontend running on http://localhost:3000
- [ ] No error messages in either terminal
- [ ] Wallet extension installed (Phantom/Solflare)
- [ ] Connected to Solana Devnet

---

## 🎯 **Summary Commands**

```powershell
# Start backend
cd C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\api
npm run dev

# Start frontend (new terminal)
cd C:\Users\7haid\OneDrive\Desktop\percolator-v2\website\frontend
npm run dev

# Visit app
start http://localhost:3000/dashboard
```

---

**Both servers are now running! Open http://localhost:3000/dashboard to start trading! 🚀📈✨**

