# 🔧 Initialize Using Your Phantom Wallet

You already have **~9 SOL** in your Phantom wallet! Let's use that instead.

---

## 📋 **Option 1: Export Phantom Private Key** (Easiest)

### **Steps:**

1. **Open Phantom Wallet**
2. **Go to Settings** → **Security & Privacy**
3. **Click "Export Private Key"**
4. **Enter your password**
5. **Copy the private key** (it's an array of numbers)

### **Create a file:** `scripts/phantom-keypair.json`

Paste the private key array (should look like):
```json
[123,45,67,89,...]
```

### **Update the script:**

Change line 29-30 in `initialize-slab.ts`:
```typescript
const KEYPAIR_PATH = 'phantom-keypair.json';  // Use your Phantom key
```

### **Run it:**
```bash
npm run init-slab
```

---

## 📋 **Option 2: Use Solana CLI** (If installed in WSL)

```bash
# In WSL
solana-keygen new --outfile slab-payer.json --force
solana airdrop 2 --url devnet
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator/scripts
npm run init-slab
```

---

## 📋 **Option 3: Web Faucet** (Current Method)

1. Go to: https://faucet.solana.com/
2. Paste: `4kY63cS5dn7bH7p2EJyVD3yetJvKs1nko4ZpWuDKkDPX`
3. Get 2 SOL
4. Run: `npm run init-slab`

---

## 💡 **Recommended:**

**Use the web faucet!** It's:
- ✅ Simple and safe
- ✅ Doesn't require exporting keys
- ✅ Just takes a minute

Then you can initialize and start **real trading**! 🚀

