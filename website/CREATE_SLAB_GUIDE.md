# 🎯 Create Slab & LP Management Guide

## Overview

The **Create Slab** page allows users to:
1. ✅ **Create their own trading pairs** (slabs)
2. ✅ **Provide liquidity** as a market maker
3. ✅ **Earn trading fees** from their liquidity

Access it from the dashboard via the **"Create Slab"** button in the header.

---

## 📍 Page Location

- **URL:** `/create-slab`
- **File:** `website/frontend/src/app/create-slab/page.tsx`
- **Access:** Click "Create Slab" button on dashboard header (next to "Home")

---

## 🎨 Features

### **1. Create New Slab** (Left Panel)

Users can launch their own trading pair with:

- **Base Currency:** e.g., MEME, DOGE, PEPE, etc.
- **Quote Currency:** USDC (fixed)
- **Mark Price:** Initial price in USD
- **Tick Size:** Price increment (e.g., $1.00)
- **Lot Size:** Quantity increment (e.g., 1.0 SOL)

**Cost:** ~0.46 SOL (rent-exempt, refundable)

**What Happens:**
1. Generates new slab account keypair
2. Calculates rent (65536 bytes)
3. Creates account on Solana
4. Initializes slab with parameters
5. User becomes the LP owner
6. Slab address is displayed

---

### **2. Provide Liquidity** (Right Panel)

Users can add liquidity to any slab:

- **Slab Address:** Enter manually or select from created slabs
- **Amount:** How much SOL to provide
- **Price:** At what price level
- **Side:** Automatically set to Buy (can be extended)

**What Happens:**
1. Creates a PlaceOrder instruction (as LP/maker)
2. Posts order to the slab
3. Order sits in orderbook as resting liquidity
4. Earns fees when matched

---

## 🔧 Technical Details

### **Create Slab Flow:**

```
User fills form
      ↓
Generate new keypair for slab
      ↓
Calculate rent (65536 bytes = ~0.46 SOL)
      ↓
Create account instruction
      ↓
Initialize slab instruction:
  - Discriminator: 0
  - Mark price (i64, scaled by 1e6)
  - Fees (u64, 0.1% = 1000000)
  - Contract size (u64)
  - LP owner (Pubkey - user)
  - Router ID (Pubkey - router program)
  - Instrument (u32)
  - Tick size (u64)
  - Lot size (u64)
  - Min order size (u64)
      ↓
Sign with both keypairs (user + slab)
      ↓
Send transaction
      ↓
Confirm on-chain
      ↓
Display slab address
```

### **Provide Liquidity Flow:**

```
User selects slab + fills form
      ↓
Scale amount and price (× 1e6)
      ↓
Create PlaceOrder instruction:
  - Discriminator: 3
  - Side: 0 (Buy) or 1 (Sell)
  - Price (i64, scaled)
  - Quantity (i64, scaled)
      ↓
Send to slab program
      ↓
Confirm on-chain
      ↓
Order appears in orderbook
```

---

## 📊 Example Use Cases

### **Use Case 1: Create Meme Coin Slab**
```
Base Currency: BONK
Mark Price: $0.000025
Tick Size: $0.000001
Lot Size: 1000000.0
```
Result: Users can trade BONK/USDC with micro-penny precision

### **Use Case 2: Provide Liquidity to SOL Slab**
```
Slab: 7pyCSG18qBXipgUCApPxBo48T3a1M7arhrf1AbkMtAFL
Amount: 10.0 SOL
Price: $185.00
```
Result: 10 SOL sell order at $185, earns fees when hit

---

## 🎯 Benefits

### **For Slab Creators:**
- ✅ **Own the trading pair** (LP owner privileges)
- ✅ **Set custom parameters** (tick/lot sizes)
- ✅ **Earn fees** from all trades on your slab
- ✅ **Fully on-chain** and trustless

### **For Liquidity Providers:**
- ✅ **Earn trading fees** (0.1% per trade)
- ✅ **No impermanent loss** (limit orders, not AMM)
- ✅ **Full control** (cancel orders anytime)
- ✅ **Transparent** (all orders on blockchain)

---

## 🚀 Integration with Dashboard

The Create Slab page is **fully integrated** with the dashboard:

1. **Header Button:** Green "Create Slab" button
2. **Wallet Connection:** Shares wallet state
3. **Toast Notifications:** Uses same system
4. **Config Management:** Reads from `program-config.ts`
5. **Consistent Styling:** Matches dashboard theme

---

## 💡 Future Enhancements

Potential additions:
- [ ] View all created slabs (user's portfolio)
- [ ] Analytics per slab (volume, fees earned)
- [ ] Advanced LP tools (spread orders, grid trading)
- [ ] Slab settings page (halt/resume trading)
- [ ] Transfer LP ownership
- [ ] Close slab (recover rent)

---

## 🔑 Key Files

- **Page:** `website/frontend/src/app/create-slab/page.tsx`
- **Config:** `website/frontend/src/lib/program-config.ts`
- **Slab Program:** `programs/slab/src/instructions/initialize.rs`
- **PlaceOrder:** `programs/slab/src/instructions/place_order.rs`

---

## 🎉 Result

Users can now:
1. **Create custom trading pairs** in seconds
2. **Provide liquidity** and earn fees
3. **Build their own mini-exchange** on Solana
4. **All fully decentralized** and on-chain!

---

**Your platform now supports permissionless market creation! 🚀📈✨**


