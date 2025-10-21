# 🔧 Percolator Initialization Scripts

Scripts to initialize Percolator programs on Solana devnet.

---

## 🚀 Quick Start

### **1. Install Dependencies**

```bash
cd scripts
npm install
```

### **2. Run Slab Initialization**

```bash
npm run init-slab
```

This will:
- ✅ Create a new Slab account (10 MB)
- ✅ Initialize it with trading parameters
- ✅ Save the account address to `slab-account.json`
- ✅ Show you the transaction on Solana Explorer

---

## 📋 **What the Script Does:**

1. **Connects to Devnet** - `https://api.devnet.solana.com`
2. **Creates/Loads Payer** - Generates keypair or loads from `slab-payer.json`
3. **Requests Airdrop** - Gets 2 SOL if balance is low
4. **Creates Slab Account** - Allocates 10 MB of space
5. **Initializes Slab** - Calls the Initialize instruction with:
   - IMR: 5% (initial margin ratio)
   - MMR: 3% (maintenance margin ratio)
   - Maker fee: -0.02% (rebate)
   - Taker fee: 0.05%
   - Batch window: 100ms
   - Freeze levels: 3

---

## 📦 **Output Files:**

- **`slab-payer.json`** - Keypair used to pay for initialization (keep this safe!)
- **`slab-account.json`** - Contains the Slab account address and transaction details

---

## 🔗 **After Initialization:**

Update your backend to use the initialized Slab account:

1. Open the generated `slab-account.json`
2. Copy the `slabAccount` address
3. Update `api/src/services/transactions.ts`:

```typescript
// Add this constant
export const SLAB_ACCOUNT = new PublicKey('YOUR_SLAB_ACCOUNT_FROM_JSON');
```

4. Rebuild the backend:
```bash
cd api
npm run build
npm run dev
```

---

## ⚠️ **Important Notes:**

- **Requires ~2 SOL** - Script will request airdrop automatically
- **Devnet only** - This script is for testing on devnet
- **Keep keypair safe** - The `slab-payer.json` has your test SOL
- **One-time setup** - Only need to run once per deployment

---

## 🐛 **Troubleshooting:**

### **"Insufficient balance" error:**
Get SOL from the faucet: https://faucet.solana.com/

### **"Transaction failed" error:**
Check the logs in the terminal for detailed error messages.

### **"Module not found" error:**
Make sure you ran `npm install` in the scripts directory.

---

## 🎯 **Next Steps:**

After running this script:

1. ✅ Slab will be initialized
2. ✅ Update backend with Slab account address
3. ✅ Test Reserve/Commit on dashboard
4. ✅ See real transactions on Solana Explorer!

---

## 📝 **Script Details:**

The initialization script:
- Uses **raw instruction building** (no Anchor needed)
- Creates a **10 MB account** for the full order book state
- Sets up **anti-toxicity parameters** (batch auctions, freeze levels)
- Saves all details for easy reference

---

**Built with:** TypeScript, @solana/web3.js, ts-node

