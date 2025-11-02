# 📊 Trading Parameters Explained (Simple Guide)

## 🎯 **The 3 Key Numbers**

When you create a slab, you set 3 important numbers that control how trading works:

```
┌─────────────────────────────────────────────┐
│  MARK PRICE    │  TICK SIZE   │  LOT SIZE   │
│    $43,000     │    $100      │    0.01     │
└─────────────────────────────────────────────┘
```

---

## 1️⃣ **MARK PRICE** (Starting Price)

### **What is it?**
The **initial price** of the asset in your orderbook.

### **BTC Example:**
- Mark Price = **$43,000**
- Meaning: **1 BTC costs $43,000 USDC**

### **Think of it like:**
🏪 A store price tag: "Bitcoin - $43,000 each"

### **Real Use:**
```
If mark price is $43,000:
  • Buy 0.5 BTC = Pay $21,500 USDC
  • Buy 1.0 BTC = Pay $43,000 USDC
  • Buy 2.0 BTC = Pay $86,000 USDC
```

---

## 2️⃣ **TICK SIZE** (Price Steps)

### **What is it?**
The **smallest price change** allowed between orders.

### **BTC Example:**
- Tick Size = **$100**
- Meaning: **Prices must be in $100 increments**

### **Think of it like:**
🪜 A ladder with $100 steps - you can't stand between the steps!

### **Valid Prices:**
```
✅ $43,000  (43,000 ÷ 100 = 430)
✅ $43,100  (43,100 ÷ 100 = 431)
✅ $43,200  (43,200 ÷ 100 = 432)
✅ $42,900  (42,900 ÷ 100 = 429)
```

### **Invalid Prices:**
```
❌ $43,050  (not a multiple of $100)
❌ $43,075  (not a multiple of $100)
❌ $43,150  (not a multiple of $100)
```

### **Why does this exist?**
- ✅ Prevents spam orders at tiny differences ($43,000.01, $43,000.02, etc.)
- ✅ Keeps orderbook clean and readable
- ✅ Reduces on-chain storage and gas costs
- ✅ Makes trading more predictable

---

## 3️⃣ **LOT SIZE** (Quantity Steps)

### **What is it?**
The **smallest amount** you can trade.

### **BTC Example:**
- Lot Size = **0.01 BTC**
- Meaning: **Orders must be multiples of 0.01 BTC**

### **Think of it like:**
📦 Buying eggs - you can only buy whole cartons (0.01 BTC each), not individual eggs!

### **Valid Quantities:**
```
✅ 0.01 BTC  (1 × 0.01)
✅ 0.02 BTC  (2 × 0.01)
✅ 0.50 BTC  (50 × 0.01)
✅ 1.00 BTC  (100 × 0.01)
✅ 2.37 BTC  (237 × 0.01)
```

### **Invalid Quantities:**
```
❌ 0.005 BTC  (too small)
❌ 0.015 BTC  (not a multiple of 0.01)
❌ 0.037 BTC  (not a multiple of 0.01)
❌ 1.234 BTC  (not a multiple of 0.01)
```

### **Why does this exist?**
- ✅ Prevents "dust" orders (tiny amounts like 0.00001 BTC)
- ✅ Reduces blockchain spam
- ✅ Makes order matching cleaner
- ✅ Saves gas/transaction fees

---

## 📚 **Complete Example: Buying Bitcoin**

### **Slab Settings:**
```
Pair:        BTC/USDC
Mark Price:  $43,000
Tick Size:   $100
Lot Size:    0.01 BTC
```

### **Scenario 1: Valid Order ✅**
```
You want to buy Bitcoin:
  Price:   $42,900  ✅ (multiple of $100)
  Amount:  0.05 BTC ✅ (5 × 0.01)
  
Calculate cost:
  0.05 BTC × $42,900 = $2,145 USDC
  
Result: ✅ ORDER ACCEPTED
```

### **Scenario 2: Invalid Price ❌**
```
You want to buy Bitcoin:
  Price:   $42,925  ❌ (NOT multiple of $100)
  Amount:  0.05 BTC ✅ (valid)
  
Result: ❌ ERROR - "Price not aligned to tick size"
```

### **Scenario 3: Invalid Quantity ❌**
```
You want to buy Bitcoin:
  Price:   $43,000  ✅ (valid)
  Amount:  0.037 BTC ❌ (NOT multiple of 0.01)
  
Result: ❌ ERROR - "Quantity not aligned to lot size"
```

---

## 🎯 **Quick Reference Table**

| Pair | Mark Price | Tick Size | Lot Size | Example Order |
|------|-----------|-----------|----------|---------------|
| **SOL/USDC** | $186 | $1 | 1.0 SOL | Buy 5 SOL at $185 = $925 |
| **ETH/USDC** | $2,800 | $10 | 0.1 ETH | Buy 0.5 ETH at $2,790 = $1,395 |
| **BTC/USDC** | $43,000 | $100 | 0.01 BTC | Buy 0.1 BTC at $42,900 = $4,290 |

---

## 💡 **Why Different Pairs Have Different Tick/Lot Sizes?**

### **SOL (Cheaper, More Volatile):**
```
Price: ~$186
Tick Size: $1.00  (0.5% of price)
Lot Size: 1.0 SOL (whole coins)

Reasoning: Sol is cheaper, so $1 steps are reasonable.
People typically trade whole SOL, not fractions.
```

### **ETH (Medium Price):**
```
Price: ~$2,800
Tick Size: $10.00  (0.35% of price)
Lot Size: 0.1 ETH (fractional)

Reasoning: ETH is expensive enough that $10 steps make sense.
People often trade 0.1 ETH chunks (~$280 each).
```

### **BTC (Expensive, Stable):**
```
Price: ~$43,000
Tick Size: $100.00  (0.23% of price)
Lot Size: 0.01 BTC (fractional)

Reasoning: BTC is very expensive, so $100 steps are fine.
People often trade 0.01 BTC chunks (~$430 each).
```

---

## 🔧 **Can I Change These Later?**

**Short Answer:** Not easily (requires creating new slab).

**Why?**
- These values are **hardcoded** into the slab when it's created
- Changing them would break existing orders
- All traders rely on these being consistent

**If you need different values:**
1. Create a **new slab** with new parameters
2. Provide liquidity to the new slab
3. Users can choose which slab to use

---

## 🎓 **Advanced: How Programs Use These**

### **Validation (On-Chain):**
```rust
// Check tick size alignment
if (price % tick_size) != 0 {
  return Err(PriceNotAlignedToTickSize);
}

// Check lot size alignment
if (quantity % lot_size) != 0 {
  return Err(QuantityNotAlignedToLotSize);
}
```

### **Frontend (TypeScript):**
```typescript
// Scale to fixed-point (1e6 precision)
const priceScaled = Math.floor(parseFloat(price) * 1_000_000);
const tickSizeScaled = Math.floor(parseFloat(tickSize) * 1_000_000);

// Send to blockchain
const instructionData = Buffer.alloc(17);
instructionData.writeUInt8(3, 0); // PlaceOrder discriminator
// ... write price, quantity, etc.
```

---

## 📖 **Summary**

```
Mark Price = Starting price ($43,000)
            ↓
         "How much?"

Tick Size = Price steps ($100)
           ↓
        "Price jumps"

Lot Size = Quantity steps (0.01 BTC)
          ↓
       "Order sizes"
```

**Together they ensure:**
- ✅ Clean orderbook (no spam)
- ✅ Predictable trading
- ✅ Efficient on-chain storage
- ✅ Lower transaction costs
- ✅ Better user experience

---

**Now you understand what those 3 numbers mean! 🎉📊✨**

