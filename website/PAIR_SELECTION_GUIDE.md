# 📊 Trading Pair Selection Guide

## ✅ **Available Pairs (Restricted)**

The Create Slab page now **only allows 3 specific pairs**:

```
┌─────────────────────────────────────┐
│   SELECT TRADING PAIR               │
├─────────────────────────────────────┤
│  ┌─────────┐ ┌─────────┐ ┌─────────┐│
│  │ Solana  │ │Ethereum │ │ Bitcoin ││
│  │ SOL/USDC│ │ ETH/USDC│ │ BTC/USDC││
│  └─────────┘ └─────────┘ └─────────┘│
└─────────────────────────────────────┘
```

---

## 🎯 **Why These 3 Pairs?**

These are the **most liquid and popular** crypto pairs:
- ✅ **SOL/USDC** - Solana (native to this chain)
- ✅ **ETH/USDC** - Ethereum (most traded alt)
- ✅ **BTC/USDC** - Bitcoin (largest market cap)

---

## 📐 **Smart Defaults Per Pair**

Each pair has **optimized tick/lot sizes** based on typical trading patterns:

### **SOL/USDC**
```
Mark Price:  $186.00
Tick Size:   $1.00      (orders in $1 increments)
Lot Size:    1.0 SOL    (orders in whole SOL)
Min Order:   1.0 SOL

Example orders: $185, $186, $187 (not $185.50)
```

### **ETH/USDC**
```
Mark Price:  $2,800.00
Tick Size:   $10.00      (orders in $10 increments)
Lot Size:    0.1 ETH     (orders in 0.1 ETH)
Min Order:   0.1 ETH

Example orders: $2,800, $2,810, $2,820 (not $2,805)
```

### **BTC/USDC**
```
Mark Price:  $43,000.00
Tick Size:   $100.00     (orders in $100 increments)
Lot Size:    0.01 BTC    (orders in 0.01 BTC)
Min Order:   0.01 BTC

Example orders: $43,000, $43,100, $43,200 (not $43,050)
```

---

## 🔄 **What About USDC/BTC? (Inverse Pairs)**

### **Question:** "I want to trade USDC → BTC, not BTC → USDC!"

### **Answer:** Use **BTC/USDC** with the correct order side!

```
BTC/USDC Pair:
┌─────────────────────────────────────┐
│  Price: $43,000 per BTC             │
├─────────────────────────────────────┤
│  BUY Order:                         │
│    Pay:    USDC (e.g., $430)        │
│    Get:    0.01 BTC                 │
│    ➡️  USDC → BTC                   │
├─────────────────────────────────────┤
│  SELL Order:                        │
│    Pay:    0.01 BTC                 │
│    Get:    USDC (e.g., $430)        │
│    ➡️  BTC → USDC                   │
└─────────────────────────────────────┘
```

### **Real-World Example:**

**Scenario:** You have 10,000 USDC and want to buy Bitcoin.

```
Step 1: Go to dashboard, select BTC/USDC pair
Step 2: Click "BUY" button
Step 3: Enter:
  - Price: $43,000 (or current market)
  - Amount: 0.23 BTC (costs ~9,890 USDC)
Step 4: Submit order
Result: You traded USDC → BTC ✅
```

**Scenario:** You have 0.5 BTC and want to sell for USDC.

```
Step 1: Go to dashboard, select BTC/USDC pair
Step 2: Click "SELL" button
Step 3: Enter:
  - Price: $43,000
  - Amount: 0.5 BTC (gets ~21,500 USDC)
Step 4: Submit order
Result: You traded BTC → USDC ✅
```

---

## 🎯 **Key Concept: Base vs Quote**

In any trading pair `BASE/QUOTE`:

```
SOL/USDC:
  Base:  SOL
  Quote: USDC
  
  • BUY  = spend QUOTE (USDC) to get BASE (SOL)
  • SELL = spend BASE (SOL) to get QUOTE (USDC)
```

```
BTC/USDC:
  Base:  BTC
  Quote: USDC
  
  • BUY  = spend QUOTE (USDC) to get BASE (BTC)
  • SELL = spend BASE (BTC) to get QUOTE (USDC)
```

**So there's NO separate USDC/BTC pair needed!** Just use BTC/USDC and choose the right side.

---

## 📊 **Why Not Allow Custom Pairs?**

### **Reasons for restriction:**

1. **Liquidity Focus:** Concentrating liquidity in 3 pairs = better trading experience
2. **Oracle Support:** Only SOL, ETH, BTC have reliable price oracles
3. **Risk Management:** Unknown tokens can have extreme volatility
4. **Testing Phase:** Start with proven pairs, expand later

### **Future Expansion:**

```
Coming Soon:
  • USDT/USDC (stablecoin pair)
  • Major meme coins (BONK, PEPE)
  • DeFi tokens (JUP, ORCA)
  • Community governance to add pairs
```

---

## 💡 **User Flow Examples**

### **Example 1: Create SOL/USDC Slab**
```
1. Click "Create Slab" button
2. Select "SOL/USDC" (first button, blue highlight)
3. Fields auto-fill:
   - Mark Price: $186
   - Tick Size: $1.0
   - Lot Size: 1.0
4. Adjust mark price if needed (e.g., $190)
5. Click "Create Slab (~0.46 SOL)"
6. Sign transaction
7. ✅ Slab created!
```

### **Example 2: Provide Liquidity to BTC/USDC**
```
1. On Create Slab page, right panel
2. Enter slab address (or use newly created one)
3. Amount: 0.5 BTC
4. Price: $42,500 (below market = buy order)
5. Click "Provide Liquidity"
6. Sign transaction
7. ✅ Your 0.5 BTC is now in orderbook at $42,500
8. When someone sells BTC, you buy it!
9. You earn 0.1% fee on the trade
```

---

## 🎨 **Visual UI Changes**

### **Before (Old):**
```
Base Currency: [text input: "MEME"]
```
❌ Users could enter anything

### **After (New):**
```
Select Trading Pair:
[ SOL/USDC ] [ ETH/USDC ] [ BTC/USDC ]
     ✓            -            -
```
✅ Only 3 options, visual feedback

---

## 🚀 **Benefits of This Approach**

1. **Simplicity:** Users can't create incompatible pairs
2. **Quality:** All 3 pairs have real oracle data
3. **Liquidity:** Focus liquidity in fewer markets
4. **User-Friendly:** Big buttons, clear choices
5. **Scalable:** Easy to add more pairs later

---

## 🔑 **Technical Details**

### **Code Changes:**
```typescript
// Old: Free-form text input
const [baseCurrency, setBaseCurrency] = useState('');

// New: Type-restricted enum
const [baseCurrency, setBaseCurrency] = useState<'SOL' | 'ETH' | 'BTC'>('SOL');

// Available pairs with defaults
const availablePairs = [
  { base: 'SOL', suggestedPrice: '186', tickSize: '1.0', lotSize: '1.0' },
  { base: 'ETH', suggestedPrice: '2800', tickSize: '10.0', lotSize: '0.1' },
  { base: 'BTC', suggestedPrice: '43000', tickSize: '100.0', lotSize: '0.01' },
];
```

---

## 📝 **Summary**

✅ **Restricted to 3 pairs:** SOL, ETH, BTC (all vs USDC)  
✅ **Smart defaults** per pair  
✅ **No USDC/BTC needed** - just use BTC/USDC with correct side  
✅ **Better UX** with button selector instead of text input  
✅ **Future-proof** - easy to add more pairs later  

---

**Your slab creation is now streamlined and user-friendly! 🎉📈✨**

